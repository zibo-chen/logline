//! Main application logic

use crate::config::{AppConfig, DisplayConfig, Shortcuts, Theme};
use crate::file_watcher::FileWatcher;
use crate::i18n::set_language;
use crate::log_buffer::{LogBuffer, LogBufferConfig};
use crate::log_entry::LogEntry;
use crate::log_reader::LogReader;
use crate::remote_server::{RemoteServer, ServerConfig, ServerEvent};
use crate::search::LogFilter;
use crate::ui::activity_bar::{ActivityBar, ActivityBarAction, ActivityView};
use crate::ui::explorer_panel::{ExplorerAction, ExplorerPanel, OpenEditor};
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
            last_update: Instant::now(),
            pending_entries: 0,
            // New components
            remote_server,
            activity_bar: ActivityBar::new(),
            explorer_panel: ExplorerPanel::new(),
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
    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        // Stop any existing reader
        self.close_file();

        // Create reader
        let mut reader = LogReader::new(&path)?;

        // Read initial content
        let entries = reader.read_all()?;
        self.buffer.extend(entries);

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

        thread::spawn(move || {
            Self::reader_thread(
                reader_path,
                reader_offset,
                reader_line_count,
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

        // Update recent files
        self.config.add_recent_file(path.clone());
        let _ = self.config.save();

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
            // Close and clear
            self.close_file();
            self.buffer.clear();
            self.filtered_indices.clear();
            self.main_view.clear_selection();

            // Reopen the file
            match self.open_file(path) {
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

    /// Background reader thread function
    fn reader_thread(
        path: PathBuf,
        initial_offset: u64,
        initial_line_count: usize,
        msg_tx: Sender<ReaderMessage>,
        cmd_rx: Receiver<ReaderCommand>,
    ) {
        let mut reader = match LogReader::new(&path) {
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
                    // Check if file was truncated (rotation)
                    if reader.offset() > reader.file_size() {
                        let _ = msg_tx.send(ReaderMessage::FileReset);
                        reader.seek(0);
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

            // Auto-scroll to bottom when new entries arrive
            if self.main_view.is_auto_scroll() {
                self.main_view.scroll_to_bottom();
            }
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
                    cache_path,
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

                    // Use full address (IP:port) to distinguish multiple agents with same name
                    let display_name = format!("{}@{}", project_name, remote_addr);

                    // Automatically open the cache file for viewing
                    if let Err(e) = self.open_file(cache_path.clone()) {
                        tracing::error!("Failed to open cache file: {}", e);
                    } else {
                        self.explorer_panel.add_editor(OpenEditor {
                            name: display_name,
                            path: cache_path,
                            is_remote: true,
                            is_dirty: false,
                        });
                    }
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
            self.status_bar.show(
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
            );
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
                                    if let Some(editor) = self.explorer_panel.open_editors.get(idx)
                                    {
                                        // Switch to the selected editor/file
                                        if self.current_file.as_ref() != Some(&editor.path) {
                                            // Clear buffer before switching
                                            self.buffer.clear();
                                            self.filtered_indices.clear();
                                            self.main_view.clear_selection();

                                            let _ = self.open_file(editor.path.clone());

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
                                                let _ = self.open_file(next_editor.path.clone());
                                                self.main_view.scroll_to_bottom();
                                            }
                                        }
                                    }
                                }
                                ExplorerAction::OpenLocalFile(path) => {
                                    // Clear buffer before opening
                                    self.buffer.clear();
                                    self.filtered_indices.clear();
                                    self.main_view.clear_selection();

                                    if let Err(e) = self.open_file(path) {
                                        self.status_bar.set_message(
                                            format!("打开文件失败: {}", e),
                                            StatusLevel::Error,
                                        );
                                    } else {
                                        // Scroll to bottom to show latest logs
                                        self.main_view.scroll_to_bottom();
                                    }
                                }
                                ExplorerAction::OpenRemoteStream(stream) => {
                                    // Clear buffer before opening
                                    self.buffer.clear();
                                    self.filtered_indices.clear();
                                    self.main_view.clear_selection();

                                    // Open the cache file for the remote stream
                                    if let Err(e) = self.open_file(stream.cache_path.clone()) {
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

        // Handle actions
        if let Some(action) = action {
            match action {
                AppAction::OpenFileDialog => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log files", &["log", "txt", "json"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        if let Err(e) = self.open_file(path.clone()) {
                            self.status_bar.set_message(
                                format!("Failed to open file: {}", e),
                                StatusLevel::Error,
                            );
                        } else {
                            // Add to explorer and recent files
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
                            self.explorer_panel.add_recent_file(path);
                        }
                    }
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
