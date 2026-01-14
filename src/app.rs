//! Main application logic

use crate::bookmarks::BookmarksStore;
use crate::config::{AppConfig, DisplayConfig, Shortcuts, Theme};
use crate::file_watcher::FileWatcher;
use crate::i18n::set_language;
use crate::log_buffer::{LogBuffer, LogBufferConfig};
use crate::log_entry::LogEntry;
use crate::log_reader::{LogReader, LogReaderConfig};
use crate::remote_server::{RemoteServer, ServerConfig, ServerEvent};
use crate::search::LogFilter;
use crate::tray::{TrayEvent, TrayManager};
use crate::ui::activity_bar::{ActivityBar, ActivityBarAction, ActivityView};
use crate::ui::advanced_filters_panel::AdvancedFiltersPanel;
use crate::ui::bookmarks_panel::{BookmarkAction, BookmarksPanel};
use crate::ui::explorer_panel::{ExplorerAction, ExplorerPanel, OpenEditor};
use crate::ui::file_picker_dialog::{FilePickerAction, FilePickerDialog};
use crate::ui::filter_panel::FilterPanel;
use crate::ui::global_search_panel::{GlobalSearchAction, GlobalSearchPanel};
use crate::ui::main_view::{ContextMenuAction, MainView};
use crate::ui::search_bar::{SearchBar, SearchBarAction};
use crate::ui::settings_panel::{SettingsAction, SettingsPanel};
use crate::ui::status_bar::{StatusBar, StatusLevel};
use crate::ui::toolbar::{Toolbar, ToolbarAction, ToolbarState};

use crate::mcp::{McpConfig, McpServer};

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use eframe::egui;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// Messages from background reader thread
#[derive(Debug)]
enum ReaderMessage {
    /// New log entries
    NewEntries(Vec<LogEntry>),
    /// File was reset (rotation)
    FileReset,
    /// Error occurred
    Error(String),
    /// Reading complete
    #[allow(dead_code)]
    Complete,
}

/// Messages to background reader thread
#[derive(Debug)]
enum ReaderCommand {
    /// Stop reading
    Stop,
}

/// Main application state
pub struct LoglineApp {
    /// Application configuration
    config: AppConfig,
    /// Display configuration
    display_config: DisplayConfig,
    /// Keyboard shortcuts
    shortcuts: Shortcuts,

    /// Log buffer
    buffer: LogBuffer,
    /// Current file path
    current_file: Option<PathBuf>,
    /// Log reader (for sync file info)
    reader: Option<LogReader>,
    /// File watcher
    watcher: Option<FileWatcher>,

    /// Background reader message receiver
    reader_rx: Option<Receiver<ReaderMessage>>,
    /// Background reader command sender
    reader_tx: Option<Sender<ReaderCommand>>,
    /// Current file encoding
    current_encoding: Option<&'static encoding_rs::Encoding>,

    /// Search and filter engine
    filter: LogFilter,
    /// Filtered indices cache
    filtered_indices: Vec<usize>,
    /// Whether filter is active
    filter_active: bool,

    /// Main log view
    main_view: MainView,
    /// Search bar
    search_bar: SearchBar,
    /// Filter panel
    filter_panel: FilterPanel,
    /// Status bar
    status_bar: StatusBar,
    /// Toolbar state
    toolbar_state: ToolbarState,

    /// Go-to-line dialog state
    goto_dialog: GotoLineDialog,
    /// File picker dialog
    file_picker_dialog: FilePickerDialog,

    /// Last update time for rate limiting
    last_update: Instant,
    /// Pending entries count (for batching)
    pending_entries: usize,

    // === New: Remote server and sidebar ===
    /// Remote log server
    remote_server: RemoteServer,
    /// Activity bar (left-side icons)
    activity_bar: ActivityBar,
    /// Explorer panel (file/stream browser)
    explorer_panel: ExplorerPanel,
    /// Advanced filters panel
    advanced_filters_panel: AdvancedFiltersPanel,
    /// Bookmarks panel
    bookmarks_panel: BookmarksPanel,
    /// Settings panel
    settings_panel: SettingsPanel,
    /// Global search panel
    global_search_panel: GlobalSearchPanel,
    /// Sidebar visibility
    sidebar_visible: bool,

    // === MCP Server ===
    /// MCP server for AI log analysis
    mcp_server: Option<McpServer>,
    /// Tokio runtime for async MCP operations
    tokio_runtime: Option<tokio::runtime::Runtime>,

    /// Whether this is the first frame (for initial theme application)
    first_frame: bool,

    // === System Tray ===
    /// System tray manager
    tray_manager: Option<TrayManager>,
    /// Whether the app should quit completely
    should_quit: bool,
    /// Whether tray has been initialized
    tray_initialized: bool,

    // === Bookmarks Persistence ===
    /// Bookmarks storage
    bookmarks_store: BookmarksStore,
}

impl LoglineApp {
    /// Create a new application instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load configuration
        let config = AppConfig::load().unwrap_or_default();

        // Apply language from config
        set_language(config.language);

        // Setup fonts with Chinese support
        Self::setup_fonts(&cc.egui_ctx);

        // Apply theme
        let visuals = match config.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        cc.egui_ctx.set_visuals(visuals);

        // Create remote server with config
        let server_config = ServerConfig {
            port: config.remote_server.port,
            ..Default::default()
        };
        let mut remote_server = RemoteServer::new(server_config);

        // Auto-start remote server if configured
        if config.remote_server.auto_start {
            if let Err(e) = remote_server.start() {
                tracing::error!("Failed to auto-start remote server: {}", e);
            }
        }

        // Create settings panel with language and display config from config
        let mut settings_panel = SettingsPanel::new();
        settings_panel.language = config.language;
        settings_panel.display_config = config.display.clone();
        settings_panel.dark_theme = config.theme == Theme::Dark;
        settings_panel.mcp_enabled = config.mcp.enabled;
        settings_panel.mcp_port = config.mcp.port.to_string();
        settings_panel.server_port = config.remote_server.port.to_string();
        settings_panel.auto_start_server = config.remote_server.auto_start;

        // Initialize MCP server if enabled
        let (mcp_server, tokio_runtime) = {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .ok();

            let mcp_server = if config.mcp.enabled {
                let cache_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("logline")
                    .join("cache");

                let mcp_config = McpConfig {
                    port: config.mcp.port,
                    bind_address: config.mcp.bind_address.clone(),
                };

                let mut server = McpServer::new(mcp_config, cache_dir);

                // Start the server
                if let Some(ref rt) = runtime {
                    rt.block_on(async {
                        if let Err(e) = server.start().await {
                            tracing::error!("Failed to start MCP server: {}", e);
                        } else {
                            tracing::info!("MCP server started at {}", server.endpoint_url());
                        }
                    });
                }

                Some(server)
            } else {
                None
            };

            (mcp_server, runtime)
        };

        Self {
            display_config: config.display.clone(),
            shortcuts: Shortcuts::default(),
            buffer: LogBuffer::with_config(LogBufferConfig {
                max_lines: config.buffer.max_lines,
                auto_trim: config.buffer.auto_trim,
            }),
            current_file: None,
            reader: None,
            watcher: None,
            reader_rx: None,
            reader_tx: None,
            current_encoding: None,
            filter: LogFilter::new(),
            filtered_indices: Vec::new(),
            filter_active: false,
            main_view: {
                let mut view = MainView::new();
                // Apply theme from config at initialization
                view.set_dark_theme(config.theme == Theme::Dark);
                view
            },
            search_bar: SearchBar::new(),
            filter_panel: FilterPanel::new(),
            status_bar: StatusBar::new(),
            toolbar_state: ToolbarState {
                auto_scroll: true,
                search_visible: false,
                dark_theme: config.theme == Theme::Dark,
                reverse_order: false,
            },
            goto_dialog: GotoLineDialog::default(),
            file_picker_dialog: FilePickerDialog::new(),
            last_update: Instant::now(),
            pending_entries: 0,
            // New components
            remote_server,
            activity_bar: ActivityBar::new(),
            explorer_panel: {
                let mut panel = ExplorerPanel::new();
                // Load recent files from config
                panel.local_files = config.recent_files.clone();
                panel
            },
            advanced_filters_panel: AdvancedFiltersPanel::new(),
            bookmarks_panel: BookmarksPanel::new(),
            settings_panel,
            global_search_panel: {
                let mut panel = GlobalSearchPanel::new();
                panel.set_dark_theme(config.theme == Theme::Dark);
                panel
            },
            config,
            sidebar_visible: true,
            // MCP server
            mcp_server,
            tokio_runtime,
            // First frame flag for initial theme application
            first_frame: true,
            // System tray - will be initialized after event loop starts
            tray_manager: None,
            should_quit: false,
            tray_initialized: false,
            // Load bookmarks from disk
            bookmarks_store: BookmarksStore::load().unwrap_or_default(),
        }
    }

    /// Setup fonts with Chinese character support
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        // Load JetBrains Mono (English monospace font)
        fonts.font_data.insert(
            "jetbrains_mono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/JetBrainsMono-Regular.ttf"))
                .into(),
        );

        // Load Noto Sans SC (Chinese font)
        fonts.font_data.insert(
            "noto_sans_sc".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/NotoSansSC[wght].ttf")).into(),
        );

        // Configure font families with fallback order:
        // JetBrains Mono for ASCII/English, then Noto Sans SC for Chinese
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "jetbrains_mono".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("noto_sans_sc".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "jetbrains_mono".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("noto_sans_sc".to_owned());

        ctx.set_fonts(fonts);
    }

    /// Open a log file
    pub fn open_file(
        &mut self,
        path: PathBuf,
        encoding: Option<&'static encoding_rs::Encoding>,
    ) -> Result<()> {
        // Stop any existing reader (but don't save bookmarks here,
        // as they should already be saved before clearing the buffer)
        self.close_file_without_saving();

        // If no encoding specified, try to get saved encoding for this file
        let final_encoding = encoding.or_else(|| self.config.get_file_encoding(&path));

        // Save encoding
        self.current_encoding = final_encoding;

        // Create reader with optional encoding
        let config = LogReaderConfig {
            encoding: final_encoding,
            ..Default::default()
        };
        let mut reader = LogReader::with_config(&path, config)?;

        // Read initial content
        let entries = reader.read_all()?;
        self.buffer.extend(entries);

        // Restore bookmarks for this file
        if let Some(file_bookmarks) = self.bookmarks_store.get_bookmarks(&path) {
            let bookmarked_lines: Vec<usize> = file_bookmarks.lines.iter().copied().collect();
            // Collect all indices first to avoid borrow conflicts
            let indices_to_bookmark: Vec<usize> = bookmarked_lines
                .into_iter()
                .filter_map(|line_number| {
                    self.buffer
                        .iter()
                        .position(|e| e.line_number == line_number)
                })
                .collect();

            // Then toggle bookmarks
            for index in indices_to_bookmark {
                self.buffer.toggle_bookmark(index);
            }
        }

        // Update filter
        self.update_filter();

        // Create file watcher
        let watcher = FileWatcher::new(&path)?;

        // Start background reader thread
        let (msg_tx, msg_rx) = bounded::<ReaderMessage>(1000);
        let (cmd_tx, cmd_rx) = bounded::<ReaderCommand>(10);

        let reader_path = path.clone();
        let reader_offset = reader.offset();
        let reader_line_count = reader.line_count();
        let reader_encoding = final_encoding;

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

        // Store state
        self.current_file = Some(path.clone());
        self.reader = Some(reader);
        self.watcher = Some(watcher);
        self.reader_rx = Some(msg_rx);
        self.reader_tx = Some(cmd_tx);

        // Update recent files (only for local files, not cache files)
        let is_cache_file = if let Some(data_dir) = dirs::data_dir() {
            let cache_dir = data_dir.join("logline").join("cache");
            path.starts_with(&cache_dir)
        } else {
            false
        };

        if !is_cache_file {
            self.config.add_recent_file(path.clone());
            let _ = self.config.save();
            // Also update explorer panel's local files list
            self.explorer_panel.local_files = self.config.recent_files.clone();
        }

        // Sync to MCP server
        if let Some(ref mcp_server) = self.mcp_server {
            mcp_server.add_local_file(path);
        }

        self.status_bar
            .set_message("File opened", StatusLevel::Success);

        Ok(())
    }

    /// Close the current file
    pub fn close_file(&mut self) {
        // Save bookmarks before closing
        let current_path = self.current_file.clone();
        if let Some(path) = current_path {
            self.save_current_bookmarks(&path);
        }

        self.close_file_without_saving();
    }

    /// Close file without saving bookmarks (used internally when switching files)
    fn close_file_without_saving(&mut self) {
        self.current_encoding = None;
        // Stop reader thread
        if let Some(tx) = self.reader_tx.take() {
            let _ = tx.send(ReaderCommand::Stop);
        }
        self.reader_rx = None;

        // Stop watcher
        if let Some(watcher) = self.watcher.take() {
            watcher.stop();
        }

        self.reader = None;
        self.current_file = None;
    }

    /// Save bookmarks for the current file
    fn save_current_bookmarks(&mut self, path: &PathBuf) {
        use std::collections::HashSet;

        // Collect all bookmarked line numbers
        let bookmarked_lines: HashSet<usize> = self
            .buffer
            .iter()
            .filter(|e| e.bookmarked)
            .map(|e| e.line_number)
            .collect();

        // Update bookmarks store
        self.bookmarks_store.set_bookmarks(path, bookmarked_lines);

        // Save to disk
        if let Err(e) = self.bookmarks_store.save() {
            tracing::error!("Failed to save bookmarks: {}", e);
        }
    }

    /// Start the MCP server
    fn start_mcp_server(&mut self) {
        if self.mcp_server.is_some() {
            return; // Already running
        }

        let cache_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("logline")
            .join("cache");

        let mcp_config = McpConfig {
            port: self.config.mcp.port,
            bind_address: self.config.mcp.bind_address.clone(),
        };

        let mut server = McpServer::new(mcp_config, cache_dir);

        // Ensure we have a runtime
        if self.tokio_runtime.is_none() {
            self.tokio_runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .ok();
        }

        if let Some(ref rt) = self.tokio_runtime {
            let result = rt.block_on(async { server.start().await });

            match result {
                Ok(()) => {
                    let endpoint = server.endpoint_url();
                    self.status_bar
                        .set_message(format!("MCP服务已启动: {}", endpoint), StatusLevel::Success);
                    tracing::info!("MCP server started at {}", endpoint);
                    self.mcp_server = Some(server);
                }
                Err(e) => {
                    self.status_bar
                        .set_message(format!("MCP服务启动失败: {}", e), StatusLevel::Error);
                    tracing::error!("Failed to start MCP server: {}", e);
                }
            }
        }
    }

    /// Stop the MCP server
    fn stop_mcp_server(&mut self) {
        if let Some(mut server) = self.mcp_server.take() {
            if let Some(ref rt) = self.tokio_runtime {
                rt.block_on(async {
                    server.stop();
                });
            }
            self.status_bar
                .set_message("MCP服务已停止", StatusLevel::Info);
            tracing::info!("MCP server stopped");
        }
    }

    /// Reload the current file from the beginning
    pub fn reload_file(&mut self) {
        if let Some(path) = self.current_file.clone() {
            let encoding = self.current_encoding; // Preserve encoding

            // Close and clear
            self.close_file();
            self.buffer.clear();
            self.filtered_indices.clear();
            self.main_view.clear_selection();

            // Reopen the file with the same encoding
            match self.open_file(path, encoding) {
                Ok(()) => {
                    self.status_bar
                        .set_message("File reloaded", StatusLevel::Success);
                }
                Err(e) => {
                    self.status_bar
                        .set_message(format!("Reload failed: {}", e), StatusLevel::Error);
                }
            }
        } else {
            self.status_bar
                .set_message("No file to reload", StatusLevel::Warning);
        }
    }

    /// Change the encoding of the current file
    pub fn change_encoding(&mut self, encoding: Option<&'static encoding_rs::Encoding>) {
        if let Some(path) = self.current_file.clone() {
            // Save encoding preference
            self.config.set_file_encoding(path.clone(), encoding);
            let _ = self.config.save();

            // Close and clear
            self.close_file();
            self.buffer.clear();
            self.filtered_indices.clear();
            self.main_view.clear_selection();

            // Reopen with new encoding
            match self.open_file(path, encoding) {
                Ok(()) => {
                    let encoding_name = encoding.map(|e| e.name()).unwrap_or("Auto");
                    self.status_bar.set_message(
                        format!("Encoding changed to: {}", encoding_name),
                        StatusLevel::Success,
                    );
                }
                Err(e) => {
                    self.status_bar.set_message(
                        format!("Failed to change encoding: {}", e),
                        StatusLevel::Error,
                    );
                }
            }
        }
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

        // Seek to where we left off with correct line count
        reader.seek_with_line_count(initial_offset, initial_line_count);

        loop {
            // Check for stop command
            if let Ok(ReaderCommand::Stop) = cmd_rx.try_recv() {
                break;
            }

            // Check for new content
            match reader.has_new_content() {
                Ok(true) => {
                    // Check if file was truncated (rotation) by comparing with actual file size
                    match std::fs::metadata(&path) {
                        Ok(metadata) => {
                            let current_size = metadata.len();
                            if current_size < reader.offset() {
                                // File was truncated - this is a rotation
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

            // Poll interval
            thread::sleep(Duration::from_millis(50));
        }
    }

    /// Process messages from background reader
    fn process_reader_messages(&mut self) {
        let Some(rx) = &self.reader_rx else { return };

        let mut new_entries = Vec::new();
        let mut _had_reset = false;

        // Drain all available messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ReaderMessage::NewEntries(entries) => {
                    new_entries.extend(entries);
                }
                ReaderMessage::FileReset => {
                    _had_reset = true;
                    self.buffer.clear();
                    self.status_bar
                        .set_message("File rotated, reloading...", StatusLevel::Warning);
                }
                ReaderMessage::Error(e) => {
                    self.status_bar
                        .set_message(format!("Error: {}", e), StatusLevel::Error);
                }
                ReaderMessage::Complete => {}
            }
        }

        // Add new entries to buffer
        if !new_entries.is_empty() {
            self.buffer.extend(new_entries);
            self.filter.mark_dirty();
            self.pending_entries += 1;

            // Note: When auto-scroll is enabled, stick_to_bottom in ScrollArea
            // will automatically keep us at the bottom. No need to manually
            // scroll, which prevents flickering.
        }
    }

    /// Process events from remote server
    fn process_server_events(&mut self) {
        let rx = self.remote_server.event_receiver();
        let mut has_stream_changes = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                ServerEvent::AgentConnected {
                    project_name,
                    stream_id,
                    remote_addr,
                    cache_path: _,
                } => {
                    tracing::info!(
                        "Agent '{}' ({}) connected from {}",
                        project_name,
                        stream_id,
                        remote_addr
                    );
                    self.status_bar.set_message(
                        format!("Agent '{}' 已连接 ({})", project_name, remote_addr),
                        StatusLevel::Success,
                    );

                    // Don't automatically open the remote stream, just add it to the explorer
                    // User can manually click to open it
                    has_stream_changes = true;
                }
                ServerEvent::AgentDisconnected {
                    project_name,
                    stream_id,
                } => {
                    tracing::info!("Agent '{}' ({}) disconnected", project_name, stream_id);
                    self.status_bar.set_message(
                        format!("Agent '{}' 已断开", stream_id),
                        StatusLevel::Warning,
                    );
                    has_stream_changes = true;
                }
                ServerEvent::LogDataReceived {
                    project_name: _,
                    bytes: _,
                } => {
                    // File watcher will handle the UI refresh when cache file changes
                    // No need to update stream list here
                }
                ServerEvent::Error(e) => {
                    tracing::error!("Server error: {}", e);
                    self.status_bar
                        .set_message(format!("服务器错误: {}", e), StatusLevel::Error);
                }
                ServerEvent::Started { port } => {
                    tracing::info!("Remote server started on port {}", port);
                }
                ServerEvent::Stopped => {
                    tracing::info!("Remote server stopped");
                }
            }
        }

        // Only sync remote streams to MCP server when there are actual changes
        if has_stream_changes {
            if let Some(ref mcp_server) = self.mcp_server {
                mcp_server.update_remote_streams(self.remote_server.streams());
            }
        }
    }

    /// Update filtered indices
    fn update_filter(&mut self) {
        let indices = self.filter.apply(&self.buffer);
        self.filtered_indices = indices.to_vec();
        self.filter_active = self.filter.is_filtering();
    }

    /// Clear the log buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.filtered_indices.clear();
        self.filter.mark_dirty();
        self.main_view.clear_selection();
        self.status_bar
            .set_message("Display cleared", StatusLevel::Info);
    }

    /// Handle keyboard shortcuts
    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Option<AppAction> {
        // Check for shortcuts using ctx.input_mut
        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.open_file)) {
            return Some(AppAction::OpenFileDialog);
        }

        // Reload file shortcut (Cmd+Shift+R) - check before toggle_reverse_order (Cmd+R)
        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.reload_file)) {
            self.reload_file();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.find)) {
            self.search_bar.toggle();
            self.toolbar_state.search_visible = self.search_bar.visible;
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.find_next)) {
            if let Some(m) = self.filter.search.next() {
                self.main_view.scroll_to_line(m.buffer_index);
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.find_prev)) {
            if let Some(m) = self.filter.search.previous() {
                self.main_view.scroll_to_line(m.buffer_index);
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.clear)) {
            self.clear_buffer();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.goto_line)) {
            self.goto_dialog.open = true;
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.toggle_auto_scroll)) {
            self.main_view.toggle_auto_scroll();
            self.toolbar_state.auto_scroll = self.main_view.is_auto_scroll();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.toggle_reverse_order)) {
            self.main_view.toggle_reverse_order();
            self.toolbar_state.reverse_order = self.main_view.is_reverse_order();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.goto_top)) {
            self.main_view.scroll_to_top();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.goto_bottom)) {
            self.main_view.scroll_to_bottom();
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.copy)) {
            let filtered = if self.filter_active {
                Some(self.filtered_indices.as_slice())
            } else {
                None
            };
            if let Some(text) = self.main_view.get_selected_text(&self.buffer, filtered) {
                let lines_count = self.main_view.selected_lines_count();
                ctx.copy_text(text);
                if lines_count > 1 {
                    self.status_bar.set_message(
                        format!("Copied {} lines to clipboard", lines_count),
                        StatusLevel::Info,
                    );
                } else {
                    self.status_bar
                        .set_message("Copied to clipboard", StatusLevel::Info);
                }
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.toggle_bookmark)) {
            let filtered = if self.filter_active {
                Some(self.filtered_indices.as_slice())
            } else {
                None
            };
            let indices = self.main_view.get_selected_indices(filtered);
            if !indices.is_empty() {
                let count = self.buffer.toggle_bookmarks(&indices);
                if count > 1 {
                    self.status_bar.set_message(
                        format!("Toggled bookmarks on {} lines", count),
                        StatusLevel::Info,
                    );
                }
                // Mark filter as dirty to update bookmark-only filter
                if self.filter.filter.bookmarks_only {
                    self.update_filter();
                }

                // Auto-save bookmarks
                let current_path = self.current_file.clone();
                if let Some(path) = current_path {
                    self.save_current_bookmarks(&path);
                }
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.select_all)) {
            let total_rows = if self.filter_active {
                self.filtered_indices.len()
            } else {
                self.buffer.len()
            };
            if total_rows > 0 {
                self.main_view.select_all(total_rows);
            }
            return None;
        }

        None
    }

    /// Handle context menu actions
    fn handle_context_menu_action(&mut self, action: ContextMenuAction, ctx: egui::Context) {
        match action {
            ContextMenuAction::Copy => {
                let filtered = if self.filter_active {
                    Some(self.filtered_indices.as_slice())
                } else {
                    None
                };
                if let Some(text) = self.main_view.get_selected_text(&self.buffer, filtered) {
                    let lines_count = self.main_view.selected_lines_count();
                    ctx.copy_text(text);
                    if lines_count > 1 {
                        self.status_bar.set_message(
                            format!("Copied {} lines to clipboard", lines_count),
                            StatusLevel::Info,
                        );
                    } else {
                        self.status_bar
                            .set_message("Copied to clipboard", StatusLevel::Info);
                    }
                }
            }
            ContextMenuAction::CopyAll => {
                let filtered = if self.filter_active {
                    Some(self.filtered_indices.as_slice())
                } else {
                    None
                };
                let text = self.get_all_visible_text(filtered);
                if !text.is_empty() {
                    let lines_count = text.lines().count();
                    ctx.copy_text(text);
                    self.status_bar.set_message(
                        format!("Copied {} lines to clipboard", lines_count),
                        StatusLevel::Info,
                    );
                }
            }
            ContextMenuAction::ToggleBookmark => {
                let filtered = if self.filter_active {
                    Some(self.filtered_indices.as_slice())
                } else {
                    None
                };
                let indices = self.main_view.get_selected_indices(filtered);
                if !indices.is_empty() {
                    let count = self.buffer.toggle_bookmarks(&indices);
                    if count > 1 {
                        self.status_bar.set_message(
                            format!("Toggled bookmarks on {} lines", count),
                            StatusLevel::Info,
                        );
                    }
                    // Mark filter as dirty to update bookmark-only filter
                    if self.filter.filter.bookmarks_only {
                        self.update_filter();
                    }

                    // Auto-save bookmarks
                    let current_path = self.current_file.clone();
                    if let Some(path) = current_path {
                        self.save_current_bookmarks(&path);
                    }
                }
            }
            ContextMenuAction::ClearSelection => {
                self.main_view.clear_selection();
            }
            ContextMenuAction::SelectAll => {
                // Already handled in main_view
            }
            ContextMenuAction::ScrollToTop => {
                self.main_view.scroll_to_top();
            }
            ContextMenuAction::ScrollToBottom => {
                self.main_view.scroll_to_bottom();
            }
        }
    }

    /// Get all visible text for copy
    fn get_all_visible_text(&self, filtered_indices: Option<&[usize]>) -> String {
        let mut lines = Vec::new();
        if let Some(indices) = filtered_indices {
            for &idx in indices {
                if let Some(entry) = self.buffer.get(idx) {
                    lines.push(entry.content.clone());
                }
            }
        } else {
            for i in 0..self.buffer.len() {
                if let Some(entry) = self.buffer.get(i) {
                    lines.push(entry.content.clone());
                }
            }
        }
        lines.join("\n")
    }

    /// Handle toolbar actions
    fn handle_toolbar_action(&mut self, action: ToolbarAction) -> Option<AppAction> {
        match action {
            ToolbarAction::OpenFile => Some(AppAction::OpenFileDialog),
            ToolbarAction::ReloadFile => {
                self.reload_file();
                None
            }
            ToolbarAction::ToggleAutoScroll => {
                self.main_view.toggle_auto_scroll();
                self.toolbar_state.auto_scroll = self.main_view.is_auto_scroll();
                None
            }
            ToolbarAction::Clear => {
                self.clear_buffer();
                None
            }
            ToolbarAction::ToggleSearch => {
                self.search_bar.toggle();
                self.toolbar_state.search_visible = self.search_bar.visible;
                None
            }
            ToolbarAction::GoToLine => {
                self.goto_dialog.open = true;
                None
            }
            ToolbarAction::GoToTop => {
                self.main_view.scroll_to_top();
                None
            }
            ToolbarAction::GoToBottom => {
                self.main_view.scroll_to_bottom();
                None
            }
            ToolbarAction::ToggleTheme => {
                self.config.theme.toggle();
                self.toolbar_state.dark_theme = self.config.theme == Theme::Dark;
                self.main_view.set_dark_theme(self.toolbar_state.dark_theme);
                self.settings_panel.dark_theme = self.toolbar_state.dark_theme;
                self.global_search_panel
                    .set_dark_theme(self.toolbar_state.dark_theme);
                let _ = self.config.save();
                Some(AppAction::UpdateTheme)
            }
            ToolbarAction::OpenSettings => {
                // Switch to settings view in sidebar
                self.sidebar_visible = true;
                self.activity_bar.active_view = ActivityView::Settings;
                None
            }
            ToolbarAction::ToggleReverseOrder => {
                self.main_view.toggle_reverse_order();
                self.toolbar_state.reverse_order = self.main_view.is_reverse_order();
                None
            }
            ToolbarAction::None => None,
        }
    }

    /// Handle search bar actions
    fn handle_search_action(&mut self, action: SearchBarAction) {
        match action {
            SearchBarAction::SearchChanged => {
                self.filter.search.search(&self.buffer);
                self.update_filter();
            }
            SearchBarAction::FindNext => {
                if let Some(m) = self.filter.search.next() {
                    self.main_view.scroll_to_line(m.buffer_index);
                }
            }
            SearchBarAction::FindPrev => {
                if let Some(m) = self.filter.search.previous() {
                    self.main_view.scroll_to_line(m.buffer_index);
                }
            }
            SearchBarAction::Close => {
                self.search_bar.close();
                self.toolbar_state.search_visible = false;
            }
            SearchBarAction::None => {}
        }
    }
}

impl eframe::App for LoglineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Initialize system tray after event loop has started (macOS requirement)
        if !self.tray_initialized {
            self.tray_initialized = true;
            match TrayManager::new() {
                Ok(tray) => {
                    tracing::info!("System tray initialized");
                    self.tray_manager = Some(tray);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize system tray: {}", e);
                }
            }
        }

        // Handle system tray events
        if let Some(ref tray) = self.tray_manager {
            if let Some(event) = tray.poll_events() {
                match event {
                    TrayEvent::ShowWindow => {
                        // Restore the window
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    TrayEvent::Quit => {
                        // Actually quit the application
                        self.should_quit = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }

        // Handle window close button - minimize to tray instead of quitting
        if ctx.input(|i| i.viewport().close_requested()) && !self.should_quit {
            if self.tray_manager.is_some() {
                // Prevent the window from closing
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                // Hide the window instead
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                tracing::info!("Window minimized to tray");
            }
        }

        // Apply theme on first frame to ensure it takes effect after eframe initialization
        if self.first_frame {
            self.first_frame = false;
            let visuals = match self.config.theme {
                Theme::Dark => egui::Visuals::dark(),
                Theme::Light => egui::Visuals::light(),
            };
            ctx.set_visuals(visuals);
        }

        // Process background messages
        self.process_reader_messages();

        // Process remote server events
        self.process_server_events();

        // Rate-limit filter updates
        if self.last_update.elapsed() > Duration::from_millis(16) {
            if self.pending_entries > 0 {
                self.update_filter();
                self.pending_entries = 0;
            }
            self.last_update = Instant::now();
        }

        // Handle shortcuts
        let mut action = self.handle_shortcuts(ctx);

        // Top panel with toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            let toolbar_action = Toolbar::show(ui, &mut self.toolbar_state);
            if let Some(a) = self.handle_toolbar_action(toolbar_action) {
                action = Some(a);
            }
        });

        // Search bar panel
        if self.search_bar.visible {
            egui::TopBottomPanel::top("search").show(ctx, |ui| {
                let search_action = self.search_bar.show(ui, &mut self.filter.search);
                self.handle_search_action(search_action);
            });
        }

        // Filter panel
        egui::TopBottomPanel::top("filter").show(ctx, |ui| {
            if self.filter_panel.show(ui, &mut self.filter.filter) {
                self.update_filter();
            }
        });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if let Some(action) = self.status_bar.show(
                ui,
                self.current_file.as_deref(),
                &self.buffer,
                self.reader.as_ref(),
                self.main_view.is_auto_scroll(),
                if self.filter_active {
                    Some(self.filtered_indices.len())
                } else {
                    None
                },
                self.main_view.selected_lines_count(),
            ) {
                match action {
                    crate::ui::status_bar::StatusBarAction::ChangeEncoding(encoding) => {
                        self.change_encoding(encoding);
                    }
                }
            }
        });

        // === New: Activity Bar (leftmost narrow panel) ===
        egui::SidePanel::left("activity_bar")
            .exact_width(48.0)
            .resizable(false)
            .show(ctx, |ui| {
                // Update server status in activity bar
                self.activity_bar.server_running = self.remote_server.is_running();
                self.activity_bar.server_port = self.settings_panel.port();
                self.activity_bar.connected_agents = self
                    .remote_server
                    .streams()
                    .iter()
                    .filter(|s| s.status == crate::remote_server::ConnectionStatus::Online)
                    .count();

                match self.activity_bar.show(ui) {
                    ActivityBarAction::SwitchView(view) => {
                        self.sidebar_visible = true;
                        self.activity_bar.active_view = view;
                    }
                    ActivityBarAction::TogglePanel => {
                        self.sidebar_visible = !self.sidebar_visible;
                    }
                    ActivityBarAction::ToggleServer => {
                        if self.remote_server.is_running() {
                            self.remote_server.stop();
                            self.status_bar
                                .set_message("远程服务已停止", StatusLevel::Info);
                        } else {
                            // Update port from settings before starting
                            let port = self.settings_panel.port();
                            self.remote_server.set_port(port);

                            match self.remote_server.start() {
                                Ok(()) => {
                                    self.status_bar.set_message(
                                        format!("远程服务已启动 (端口 {})", port),
                                        StatusLevel::Success,
                                    );
                                }
                                Err(e) => {
                                    self.status_bar.set_message(
                                        format!("启动服务失败: {}", e),
                                        StatusLevel::Error,
                                    );
                                }
                            }
                        }
                    }
                    ActivityBarAction::None => {}
                }
            });

        // === New: Sidebar Panel ===
        if self.sidebar_visible {
            egui::SidePanel::left("sidebar")
                .min_width(200.0)
                .default_width(250.0)
                .max_width(400.0)
                .show(ctx, |ui| {
                    match self.activity_bar.active_view {
                        ActivityView::Explorer => {
                            // Update remote streams
                            self.explorer_panel
                                .update_remote_streams(self.remote_server.streams());

                            match self.explorer_panel.show(ui) {
                                ExplorerAction::SelectEditor(idx) => {
                                    // Get editor path first to avoid borrow conflicts
                                    let editor_path = self
                                        .explorer_panel
                                        .open_editors
                                        .get(idx)
                                        .map(|e| e.path.clone());

                                    if let Some(path) = editor_path {
                                        // Switch to the selected editor/file
                                        if self.current_file.as_ref() != Some(&path) {
                                            // Save bookmarks for current file before switching
                                            let current_path = self.current_file.clone();
                                            if let Some(current) = current_path {
                                                self.save_current_bookmarks(&current);
                                            }

                                            // Clear buffer before switching
                                            self.buffer.clear();
                                            self.filtered_indices.clear();
                                            self.main_view.clear_selection();

                                            let _ = self.open_file(path, None);

                                            // Scroll to bottom to show latest logs
                                            self.main_view.scroll_to_bottom();
                                        }
                                    }
                                }
                                ExplorerAction::CloseEditor(idx, editor) => {
                                    // Check if this is the currently displayed file
                                    let is_current =
                                        self.current_file.as_ref() == Some(&editor.path);

                                    // Close editor from list
                                    self.explorer_panel.close_editor(idx);

                                    if is_current {
                                        if editor.is_remote {
                                            // Remote log: clear display but keep background connection
                                            self.buffer.clear();
                                            self.filtered_indices.clear();
                                            self.main_view.clear_selection();
                                            self.current_file = None;
                                            // Stop the reader but don't stop remote server
                                            if let Some(tx) = self.reader_tx.take() {
                                                let _ = tx.send(ReaderCommand::Stop);
                                            }
                                            self.reader_rx = None;
                                            if let Some(watcher) = self.watcher.take() {
                                                watcher.stop();
                                            }
                                            self.reader = None;
                                        } else {
                                            // Local file: close file completely
                                            self.close_file();
                                            self.buffer.clear();
                                            self.filtered_indices.clear();
                                            self.main_view.clear_selection();
                                        }

                                        // If there's another open editor, switch to it
                                        if let Some(selected_idx) =
                                            self.explorer_panel.selected_editor
                                        {
                                            if let Some(next_editor) =
                                                self.explorer_panel.open_editors.get(selected_idx)
                                            {
                                                let _ =
                                                    self.open_file(next_editor.path.clone(), None);
                                                self.main_view.scroll_to_bottom();
                                            }
                                        }
                                    }
                                }
                                ExplorerAction::OpenLocalFile(path) => {
                                    // Save bookmarks for current file before opening new one
                                    let current_path = self.current_file.clone();
                                    if let Some(path_to_save) = current_path {
                                        self.save_current_bookmarks(&path_to_save);
                                    }

                                    // Clear buffer before opening
                                    self.buffer.clear();
                                    self.filtered_indices.clear();
                                    self.main_view.clear_selection();

                                    if let Err(e) = self.open_file(path.clone(), None) {
                                        self.status_bar.set_message(
                                            format!("打开文件失败: {}", e),
                                            StatusLevel::Error,
                                        );
                                    } else {
                                        // Add to open editors
                                        let name = path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| path.display().to_string());
                                        self.explorer_panel.add_editor(OpenEditor {
                                            name,
                                            path: path.clone(),
                                            is_remote: false,
                                            is_dirty: false,
                                        });
                                        // Scroll to bottom to show latest logs
                                        self.main_view.scroll_to_bottom();
                                    }
                                }
                                ExplorerAction::OpenRemoteStream(stream) => {
                                    // Save bookmarks for current file before opening new one
                                    let current_path = self.current_file.clone();
                                    if let Some(path) = current_path {
                                        self.save_current_bookmarks(&path);
                                    }

                                    // Clear buffer before opening
                                    self.buffer.clear();
                                    self.filtered_indices.clear();
                                    self.main_view.clear_selection();

                                    // Open the cache file for the remote stream
                                    if let Err(e) = self.open_file(stream.cache_path.clone(), None)
                                    {
                                        self.status_bar.set_message(
                                            format!("打开远程流失败: {}", e),
                                            StatusLevel::Error,
                                        );
                                    } else {
                                        // Add to open editors
                                        self.explorer_panel.add_editor(OpenEditor {
                                            name: stream.project_name.clone(),
                                            path: stream.cache_path,
                                            is_remote: true,
                                            is_dirty: false,
                                        });
                                        // Scroll to bottom to show latest logs
                                        self.main_view.scroll_to_bottom();
                                    }
                                }
                                ExplorerAction::OpenFileDialog => {
                                    action = Some(AppAction::OpenFileDialog);
                                }
                                ExplorerAction::None => {}
                            }
                        }
                        ActivityView::Search => {
                            // Global search view
                            match self.global_search_panel.show(ui, &self.buffer) {
                                GlobalSearchAction::JumpToLine(buffer_index) => {
                                    // Jump to the line in main view
                                    self.main_view.scroll_to_line(buffer_index);
                                    // Select the line
                                    self.main_view.set_selection(buffer_index, buffer_index);
                                }
                                GlobalSearchAction::None => {}
                            }
                        }
                        ActivityView::Filters => {
                            // Advanced filters view
                            if self
                                .advanced_filters_panel
                                .show(ui, &mut self.filter.filter)
                            {
                                // Filter changed, update the view
                                self.filter.mark_dirty();
                                self.update_filter();
                            }
                        }
                        ActivityView::Bookmarks => {
                            // Bookmarks view
                            match self.bookmarks_panel.show(ui, &self.buffer) {
                                BookmarkAction::JumpToLine(line_number) => {
                                    // Find the buffer index for this line number
                                    let index = self
                                        .buffer
                                        .iter()
                                        .position(|e| e.line_number == line_number);

                                    if let Some(index) = index {
                                        self.main_view.scroll_to_line(index);
                                        self.main_view.set_selection(index, index);
                                    }
                                }
                                BookmarkAction::RemoveBookmark(line_number) => {
                                    // Find and remove bookmark
                                    let index = self
                                        .buffer
                                        .iter()
                                        .position(|e| e.line_number == line_number);

                                    if let Some(index) = index {
                                        self.buffer.toggle_bookmark(index);
                                        if self.filter.filter.bookmarks_only {
                                            self.filter.mark_dirty();
                                            self.update_filter();
                                        }

                                        // Auto-save bookmarks
                                        let current_path = self.current_file.clone();
                                        if let Some(path) = current_path {
                                            self.save_current_bookmarks(&path);
                                        }
                                    }
                                }
                                BookmarkAction::RemoveSegment(indices) => {
                                    // Remove all bookmarks in the segment
                                    let count = self.buffer.toggle_bookmarks(&indices);
                                    if count > 0 {
                                        if self.filter.filter.bookmarks_only {
                                            self.filter.mark_dirty();
                                            self.update_filter();
                                        }

                                        // Auto-save bookmarks
                                        let current_path = self.current_file.clone();
                                        if let Some(path) = current_path {
                                            self.save_current_bookmarks(&path);
                                        }
                                    }
                                }
                                BookmarkAction::ClearAll => {
                                    // Clear all bookmarks
                                    let all_indices: Vec<usize> = self
                                        .buffer
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, e)| e.bookmarked)
                                        .map(|(i, _)| i)
                                        .collect();
                                    if !all_indices.is_empty() {
                                        self.buffer.toggle_bookmarks(&all_indices);
                                        if self.filter.filter.bookmarks_only {
                                            self.filter.mark_dirty();
                                            self.update_filter();
                                        }
                                        self.status_bar
                                            .set_message("所有书签已清除", StatusLevel::Info);

                                        // Auto-save bookmarks
                                        let current_path = self.current_file.clone();
                                        if let Some(path) = current_path {
                                            self.save_current_bookmarks(&path);
                                        }
                                    }
                                }
                                BookmarkAction::None => {}
                            }
                        }
                        ActivityView::Settings => {
                            match self.settings_panel.show(ui) {
                                SettingsAction::ThemeChanged(dark) => {
                                    self.config.theme =
                                        if dark { Theme::Dark } else { Theme::Light };
                                    self.toolbar_state.dark_theme = dark;
                                    self.main_view.set_dark_theme(dark);
                                    self.global_search_panel.set_dark_theme(dark);
                                    action = Some(AppAction::UpdateTheme);
                                    let _ = self.config.save();
                                }
                                SettingsAction::PortChanged => {
                                    // Save the port change to config
                                    self.config.remote_server.port = self.settings_panel.port();
                                    let _ = self.config.save();
                                    // Port change will take effect on next server restart
                                    self.status_bar.set_message(
                                        "端口变更将在重启服务后生效",
                                        StatusLevel::Info,
                                    );
                                }
                                SettingsAction::AutoStartChanged => {
                                    self.config.remote_server.auto_start =
                                        self.settings_panel.auto_start_server;
                                    let _ = self.config.save();
                                }
                                SettingsAction::LanguageChanged(lang) => {
                                    set_language(lang);
                                    self.config.language = lang;
                                    let _ = self.config.save();
                                }
                                SettingsAction::DisplayConfigChanged => {
                                    self.display_config =
                                        self.settings_panel.display_config.clone();
                                    self.config.display = self.display_config.clone();
                                    let _ = self.config.save();
                                }
                                SettingsAction::McpEnabledChanged(enabled) => {
                                    self.config.mcp.enabled = enabled;
                                    let _ = self.config.save();

                                    if enabled {
                                        // Start MCP server
                                        self.start_mcp_server();
                                    } else {
                                        // Stop MCP server
                                        self.stop_mcp_server();
                                    }
                                }
                                SettingsAction::McpPortChanged => {
                                    self.config.mcp.port = self.settings_panel.mcp_port_number();
                                    let _ = self.config.save();
                                    self.status_bar.set_message(
                                        "MCP端口变更将在重启服务后生效",
                                        StatusLevel::Info,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                });
        }

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            let filtered = if self.filter_active {
                Some(self.filtered_indices.as_slice())
            } else {
                None
            };

            let (_, context_action) = self.main_view.show(
                ui,
                &self.buffer,
                filtered,
                &self.filter.search,
                &self.display_config,
            );

            // Handle context menu actions
            if let Some(ctx_action) = context_action {
                self.handle_context_menu_action(ctx_action, ctx.clone());
            }
        });

        // Go to line dialog
        if self.goto_dialog.open {
            self.show_goto_dialog(ctx);
        }

        // File picker dialog
        match self.file_picker_dialog.show(ctx) {
            FilePickerAction::OpenFile(path, encoding) => {
                // Clear buffer before opening
                self.buffer.clear();
                self.filtered_indices.clear();
                self.main_view.clear_selection();

                if let Err(e) = self.open_file(path.clone(), encoding) {
                    self.status_bar
                        .set_message(format!("打开文件失败: {}", e), StatusLevel::Error);
                } else {
                    // Add to explorer
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    self.explorer_panel.add_editor(OpenEditor {
                        name,
                        path: path.clone(),
                        is_remote: false,
                        is_dirty: false,
                    });
                    // Scroll to bottom to show latest logs
                    self.main_view.scroll_to_bottom();
                }
            }
            FilePickerAction::Cancel => {}
            FilePickerAction::None => {}
        }

        // Handle actions
        if let Some(action) = action {
            match action {
                AppAction::OpenFileDialog => {
                    // Update recent files for file picker dialog
                    let recent_files: Vec<PathBuf> = self
                        .explorer_panel
                        .open_editors
                        .iter()
                        .filter(|e| !e.is_remote)
                        .map(|e| e.path.clone())
                        .collect();
                    self.file_picker_dialog.set_recent_files(recent_files);

                    // Show the new file picker dialog
                    self.file_picker_dialog.show_dialog();
                }
                AppAction::UpdateTheme => {
                    let visuals = match self.config.theme {
                        Theme::Dark => egui::Visuals::dark(),
                        Theme::Light => egui::Visuals::light(),
                    };
                    ctx.set_visuals(visuals);
                }
            }
        }

        // Request repaint for real-time updates only when actively viewing a file
        // Reduced repaint frequency when remote server is running but no file is open
        if self.watcher.is_some() {
            // Actively watching a file, need frequent updates
            ctx.request_repaint_after(Duration::from_millis(50));
        } else if self.remote_server.is_running() {
            // Server is running but no active file, less frequent updates
            ctx.request_repaint_after(Duration::from_millis(200));
        } else if self.tray_manager.is_some() {
            // Tray is active, need periodic repaint to handle tray events
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save configuration
        let _ = self.config.save();

        // Stop remote server
        self.remote_server.stop();

        // Close file
        self.close_file();
    }
}

impl LoglineApp {
    /// Show go-to-line dialog
    fn show_goto_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Go to Line")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Line number:");
                    let response = ui.text_edit_singleline(&mut self.goto_dialog.input);

                    if self.goto_dialog.focus_input {
                        response.request_focus();
                        self.goto_dialog.focus_input = false;
                    }

                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.goto_dialog.submit = true;
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() {
                        self.goto_dialog.submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.goto_dialog.open = false;
                    }
                });

                if self.goto_dialog.submit {
                    if let Ok(line) = self.goto_dialog.input.parse::<usize>() {
                        // Find index for this line number
                        if let Some((idx, _)) = self
                            .buffer
                            .iter()
                            .enumerate()
                            .find(|(_, e)| e.line_number == line)
                        {
                            self.main_view.scroll_to_line(idx);
                        }
                    }
                    self.goto_dialog.open = false;
                    self.goto_dialog.submit = false;
                    self.goto_dialog.input.clear();
                }
            });
    }
}

/// Application actions
#[derive(Debug, Clone, Copy)]
enum AppAction {
    OpenFileDialog,
    UpdateTheme,
}

/// Go-to-line dialog state
#[derive(Default)]
struct GotoLineDialog {
    open: bool,
    input: String,
    focus_input: bool,
    submit: bool,
}
