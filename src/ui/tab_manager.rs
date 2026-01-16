//! Tab Manager - Manages multiple tab states for multi-tab log viewing
//!
//! Each tab maintains its own independent state including log buffer,
//! filters, search state, and view configuration.
//! Also supports split view for viewing two logs side by side.

use crate::bookmarks::BookmarksStore;
use crate::config::FileGrokConfig;
use crate::file_watcher::FileWatcher;
use crate::grok_parser::GrokParser;
use crate::log_buffer::{LogBuffer, LogBufferConfig};
use crate::log_entry::LogEntry;
use crate::log_reader::{LogReader, LogReaderConfig};
use crate::search::LogFilter;
use crate::ui::main_view::MainView;
use crate::ui::split_view::{SplitAction, SplitPane, SplitView};
use crate::ui::tab_bar::{Tab, TabBar, TabBarAction, TabId};

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Messages from background reader thread
#[derive(Debug)]
pub enum ReaderMessage {
    /// New log entries (appended to end)
    NewEntries(Vec<LogEntry>),
    /// Previous chunk loaded (prepended to beginning, for lazy loading)
    PreviousChunk(Vec<LogEntry>, u64), // entries, new_start_offset
    /// File was reset (rotation)
    FileReset,
    /// Error occurred
    Error(String),
}

/// Messages to background reader thread
#[derive(Debug)]
pub enum ReaderCommand {
    /// Stop reading
    Stop,
    /// Load previous chunk (for lazy loading when scrolling up)
    LoadPreviousChunk(u64, usize), // before_offset, max_lines
}

/// State for a single tab
pub struct TabState {
    /// File path
    pub path: PathBuf,
    /// Log buffer
    pub buffer: LogBuffer,
    /// Log reader (for sync file info)
    pub reader: Option<LogReader>,
    /// File watcher
    pub watcher: Option<FileWatcher>,
    /// Background reader message receiver
    pub reader_rx: Option<Receiver<ReaderMessage>>,
    /// Background reader command sender
    pub reader_tx: Option<Sender<ReaderCommand>>,
    /// Current file encoding
    pub encoding: Option<&'static encoding_rs::Encoding>,
    /// Search and filter engine
    pub filter: LogFilter,
    /// Filtered indices cache
    pub filtered_indices: Vec<usize>,
    /// Whether filter is active
    pub filter_active: bool,
    /// Main log view state
    pub main_view: MainView,
    /// Pending entries count (for batching)
    pub pending_entries: usize,
    /// Index of the next entry to check for grok parsing
    pub grok_parse_progress: usize,
    /// Per-tab grok parser (for file-specific patterns)
    pub grok_parser: Option<GrokParser>,
    /// Per-tab grok config
    pub grok_config: Option<FileGrokConfig>,
}

impl TabState {
    /// Create a new tab state
    pub fn new(id: TabId, path: PathBuf, buffer_config: LogBufferConfig) -> Self {
        // Create a unique ID for the main view based on tab ID
        let view_id = egui::Id::new(format!("main_view_tab_{}", id));

        Self {
            path,
            buffer: LogBuffer::with_config(buffer_config),
            reader: None,
            watcher: None,
            reader_rx: None,
            reader_tx: None,
            encoding: None,
            filter: LogFilter::new(),
            filtered_indices: Vec::new(),
            filter_active: false,
            main_view: MainView::with_id(view_id),
            pending_entries: 0,
            grok_parse_progress: 0,
            grok_parser: None,
            grok_config: None,
        }
    }

    /// Open the file and start reading
    pub fn open_file(
        &mut self,
        encoding: Option<&'static encoding_rs::Encoding>,
        bookmarks_store: &BookmarksStore,
    ) -> Result<()> {
        // Save encoding
        self.encoding = encoding;

        // Create reader with optional encoding
        let config = LogReaderConfig {
            encoding,
            ..Default::default()
        };
        let mut reader = LogReader::with_config(&self.path, config)?;

        // Read initial content using tail mode for better performance with large files
        let initial_lines = self.buffer.chunk_size() * 2; // Load ~10k lines initially
        let (entries, start_offset, total_lines) = reader.read_tail(initial_lines)?;
        
        // Initialize buffer with lazy load state
        self.buffer.init_with_tail(entries, start_offset, total_lines);

        // Restore bookmarks for this file
        if let Some(file_bookmarks) = bookmarks_store.get_bookmarks(&self.path) {
            let bookmarked_lines: Vec<usize> = file_bookmarks.lines.iter().copied().collect();
            let indices_to_bookmark: Vec<usize> = bookmarked_lines
                .into_iter()
                .filter_map(|line_number| {
                    self.buffer
                        .iter()
                        .position(|e| e.line_number == line_number)
                })
                .collect();

            for index in indices_to_bookmark {
                self.buffer.toggle_bookmark(index);
            }
        }

        // Update filter
        self.update_filter();

        // Create file watcher
        let watcher = FileWatcher::new(&self.path)?;

        // Start background reader thread
        let (msg_tx, msg_rx) = bounded::<ReaderMessage>(1000);
        let (cmd_tx, cmd_rx) = bounded::<ReaderCommand>(10);

        let reader_path = self.path.clone();
        let reader_offset = reader.offset();
        let reader_line_count = reader.line_count();
        let reader_encoding = encoding;

        thread::spawn(move || {
            Self::reader_thread(
                reader_path,
                reader_offset,
                reader_line_count,
                reader_encoding,
                msg_tx,
                cmd_rx,
            );
        });

        self.reader = Some(reader);
        self.watcher = Some(watcher);
        self.reader_rx = Some(msg_rx);
        self.reader_tx = Some(cmd_tx);

        Ok(())
    }

    /// Background reader thread function
    fn reader_thread(
        path: PathBuf,
        initial_offset: u64,
        initial_line_count: usize,
        encoding: Option<&'static encoding_rs::Encoding>,
        msg_tx: Sender<ReaderMessage>,
        cmd_rx: Receiver<ReaderCommand>,
    ) {
        let config = LogReaderConfig {
            encoding,
            ..Default::default()
        };
        let mut reader = match LogReader::with_config(&path, config) {
            Ok(r) => r,
            Err(e) => {
                let _ = msg_tx.send(ReaderMessage::Error(e.to_string()));
                return;
            }
        };

        reader.seek_with_line_count(initial_offset, initial_line_count);

        loop {
            // Check for commands first
            match cmd_rx.try_recv() {
                Ok(ReaderCommand::Stop) => break,
                Ok(ReaderCommand::LoadPreviousChunk(before_offset, max_lines)) => {
                    // Load previous chunk for lazy loading
                    match reader.read_previous_chunk(before_offset, max_lines) {
                        Ok((entries, new_start_offset)) if !entries.is_empty() => {
                            let _ = msg_tx.send(ReaderMessage::PreviousChunk(entries, new_start_offset));
                        }
                        Ok(_) => {
                            // No more data to load
                            let _ = msg_tx.send(ReaderMessage::PreviousChunk(Vec::new(), 0));
                        }
                        Err(e) => {
                            let _ = msg_tx.send(ReaderMessage::Error(e.to_string()));
                        }
                    }
                }
                Err(_) => {} // No command
            }

            match reader.has_new_content() {
                Ok(true) => {
                    match std::fs::metadata(&path) {
                        Ok(metadata) => {
                            let current_size = metadata.len();
                            if current_size < reader.offset() {
                                let _ = msg_tx.send(ReaderMessage::FileReset);
                                reader.seek_with_line_count(0, 0);
                            }
                        }
                        Err(e) => {
                            let _ = msg_tx.send(ReaderMessage::Error(e.to_string()));
                        }
                    }

                    match reader.read_new_lines() {
                        Ok(entries) if !entries.is_empty() => {
                            let _ = msg_tx.send(ReaderMessage::NewEntries(entries));
                        }
                        Err(e) => {
                            let _ = msg_tx.send(ReaderMessage::Error(e.to_string()));
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    let _ = msg_tx.send(ReaderMessage::Error(e.to_string()));
                    thread::sleep(Duration::from_secs(1));
                }
                _ => {}
            }

            thread::sleep(Duration::from_millis(50));
        }
    }

    /// Process messages from background reader
    pub fn process_reader_messages(&mut self) -> bool {
        let Some(rx) = &self.reader_rx else {
            return false;
        };

        let mut new_entries = Vec::new();
        let mut prepend_entries: Option<(Vec<LogEntry>, u64)> = None;
        let mut had_changes = false;
        let mut had_reset = false;

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ReaderMessage::NewEntries(entries) => {
                    new_entries.extend(entries);
                    had_changes = true;
                }
                ReaderMessage::PreviousChunk(entries, new_start_offset) => {
                    // Handle lazy-loaded previous chunk
                    self.buffer.lazy_load.loading_in_progress = false;
                    if entries.is_empty() {
                        // No more data, mark as fully loaded
                        self.buffer.lazy_load.fully_loaded = true;
                    } else {
                        prepend_entries = Some((entries, new_start_offset));
                        had_changes = true;
                    }
                }
                ReaderMessage::FileReset => {
                    had_reset = true;
                    self.buffer.clear();
                }
                ReaderMessage::Error(e) => {
                    tracing::error!("Reader error for {:?}: {}", self.path, e);
                    self.buffer.lazy_load.loading_in_progress = false;
                }
            }
        }

        // Handle prepended entries first (from lazy loading)
        if let Some((entries, new_start_offset)) = prepend_entries {
            let prepend_count = entries.len();
            self.buffer.prepend(entries);
            self.buffer.lazy_load.loaded_start_offset = new_start_offset;
            self.buffer.lazy_load.first_loaded_line = self.buffer.first_line_number();
            self.buffer.lazy_load.load_more_requested = false;
            
            // Adjust grok_parse_progress since we prepended items
            // The existing parsed items are now at higher indices
            self.grok_parse_progress += prepend_count;
            
            self.filter.mark_dirty();
            self.pending_entries += 1;
        }

        if !new_entries.is_empty() {
            let old_first_line = self.buffer.first_line_number();
            self.buffer.extend(new_entries);
            let new_first_line = self.buffer.first_line_number();

            // Adjust grok_parse_progress if items were trimmed from the front
            if new_first_line > old_first_line {
                let dropped = new_first_line - old_first_line;
                self.grok_parse_progress = self.grok_parse_progress.saturating_sub(dropped);
            }

            self.filter.mark_dirty();
            self.pending_entries += 1;
        }

        had_changes || had_reset
    }

    /// Request to load more data (for lazy loading when scrolling up)
    pub fn request_load_more(&mut self) {
        if !self.buffer.lazy_load.enabled 
            || self.buffer.lazy_load.fully_loaded 
            || self.buffer.lazy_load.loading_in_progress 
        {
            return;
        }
        
        if let Some(tx) = &self.reader_tx {
            let before_offset = self.buffer.lazy_load.loaded_start_offset;
            let chunk_size = self.buffer.chunk_size();
            
            if tx.try_send(ReaderCommand::LoadPreviousChunk(before_offset, chunk_size)).is_ok() {
                self.buffer.lazy_load.loading_in_progress = true;
                self.buffer.lazy_load.load_more_requested = false;
            }
        }
    }

    /// Update filtered indices
    pub fn update_filter(&mut self) {
        let indices = self.filter.apply(&self.buffer);
        self.filtered_indices = indices.to_vec();
        self.filter_active = self.filter.is_filtering();
    }

    /// Clear the buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.filtered_indices.clear();
        self.filter.mark_dirty();
        self.main_view.clear_selection();
        self.grok_parse_progress = 0;
    }

    /// Close the tab (stop reader, watcher)
    pub fn close(&mut self) {
        if let Some(tx) = self.reader_tx.take() {
            let _ = tx.send(ReaderCommand::Stop);
        }
        self.reader_rx = None;

        if let Some(watcher) = self.watcher.take() {
            watcher.stop();
        }

        self.reader = None;
    }

    /// Reload the file
    pub fn reload(&mut self, bookmarks_store: &BookmarksStore) -> Result<()> {
        let encoding = self.encoding;
        self.close();
        self.buffer.clear();
        self.filtered_indices.clear();
        self.main_view.clear_selection();
        self.grok_parse_progress = 0;
        self.open_file(encoding, bookmarks_store)
    }

    /// Check if watcher is active
    pub fn is_watching(&self) -> bool {
        self.watcher.is_some()
    }

    /// Stop monitoring (stop reading new logs but keep watcher)
    pub fn stop_monitoring(&mut self) {
        if let Some(tx) = &self.reader_tx {
            let _ = tx.send(ReaderCommand::Stop);
        }
    }

    /// Resume monitoring (restart reader thread)
    pub fn resume_monitoring(&mut self) {
        if self.watcher.is_none() {
            return;
        }

        // Restart background reader thread
        let (msg_tx, msg_rx) = bounded::<ReaderMessage>(1000);
        let (cmd_tx, cmd_rx) = bounded::<ReaderCommand>(10);

        let reader_path = self.path.clone();
        let reader_offset = self.reader.as_ref().map(|r| r.offset()).unwrap_or(0);
        let reader_line_count = self.reader.as_ref().map(|r| r.line_count()).unwrap_or(0);
        let reader_encoding = self.encoding;

        thread::spawn(move || {
            Self::reader_thread(
                reader_path,
                reader_offset,
                reader_line_count,
                reader_encoding,
                msg_tx,
                cmd_rx,
            );
        });

        self.reader_rx = Some(msg_rx);
        self.reader_tx = Some(cmd_tx);
    }

    /// Set theme for main view
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.main_view.set_dark_theme(dark);
    }
}

/// Manager for multiple tab states
pub struct TabManager {
    /// Tab bar UI component
    pub tab_bar: TabBar,
    /// State for each open tab
    pub states: HashMap<TabId, TabState>,
    /// Split view manager
    pub split_view: SplitView,
    /// Buffer configuration for new tabs
    buffer_config: LogBufferConfig,
    /// Dark theme setting
    dark_theme: bool,
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new(LogBufferConfig::default())
    }
}

impl TabManager {
    /// Create a new tab manager
    pub fn new(buffer_config: LogBufferConfig) -> Self {
        Self {
            tab_bar: TabBar::new(),
            states: HashMap::new(),
            split_view: SplitView::new(),
            buffer_config,
            dark_theme: true,
        }
    }

    /// Set theme
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.dark_theme = dark;
        self.tab_bar.set_dark_theme(dark);
        self.split_view.set_dark_theme(dark);
        for state in self.states.values_mut() {
            state.set_dark_theme(dark);
        }
    }

    /// Open a local file in a new tab
    pub fn open_local_file(
        &mut self,
        path: PathBuf,
        encoding: Option<&'static encoding_rs::Encoding>,
        bookmarks_store: &BookmarksStore,
    ) -> Result<TabId> {
        // Check if already open
        if let Some(id) = self.tab_bar.find_by_path(&path) {
            self.tab_bar.active_tab = Some(id);
            return Ok(id);
        }

        // Create new tab
        let tab = Tab::new_local(0, path.clone());
        let id = self.tab_bar.add_tab(tab);

        // Create tab state
        let mut state = TabState::new(id, path, self.buffer_config.clone());
        state.set_dark_theme(self.dark_theme);
        state.open_file(encoding, bookmarks_store)?;

        self.states.insert(id, state);

        Ok(id)
    }

    /// Open a remote stream in a new tab
    pub fn open_remote_stream(
        &mut self,
        project_name: String,
        cache_path: PathBuf,
        bookmarks_store: &BookmarksStore,
    ) -> Result<TabId> {
        // Check if already open
        if let Some(id) = self.tab_bar.find_by_path(&cache_path) {
            self.tab_bar.active_tab = Some(id);
            return Ok(id);
        }

        // Create new tab
        let tab = Tab::new_remote(0, project_name, cache_path.clone());
        let id = self.tab_bar.add_tab(tab);

        // Create tab state
        let mut state = TabState::new(id, cache_path, self.buffer_config.clone());
        state.set_dark_theme(self.dark_theme);
        state.open_file(None, bookmarks_store)?;

        self.states.insert(id, state);

        Ok(id)
    }

    /// Close a tab
    pub fn close_tab(&mut self, id: TabId, bookmarks_store: &mut BookmarksStore) -> Option<Tab> {
        // Save bookmarks before closing
        if self.states.contains_key(&id) {
            self.save_bookmarks(id, bookmarks_store);
        }

        // Close the state
        if let Some(mut state) = self.states.remove(&id) {
            state.close();
        }

        // Remove from tab bar
        self.tab_bar.close_tab(id)
    }

    /// Save bookmarks for a tab
    pub fn save_bookmarks(&self, id: TabId, bookmarks_store: &mut BookmarksStore) {
        if let Some(state) = self.states.get(&id) {
            use std::collections::HashSet;

            let bookmarked_lines: HashSet<usize> = state
                .buffer
                .iter()
                .filter(|e| e.bookmarked)
                .map(|e| e.line_number)
                .collect();

            bookmarks_store.set_bookmarks(&state.path, bookmarked_lines);
            if let Err(e) = bookmarks_store.save() {
                tracing::error!("Failed to save bookmarks: {}", e);
            }
        }
    }

    /// Get the active tab state
    pub fn get_active_state(&self) -> Option<&TabState> {
        self.tab_bar.active_tab.and_then(|id| self.states.get(&id))
    }

    /// Get the active tab state mutably
    pub fn get_active_state_mut(&mut self) -> Option<&mut TabState> {
        self.tab_bar
            .active_tab
            .and_then(|id| self.states.get_mut(&id))
    }

    /// Get a mutable tab state by ID
    pub fn get_state_mut(&mut self, id: TabId) -> Option<&mut TabState> {
        self.states.get_mut(&id)
    }

    /// Get a mutable tab state by file path
    pub fn get_state_by_path_mut(&mut self, path: &PathBuf) -> Option<&mut TabState> {
        self.states.values_mut().find(|s| &s.path == path)
    }

    /// Process reader messages for all tabs
    pub fn process_all_reader_messages(&mut self) {
        for state in self.states.values_mut() {
            state.process_reader_messages();
        }
    }

    /// Update filters for all tabs with pending entries
    pub fn update_pending_filters(&mut self) {
        for state in self.states.values_mut() {
            if state.pending_entries > 0 {
                state.update_filter();
                state.pending_entries = 0;
            }
        }
    }

    /// Check if any tab is actively watching
    pub fn any_watching(&self) -> bool {
        self.states.values().any(|s| s.is_watching())
    }

    /// Check if there are no tabs
    pub fn is_empty(&self) -> bool {
        self.tab_bar.is_empty()
    }

    /// Handle tab bar action
    pub fn handle_action(
        &mut self,
        action: TabBarAction,
        bookmarks_store: &mut BookmarksStore,
    ) -> TabBarAction {
        match action {
            TabBarAction::SelectTab(id) => {
                self.tab_bar.active_tab = Some(id);
            }
            TabBarAction::CloseTab(id) => {
                self.close_tab(id, bookmarks_store);
            }
            TabBarAction::CloseOtherTabs(keep_id) => {
                let closed = self.tab_bar.close_other_tabs(keep_id);
                for tab in closed {
                    if let Some(mut state) = self.states.remove(&tab.id) {
                        self.save_bookmarks(tab.id, bookmarks_store);
                        state.close();
                    }
                }
            }
            TabBarAction::CloseTabsToRight(id) => {
                let closed = self.tab_bar.close_tabs_to_right(id);
                for tab in closed {
                    if let Some(mut state) = self.states.remove(&tab.id) {
                        self.save_bookmarks(tab.id, bookmarks_store);
                        state.close();
                    }
                }
            }
            TabBarAction::CloseAllTabs => {
                // Save all bookmarks first
                for (id, _) in self.states.iter() {
                    self.save_bookmarks(*id, bookmarks_store);
                }

                // Close all states
                for (_, mut state) in self.states.drain() {
                    state.close();
                }

                self.tab_bar.close_all_tabs();
                self.split_view.disable_split();
            }
            TabBarAction::ReorderTabs(from, to) => {
                self.tab_bar.reorder(from, to);
            }
            TabBarAction::OpenInSplit(id) => {
                // Enable split view with this tab in the right pane
                self.enable_split(id);
            }
            TabBarAction::None => {}
        }
        action
    }

    /// Reload the active tab
    pub fn reload_active(&mut self, bookmarks_store: &BookmarksStore) -> Result<()> {
        if let Some(state) = self.get_active_state_mut() {
            state.reload(bookmarks_store)?;
        }
        Ok(())
    }

    /// Clear the active tab buffer
    pub fn clear_active_buffer(&mut self) {
        if let Some(state) = self.get_active_state_mut() {
            state.clear_buffer();
        }
    }

    // === Split View Methods ===

    /// Check if split view is active
    pub fn is_split(&self) -> bool {
        self.split_view.is_split()
    }

    /// Enable split view with a tab in the right pane
    pub fn enable_split(&mut self, right_tab_id: TabId) {
        // Set the current active tab to left pane
        if let Some(active) = self.tab_bar.active_tab {
            if active != right_tab_id {
                self.split_view.set_pane_tab(SplitPane::Left, Some(active));
            } else {
                // If the right tab is the same as active, pick another tab for left
                if let Some(other) = self.tab_bar.tabs.iter().find(|t| t.id != right_tab_id) {
                    self.split_view
                        .set_pane_tab(SplitPane::Left, Some(other.id));
                }
            }
        }
        self.split_view.enable_split(right_tab_id);
    }

    /// Disable split view
    pub fn disable_split(&mut self) {
        // Move the active tab back to the main view
        if let Some(left_tab) = self.split_view.get_pane_tab(SplitPane::Left) {
            self.tab_bar.active_tab = Some(left_tab);
        } else if let Some(right_tab) = self.split_view.get_pane_tab(SplitPane::Right) {
            self.tab_bar.active_tab = Some(right_tab);
        }
        self.split_view.disable_split();
    }

    /// Sync the split view tab selection with the tab bar
    pub fn sync_split_with_tab_bar(&mut self) {
        if self.split_view.is_split() {
            // Make sure both pane tabs still exist
            let left = self.split_view.get_pane_tab(SplitPane::Left);
            let right = self.split_view.get_pane_tab(SplitPane::Right);

            let left_exists = left
                .map(|id| self.states.contains_key(&id))
                .unwrap_or(false);
            let right_exists = right
                .map(|id| self.states.contains_key(&id))
                .unwrap_or(false);

            if !left_exists && !right_exists {
                self.split_view.disable_split();
            } else if !right_exists {
                self.split_view.disable_split();
            } else if !left_exists {
                // Move right to left and disable split
                if let Some(right_id) = right {
                    self.split_view
                        .set_pane_tab(SplitPane::Left, Some(right_id));
                }
                self.split_view.disable_split();
            }
        } else {
            // Single pane mode - sync with tab bar's active tab
            if let Some(active) = self.tab_bar.active_tab {
                self.split_view.set_pane_tab(SplitPane::Left, Some(active));
            }
        }
    }

    /// Handle split action
    pub fn handle_split_action(&mut self, action: SplitAction) {
        match action {
            SplitAction::SplitRatioChanged(_) => {
                // Ratio is already updated in split_view
            }
            SplitAction::None => {}
        }
    }

    /// Toggle split view
    pub fn toggle_split(&mut self) {
        if self.split_view.is_split() {
            self.disable_split();
        } else if self.tab_bar.len() >= 2 {
            // Find a second tab to show in split
            if let Some(active) = self.tab_bar.active_tab {
                if let Some(other) = self.tab_bar.tabs.iter().find(|t| t.id != active) {
                    self.enable_split(other.id);
                }
            }
        }
    }
}
