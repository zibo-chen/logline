//! Main application logic

use crate::bookmarks::BookmarksStore;
use crate::config::{AppConfig, DisplayConfig, Shortcuts, Theme};
use crate::grok_parser::GrokParser;
use crate::i18n::{set_language, Translations as t};
use crate::log_buffer::LogBufferConfig;
use crate::remote_server::{RemoteServer, ServerConfig, ServerEvent};
use crate::tray::{TrayEvent, TrayManager};
use crate::ui::activity_bar::{ActivityBar, ActivityBarAction, ActivityView};
use crate::ui::advanced_filters_panel::AdvancedFiltersPanel;
use crate::ui::app_titlebar::AppTitleBar;
use crate::ui::bookmarks_panel::{BookmarkAction, BookmarksPanel};
use crate::ui::explorer_panel::{ExplorerAction, ExplorerPanel};
use crate::ui::file_picker_dialog::{FilePickerAction, FilePickerDialog};
use crate::ui::filter_panel::FilterPanel;
use crate::ui::global_search_panel::{GlobalSearchAction, GlobalSearchPanel};
use crate::ui::grok_panel::{GrokPanel, GrokPanelAction};
use crate::ui::main_view::ContextMenuAction;
use crate::ui::search_bar::{SearchBar, SearchBarAction};
use crate::ui::settings_panel::{SettingsAction, SettingsPanel};
use crate::ui::status_bar::{StatusBar, StatusLevel};
use crate::ui::tab_bar::TabBarAction;
use crate::ui::tab_manager::TabManager;
use crate::ui::toolbar::{Toolbar, ToolbarAction, ToolbarState};

use crate::mcp::{McpConfig, McpServer};

use anyhow::Result;
use eframe::egui;
use egui::RichText;
use egui_desktop::{TitleBar, TitleBarOptions, ThemeMode};
#[cfg(any(target_os = "windows", target_os = "linux"))]
use egui_desktop::render_resize_handles;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Main application state
pub struct LoglineApp {
    /// Application configuration
    config: AppConfig,
    /// Display configuration
    display_config: DisplayConfig,
    /// Keyboard shortcuts
    shortcuts: Shortcuts,

    /// Tab manager for multiple log files
    tab_manager: TabManager,

    /// Search bar
    search_bar: SearchBar,
    /// Application title bar with search
    app_titlebar: AppTitleBar,
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
    /// Grok parser panel
    grok_panel: GrokPanel,
    /// Grok parser instance
    grok_parser: GrokParser,
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

    // === Custom Titlebar ===
    /// Custom title bar for the window
    title_bar: TitleBar,
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
            tab_manager: {
                let buffer_config = LogBufferConfig {
                    max_lines: config.buffer.max_lines,
                    auto_trim: config.buffer.auto_trim,
                    chunk_size: 5_000,     // Load 5k lines per chunk when scrolling up
                };
                let mut manager = TabManager::new(buffer_config);
                manager.set_dark_theme(config.theme == Theme::Dark);
                manager
            },
            search_bar: SearchBar::new(),
            app_titlebar: AppTitleBar::new(),
            filter_panel: FilterPanel::new(),
            status_bar: StatusBar::new(),
            toolbar_state: ToolbarState {
                auto_scroll: true,
                search_visible: false,
                dark_theme: config.theme == Theme::Dark,
                reverse_order: false,
                split_view_active: false,
            },
            goto_dialog: GotoLineDialog::default(),
            file_picker_dialog: FilePickerDialog::new(),
            last_update: Instant::now(),
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
            grok_panel: {
                let mut panel = GrokPanel::new();
                panel.load_from_config(&config.grok);
                panel
            },
            grok_parser: {
                let mut parser = GrokParser::new();
                // Load custom patterns from config
                parser.import_custom_patterns(config.grok.custom_patterns.clone());
                // Load custom definitions
                for (name, pattern) in &config.grok.custom_definitions {
                    parser.add_pattern_definition(name, pattern);
                }
                // Set active pattern if configured
                if let Some(builtin) = config.grok.builtin_pattern {
                    let _ = parser.set_builtin_pattern(builtin);
                } else if let Some(ref custom_name) = config.grok.custom_pattern_name {
                    if let Some(custom) = config.grok.custom_patterns.iter().find(|p| &p.name == custom_name) {
                        let _ = parser.set_custom_pattern(&custom.name, &custom.pattern);
                    }
                }
                parser
            },
            // Custom titlebar - must be before config is moved
            title_bar: TitleBar::new(
                TitleBarOptions::new()
                    .with_title("Logline")
                    .with_theme_mode(if config.theme == Theme::Dark {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    }),
            ),
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

    /// Open a log file in a new tab
    pub fn open_file(
        &mut self,
        path: PathBuf,
        encoding: Option<&'static encoding_rs::Encoding>,
    ) -> Result<()> {
        // If no encoding specified, try to get saved encoding for this file
        let final_encoding = encoding.or_else(|| self.config.get_file_encoding(&path));

        // Open in tab manager
        let tab_id = self.tab_manager.open_local_file(path.clone(), final_encoding, &self.bookmarks_store)?;

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
            mcp_server.add_local_file(path.clone());
        }

        // Restore saved grok config for this file
        self.restore_file_grok_config(tab_id, &path);

        // Scroll to bottom for new file
        if let Some(state) = self.tab_manager.get_state_mut(tab_id) {
            state.main_view.scroll_to_bottom();
        }

        // Update toolbar state from the newly opened tab
        if let Some(state) = self.tab_manager.states.get(&tab_id) {
            self.toolbar_state.auto_scroll = state.main_view.virtual_scroll.state.auto_scroll;
            self.toolbar_state.reverse_order = state.main_view.virtual_scroll.state.reverse_order;
        }

        self.status_bar
            .set_message("File opened", StatusLevel::Success);

        Ok(())
    }

    /// Open a remote stream in a new tab
    pub fn open_remote_stream(&mut self, project_name: String, cache_path: PathBuf) -> Result<()> {
        // Open in tab manager
        let tab_id = self.tab_manager.open_remote_stream(
            project_name.clone(),
            cache_path.clone(),
            &self.bookmarks_store,
        )?;

        // Scroll to bottom for new stream
        if let Some(state) = self.tab_manager.get_state_mut(tab_id) {
            state.main_view.scroll_to_bottom();
        }

        self.status_bar
            .set_message("Remote stream opened", StatusLevel::Success);

        Ok(())
    }

    /// Open a log file in split view
    pub fn open_file_in_split(&mut self, path: PathBuf) -> Result<()> {
        // First, try to open the file in a new tab if it's not already open
        let tab_id = self.tab_manager.open_local_file(path.clone(), None, &self.bookmarks_store)?;
        
        // Then open it in split view
        self.tab_manager.enable_split(tab_id);
        
        self.status_bar
            .set_message("File opened in split view", StatusLevel::Success);
        
        Ok(())
    }

    /// Close a tab by ID
    pub fn close_tab(&mut self, tab_id: crate::ui::tab_bar::TabId) {
        self.tab_manager.close_tab(tab_id, &mut self.bookmarks_store);
    }

    /// Reload the current file
    pub fn reload_file(&mut self) {
        if let Err(e) = self.tab_manager.reload_active(&self.bookmarks_store) {
            self.status_bar
                .set_message(format!("Reload failed: {}", e), StatusLevel::Error);
        } else {
            self.status_bar
                .set_message("File reloaded", StatusLevel::Success);
        }
    }

    /// Change the encoding of the current file
    pub fn change_encoding(&mut self, encoding: Option<&'static encoding_rs::Encoding>) {
        if let Some(state) = self.tab_manager.get_active_state() {
            let path = state.path.clone();
            
            // Save encoding preference
            self.config.set_file_encoding(path.clone(), encoding);
            let _ = self.config.save();

            // Close the current tab and reopen with new encoding
            if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                self.tab_manager.close_tab(tab_id, &mut self.bookmarks_store);
                
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
    }

    /// Change the grok pattern for the current tab only
    pub fn change_grok_pattern(&mut self, selection: crate::ui::status_bar::GrokPatternSelection) {
        use crate::config::FileGrokConfig;
        use crate::grok_parser::GrokParser;
        use crate::ui::status_bar::GrokPatternSelection;

        // Get the active tab
        let Some(state) = self.tab_manager.get_active_state_mut() else {
            return;
        };

        let file_path = state.path.clone();

        match selection {
            GrokPatternSelection::None => {
                // Disable grok parsing for this tab
                state.grok_parser = None;
                state.grok_config = None;
                state.grok_parse_progress = 0;
                
                // Clear grok fields for this tab only
                for entry in state.buffer.iter_mut() {
                    entry.clear_grok_fields();
                }
                
                // Remove file-specific config
                self.config.set_file_grok_config(file_path, None);
                
                self.status_bar.set_message(t::grok_pattern_cleared(), StatusLevel::Info);
            }
            GrokPatternSelection::Custom(name) => {
                // Find custom pattern from global config
                let custom = self.config.grok.custom_patterns.iter().find(|p| p.name == name).cloned();
                
                let Some(custom) = custom else {
                    self.status_bar.set_message(
                        format!("{}: {}", t::grok_pattern_error(), name),
                        StatusLevel::Error,
                    );
                    return;
                };
                
                // Create a new parser for this tab
                let mut parser = GrokParser::new();
                parser.import_custom_patterns(self.config.grok.custom_patterns.clone());
                for (n, pat) in &self.config.grok.custom_definitions {
                    parser.add_pattern_definition(n, pat);
                }
                
                let template = if custom.display_template.is_empty() {
                    None
                } else {
                    Some(custom.display_template.as_str())
                };
                
                if let Err(e) = parser.set_custom_pattern_with_template(
                    &name,
                    &custom.pattern,
                    template,
                ) {
                    self.status_bar.set_message(
                        format!("{}: {}", t::grok_pattern_error(), e),
                        StatusLevel::Error,
                    );
                    return;
                }
                // Apply pre-processor if present on the custom pattern
                if custom.pre_processor != crate::grok_parser::PreProcessor::None {
                    parser.set_pre_processor(custom.pre_processor.clone());
                }
                
                // Get state again after error check
                let state = self.tab_manager.get_active_state_mut().unwrap();
                
                state.grok_parser = Some(parser);
                state.grok_config = Some(FileGrokConfig {
                    enabled: true,
                    pattern_type: "custom".to_string(),
                    builtin_pattern: None,
                    custom_pattern_name: Some(name.clone()),
                    inline_pattern: None,
                    pre_processor: custom.pre_processor.clone(),
                });
                state.grok_parse_progress = 0;
                
                // Clear grok fields to reparse
                for entry in state.buffer.iter_mut() {
                    entry.clear_grok_fields();
                }
                
                // Save file-specific config
                let config = state.grok_config.clone();
                let path = state.path.clone();
                self.config.set_file_grok_config(path, config);
                
                self.status_bar.set_message(
                    format!("{}: {}", t::grok_active_pattern(), name),
                    StatusLevel::Success,
                );
            }
        }
        
        // Save config
        if let Err(e) = self.config.save() {
            tracing::error!("Failed to save config: {}", e);
        }
    }

    /// Restore saved grok config for a file when opening it
    fn restore_file_grok_config(&mut self, tab_id: crate::ui::tab_bar::TabId, path: &PathBuf) {
        use crate::grok_parser::{GrokParser, BuiltinPattern};
        
        let path_str = path.to_string_lossy().to_string();
        let saved_config = self.config.file_grok_configs.get(&path_str).cloned();
        
        let Some(file_config) = saved_config else {
            return; // No saved config for this file
        };
        
        if !file_config.enabled {
            return; // Grok was disabled for this file
        }
        
        // Create a parser for this tab
        let mut parser = GrokParser::new();
        parser.import_custom_patterns(self.config.grok.custom_patterns.clone());
        for (name, pat) in &self.config.grok.custom_definitions {
            parser.add_pattern_definition(name, pat);
        }
        
        // Set the pattern based on saved config
        let pattern_set = match file_config.pattern_type.as_str() {
            "builtin" => {
                if let Some(ref pattern_name) = file_config.builtin_pattern {
                    // Find builtin pattern by name
                    BuiltinPattern::all()
                        .iter()
                        .find(|p| p.display_name() == pattern_name)
                        .and_then(|pattern| parser.set_builtin_pattern(*pattern).ok())
                        .is_some()
                } else {
                    false
                }
            }
            "custom" => {
                if let Some(ref custom_name) = file_config.custom_pattern_name {
                    if let Some(custom) = self.config.grok.custom_patterns.iter().find(|p| &p.name == custom_name) {
                        let template = if custom.display_template.is_empty() {
                            None
                        } else {
                            Some(custom.display_template.as_str())
                        };
                        let pattern_ok = parser.set_custom_pattern_with_template(custom_name, &custom.pattern, template).is_ok();
                        if pattern_ok {
                            if custom.pre_processor != crate::grok_parser::PreProcessor::None {
                                parser.set_pre_processor(custom.pre_processor.clone());
                            } else if file_config.pre_processor != crate::grok_parser::PreProcessor::None {
                                parser.set_pre_processor(file_config.pre_processor.clone());
                            }
                        }
                        pattern_ok
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "inline" => {
                if let Some(ref inline) = file_config.inline_pattern {
                    let template = if inline.display_template.is_empty() {
                        None
                    } else {
                        Some(inline.display_template.as_str())
                    };
                    let pattern_ok = parser.set_custom_pattern_with_template(&inline.name, &inline.pattern, template).is_ok();
                    if pattern_ok {
                        // Inline patterns may have their own pre_processor
                        // Priority: inline pattern > file config
                        if inline.pre_processor != crate::grok_parser::PreProcessor::None {
                            parser.set_pre_processor(inline.pre_processor.clone());
                        } else if file_config.pre_processor != crate::grok_parser::PreProcessor::None {
                            parser.set_pre_processor(file_config.pre_processor.clone());
                        }
                    }
                    pattern_ok
                } else {
                    false
                }
            }
            _ => false,
        };
        
        if pattern_set {
            // Also restore pre_processor
            parser.set_pre_processor(file_config.pre_processor.clone());
            
            if let Some(state) = self.tab_manager.get_state_mut(tab_id) {
                state.grok_parser = Some(parser);
                state.grok_config = Some(file_config);
                state.grok_parse_progress = 0;
                tracing::info!("Restored grok config for file: {}", path.display());
            }
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
                    let msg = format!("{}: {}", t::mcp_server_started(), endpoint);
                    self.status_bar
                        .set_message(msg, StatusLevel::Success);
                    tracing::info!("MCP server started at {}", endpoint);
                    self.mcp_server = Some(server);
                }
                Err(e) => {
                    let msg = format!("{}: {}", t::mcp_server_start_failed(), e);
                    self.status_bar
                        .set_message(msg, StatusLevel::Error);
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

    /// Clear the active tab's buffer
    pub fn clear_buffer(&mut self) {
        self.tab_manager.clear_active_buffer();
        self.status_bar
            .set_message("Display cleared", StatusLevel::Info);
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
                    let msg = format!("{} '{}' ({})", t::agent_connected(), project_name, remote_addr);
                    self.status_bar.set_message(
                        msg,
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
                    let msg = format!("{} '{}'", t::agent_disconnected(), stream_id);
                    self.status_bar.set_message(
                        msg,
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
                    let msg = format!("{}: {}", t::server_error(), e);
                    self.status_bar
                        .set_message(msg, StatusLevel::Error);
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
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                if let Some(m) = state.filter.search.next() {
                    state.main_view.scroll_to_line(m.buffer_index);
                }
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.find_prev)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                if let Some(m) = state.filter.search.previous() {
                    state.main_view.scroll_to_line(m.buffer_index);
                }
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
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                // Toggle monitoring state
                if self.toolbar_state.auto_scroll {
                    // Currently monitoring, stop it
                    state.stop_monitoring();
                    self.toolbar_state.auto_scroll = false;
                } else {
                    // Currently stopped, resume monitoring
                    state.resume_monitoring();
                    self.toolbar_state.auto_scroll = true;
                }
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.toggle_reverse_order)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                state.main_view.toggle_reverse_order();
                self.toolbar_state.reverse_order = state.main_view.is_reverse_order();
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.goto_top)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                state.main_view.scroll_to_top();
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.goto_bottom)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                state.main_view.scroll_to_bottom();
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.copy)) {
            if let Some(state) = self.tab_manager.get_active_state() {
                let filtered = if state.filter_active {
                    Some(state.filtered_indices.as_slice())
                } else {
                    None
                };
                if let Some(text) = state.main_view.get_selected_text(&state.buffer, filtered) {
                    let lines_count = state.main_view.selected_lines_count();
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
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.toggle_bookmark)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                let filtered = if state.filter_active {
                    Some(state.filtered_indices.as_slice())
                } else {
                    None
                };
                let indices = state.main_view.get_selected_indices(filtered);
                if !indices.is_empty() {
                    let count = state.buffer.toggle_bookmarks(&indices);
                    if count > 1 {
                        self.status_bar.set_message(
                            format!("Toggled bookmarks on {} lines", count),
                            StatusLevel::Info,
                        );
                    }
                    // Mark filter as dirty to update bookmark-only filter
                    if state.filter.filter.bookmarks_only {
                        state.update_filter();
                    }
                }
            }
            // Auto-save bookmarks
            if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                self.tab_manager.save_bookmarks(tab_id, &mut self.bookmarks_store);
            }
            return None;
        }

        if ctx.input_mut(|i| i.consume_shortcut(&self.shortcuts.select_all)) {
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                let total_rows = if state.filter_active {
                    state.filtered_indices.len()
                } else {
                    state.buffer.len()
                };
                if total_rows > 0 {
                    state.main_view.select_all(total_rows);
                }
            }
            return None;
        }

        None
    }

    /// Handle context menu actions
    fn handle_context_menu_action(&mut self, action: ContextMenuAction, ctx: egui::Context) {
        match action {
            ContextMenuAction::Copy => {
                if let Some(state) = self.tab_manager.get_active_state() {
                    let filtered = if state.filter_active {
                        Some(state.filtered_indices.as_slice())
                    } else {
                        None
                    };
                    if let Some(text) = state.main_view.get_selected_text(&state.buffer, filtered) {
                        let lines_count = state.main_view.selected_lines_count();
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
            }
            ContextMenuAction::CopyAll => {
                if let Some(state) = self.tab_manager.get_active_state() {
                    let filtered = if state.filter_active {
                        Some(state.filtered_indices.as_slice())
                    } else {
                        None
                    };
                    let text = Self::get_all_visible_text_from_state(state, filtered);
                    if !text.is_empty() {
                        let lines_count = text.lines().count();
                        ctx.copy_text(text);
                        self.status_bar.set_message(
                            format!("Copied {} lines to clipboard", lines_count),
                            StatusLevel::Info,
                        );
                    }
                }
            }
            ContextMenuAction::ToggleBookmark => {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    let filtered = if state.filter_active {
                        Some(state.filtered_indices.as_slice())
                    } else {
                        None
                    };
                    let indices = state.main_view.get_selected_indices(filtered);
                    if !indices.is_empty() {
                        let count = state.buffer.toggle_bookmarks(&indices);
                        if count > 1 {
                            self.status_bar.set_message(
                                format!("Toggled bookmarks on {} lines", count),
                                StatusLevel::Info,
                            );
                        }
                        // Mark filter as dirty to update bookmark-only filter
                        if state.filter.filter.bookmarks_only {
                            state.update_filter();
                        }
                    }
                }
                // Auto-save bookmarks
                if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                    self.tab_manager.save_bookmarks(tab_id, &mut self.bookmarks_store);
                }
            }
            ContextMenuAction::ClearSelection => {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.clear_selection();
                }
            }
            ContextMenuAction::SelectAll => {
                // Already handled in main_view
            }
            ContextMenuAction::ScrollToTop => {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.scroll_to_top();
                }
            }
            ContextMenuAction::ScrollToBottom => {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.scroll_to_bottom();
                }
            }
        }
    }

    /// Get all visible text for copy from a tab state
    fn get_all_visible_text_from_state(state: &crate::ui::tab_manager::TabState, filtered_indices: Option<&[usize]>) -> String {
        let mut lines = Vec::new();
        if let Some(indices) = filtered_indices {
            for &idx in indices {
                if let Some(entry) = state.buffer.get(idx) {
                    lines.push(entry.content.clone());
                }
            }
        } else {
            for i in 0..state.buffer.len() {
                if let Some(entry) = state.buffer.get(i) {
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
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    // Toggle monitoring state
                    if self.toolbar_state.auto_scroll {
                        // Currently monitoring, stop it
                        state.stop_monitoring();
                        self.toolbar_state.auto_scroll = false;
                    } else {
                        // Currently stopped, resume monitoring
                        state.resume_monitoring();
                        self.toolbar_state.auto_scroll = true;
                    }
                }
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
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.scroll_to_top();
                }
                None
            }
            ToolbarAction::GoToBottom => {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.scroll_to_bottom();
                }
                None
            }
            ToolbarAction::ToggleTheme => {
                self.config.theme.toggle();
                self.toolbar_state.dark_theme = self.config.theme == Theme::Dark;
                self.tab_manager.set_dark_theme(self.toolbar_state.dark_theme);
                self.settings_panel.dark_theme = self.toolbar_state.dark_theme;
                self.global_search_panel
                    .set_dark_theme(self.toolbar_state.dark_theme);
                // Update titlebar theme
                self.title_bar.update_theme_mode(if self.toolbar_state.dark_theme {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                });
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
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.main_view.toggle_reverse_order();
                    self.toolbar_state.reverse_order = state.main_view.is_reverse_order();
                }
                None
            }
            ToolbarAction::ToggleSplitView => {
                self.tab_manager.toggle_split();
                self.toolbar_state.split_view_active = self.tab_manager.is_split();
                None
            }
            ToolbarAction::None => None,
        }
    }
}

impl eframe::App for LoglineApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle file drag-and-drop
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        tracing::info!("File dropped: {:?}", path);

                        // Open the dropped file in a new tab
                        match self.open_file(path.clone(), None) {
                            Ok(_) => {
                                let msg = format!("{}: {}", t::file_opened_success(), path.display());
                                self.status_bar.set_message(
                                    msg,
                                    StatusLevel::Success,
                                );
                            }
                            Err(e) => {
                                let msg = format!("{}: {}", t::file_open_failed(), e);
                                self.status_bar.set_message(
                                    msg,
                                    StatusLevel::Error,
                                );
                            }
                        }

                        // Only handle the first dropped file
                        break;
                    }
                }
            }
        });

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
        if ctx.input(|i| i.viewport().close_requested()) && !self.should_quit
            && self.tray_manager.is_some() {
                // Prevent the window from closing
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                // Hide the window instead
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                tracing::info!("Window minimized to tray");
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

        // Process background messages for all tabs
        self.tab_manager.process_all_reader_messages();
        
        // Check if any tab needs to load more data (lazy loading)
        // This is triggered when user scrolls near the top of the loaded data
        for state in self.tab_manager.states.values_mut() {
            let total_rows = state.buffer.len();
            let visible_range = state.main_view.get_visible_range(total_rows);
            if state.buffer.should_load_more(visible_range.start) {
                state.request_load_more();
            }
        }
        
        // Apply grok parsing incrementally for each tab (only parse visible entries on-demand)
        // This avoids parsing the entire file, making format switching instant
        {
            const MAX_PARSE_PER_FRAME: usize = 100; // Increased since we're only parsing visible area
            
            // Parse entries in all tabs - prioritize visible entries
            for state in self.tab_manager.states.values_mut() {
                // Skip if this tab doesn't have a grok parser
                let Some(ref parser) = state.grok_parser else {
                    continue;
                };
                
                if !parser.has_active_pattern() {
                    continue;
                }
                
                // Only log once when we start parsing (use a reduced frequency)
                static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let last = LAST_LOG.load(std::sync::atomic::Ordering::Relaxed);
                if now > last + 5 {
                    LAST_LOG.store(now, std::sync::atomic::Ordering::Relaxed);
                    tracing::info!("Parsing with grok_parser: pattern={:?}, pre_processor={:?}", 
                        parser.active_pattern_name(), 
                        parser.pre_processor());
                }
                
                let total_len = state.buffer.len();
                if total_len == 0 {
                    continue;
                }
                
                // Get visible range for on-demand parsing
                let visible_range = state.main_view.get_visible_range(total_len);
                let mut parsed_count = 0;
                
                // Priority 1: Parse visible entries first
                for i in visible_range.clone() {
                    if parsed_count >= MAX_PARSE_PER_FRAME {
                        break;
                    }
                    
                    if let Some(entry) = state.buffer.get_mut(i) {
                        if entry.grok_fields.is_none() {
                            // Use parser.parse_with_format() to support mixed patterns and formatting
                            if let Some((fields, formatted)) =
                                parser.parse_with_format(&entry.content)
                            {
                                if !fields.is_empty() {
                                    entry.set_grok_fields(fields.fields);
                                    if let Some((plain_text, segments)) = formatted {
                                        entry.formatted_content = Some(plain_text);
                                        entry.formatted_segments = Some(segments);
                                    }
                                }
                            }
                            parsed_count += 1;
                        }
                    }
                }
                
                // Note: We no longer track grok_parse_progress for sequential parsing
                // since we now parse on-demand based on visible area
                
                if parsed_count > 0 {
                    tracing::trace!("Parsed {} visible entries with grok pattern this frame", parsed_count);
                }
            }
        }

        // Process remote server events
        self.process_server_events();

        // Rate-limit filter updates
        if self.last_update.elapsed() > Duration::from_millis(16) {
            self.tab_manager.update_pending_filters();
            self.last_update = Instant::now();
        }

        // Handle shortcuts
        let mut action = self.handle_shortcuts(ctx);

        // === Application Title Bar (macOS) ===
        #[cfg(target_os = "macos")]
        {
            // Show app title bar with search on macOS
            let (search_query, should_toggle_maximize) = self.app_titlebar.show(ctx);
            
            if let Some(query) = search_query {
                // Trigger search in active tab
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    state.filter.search.set_query(query);
                    state.filter.search.search(&state.buffer);
                    state.update_filter();
                    // Open search bar to show results
                    self.search_bar.visible = true;
                    self.toolbar_state.search_visible = true;
                }
            }
            
            // Handle double-click to maximize/restore window
            if should_toggle_maximize {
                let is_maximized = ctx.input(|i| {
                    i.viewport().maximized.unwrap_or(false)
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            }
        }

        // === Custom Titlebar (Windows/Linux only) ===
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            // Render custom titlebar (handles window controls and drag)
            self.title_bar.show(ctx);
            
            // Render resize handles for window resizing when decorations are disabled
            render_resize_handles(ctx);
        }

        // Define toolbar colors based on theme
        let is_dark = ctx.style().visuals.dark_mode;
        let toolbar_bg = if is_dark {
            egui::Color32::from_rgb(37, 37, 38)
        } else {
            egui::Color32::from_rgb(243, 243, 243)
        };
        let border_color = if is_dark {
            egui::Color32::from_rgb(60, 60, 64)
        } else {
            egui::Color32::from_rgb(220, 220, 225)
        };

        // Top panel with toolbar (including level filters)
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(toolbar_bg)
                    .inner_margin(egui::Margin::symmetric(0, 6))
                    .stroke(egui::Stroke::new(1.0, border_color)),
            )
            .show(ctx, |ui| {
            let filter_config = self.tab_manager.get_active_state_mut()
                .map(|state| &mut state.filter.filter);
            let toolbar_action = Toolbar::show(ui, &mut self.toolbar_state, filter_config);
            if let Some(a) = self.handle_toolbar_action(toolbar_action) {
                action = Some(a);
            }
            
            // Update filter if changed
            if let Some(state) = self.tab_manager.get_active_state_mut() {
                state.update_filter();
            }
        });

        // Search bar panel
        if self.search_bar.visible {
            egui::TopBottomPanel::top("search").show(ctx, |ui| {
                // Get search engine from active tab if available
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    let search_action = self.search_bar.show(ui, &mut state.filter.search);
                    // Handle search action after getting the state again
                    match search_action {
                        SearchBarAction::SearchChanged => {
                            state.filter.search.search(&state.buffer);
                            state.update_filter();
                        }
                        SearchBarAction::FindNext => {
                            if let Some(m) = state.filter.search.next() {
                                state.main_view.scroll_to_line(m.buffer_index);
                            }
                        }
                        SearchBarAction::FindPrev => {
                            if let Some(m) = state.filter.search.previous() {
                                state.main_view.scroll_to_line(m.buffer_index);
                            }
                        }
                        SearchBarAction::Close => {
                            self.search_bar.close();
                            self.toolbar_state.search_visible = false;
                        }
                        SearchBarAction::None => {}
                    }
                }
            });
        }

        // Filter panel (only shown when expanded for advanced filters)
        if self.filter_panel.is_expanded() {
            egui::TopBottomPanel::top("filter").show(ctx, |ui| {
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    if self.filter_panel.show(ui, &mut state.filter.filter) {
                        state.update_filter();
                    }
                }
            });
        }

        // Bottom status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            // Get info from active tab
            let (current_file, buffer_ref, reader_ref, auto_scroll, filtered_count, selected_count) = 
                if let Some(state) = self.tab_manager.get_active_state() {
                    (
                        Some(state.path.as_path()),
                        Some(&state.buffer),
                        state.reader.as_ref(),
                        state.main_view.is_auto_scroll(),
                        if state.filter_active { Some(state.filtered_indices.len()) } else { None },
                        state.main_view.selected_lines_count(),
                    )
                } else {
                    (None, None, None, true, None, 0)
                };

            // Build grok pattern info for status bar (from current tab's grok parser)
            let grok_info = {
                use crate::ui::status_bar::GrokPatternInfo;
                use crate::grok_parser::BuiltinPattern;
                
                let mut info = GrokPatternInfo::default();
                
                // Get current tab's grok state
                if let Some(state) = self.tab_manager.get_active_state() {
                    if let Some(ref parser) = state.grok_parser {
                        info.enabled = true;
                        info.current_pattern_name = parser.active_pattern_name().map(|s| s.to_string());
                    } else {
                        info.enabled = false;
                        info.current_pattern_name = None;
                    }
                }
                
                // Available patterns come from global config
                info.builtin_patterns = BuiltinPattern::all()
                    .iter()
                    .map(|p| (*p, p.display_name()))
                    .collect();
                info.custom_pattern_names = self.config.grok.custom_patterns
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();
                info
            };
            
            if let Some(buffer) = buffer_ref {
                if let Some(action) = self.status_bar.show(
                    ui,
                    current_file,
                    buffer,
                    reader_ref,
                    auto_scroll,
                    filtered_count,
                    selected_count,
                    Some(&grok_info),
                ) {
                    match action {
                        crate::ui::status_bar::StatusBarAction::ChangeEncoding(encoding) => {
                            self.change_encoding(encoding);
                        }
                        crate::ui::status_bar::StatusBarAction::ChangeGrokPattern(selection) => {
                            self.change_grok_pattern(selection);
                        }
                    }
                }
            } else {
                // Show empty status bar when no tab is open
                ui.horizontal(|ui| {
                    ui.label(t::no_open_tabs());
                });
            }
        });

        // === Tab Bar Panel ===
        if !self.tab_manager.is_empty() {
            egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Tab bar takes full width
                    let tab_action = self.tab_manager.tab_bar.show(ui);
                    match tab_action {
                        TabBarAction::SelectTab(id) => {
                            self.tab_manager.tab_bar.active_tab = Some(id);
                            // If in split mode, also update the active pane's tab
                            if self.tab_manager.is_split() {
                                self.tab_manager.split_view.set_active_tab(id);
                            }
                            // Update toolbar state from the newly selected tab
                            if let Some(state) = self.tab_manager.states.get(&id) {
                                self.toolbar_state.auto_scroll = state.main_view.virtual_scroll.state.auto_scroll;
                                self.toolbar_state.reverse_order = state.main_view.virtual_scroll.state.reverse_order;
                            }
                        }
                        TabBarAction::CloseTab(id) => {
                            // Handle split view tab close
                            self.tab_manager.split_view.handle_tab_close(id);
                            self.close_tab(id);
                            self.toolbar_state.split_view_active = self.tab_manager.is_split();
                        }
                        TabBarAction::CloseOtherTabs(keep_id) => {
                            self.tab_manager.handle_action(TabBarAction::CloseOtherTabs(keep_id), &mut self.bookmarks_store);
                            self.tab_manager.sync_split_with_tab_bar();
                            self.toolbar_state.split_view_active = self.tab_manager.is_split();
                        }
                        TabBarAction::CloseTabsToRight(id) => {
                            self.tab_manager.handle_action(TabBarAction::CloseTabsToRight(id), &mut self.bookmarks_store);
                            self.tab_manager.sync_split_with_tab_bar();
                            self.toolbar_state.split_view_active = self.tab_manager.is_split();
                        }
                        TabBarAction::CloseAllTabs => {
                            self.tab_manager.handle_action(TabBarAction::CloseAllTabs, &mut self.bookmarks_store);
                            self.toolbar_state.split_view_active = false;
                        }
                        TabBarAction::ReorderTabs(from, to) => {
                            self.tab_manager.tab_bar.reorder(from, to);
                        }
                        TabBarAction::OpenInSplit(id) => {
                            self.tab_manager.enable_split(id);
                            self.toolbar_state.split_view_active = true;
                        }
                        TabBarAction::None => {}
                    }
                });
            });
        }

        // === New: Activity Bar (leftmost narrow panel) ===
        egui::SidePanel::left("activity_bar")
            .exact_width(48.0)
            .resizable(false)
            .show(ctx, |ui| {
                // Update server status in activity bar
                self.activity_bar.server_running = self.remote_server.is_running();
                self.activity_bar.server_port = self.settings_panel.port();
                self.activity_bar.sidebar_visible = self.sidebar_visible;
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
                        self.activity_bar.sidebar_visible = true;
                    }
                    ActivityBarAction::TogglePanel => {
                        self.sidebar_visible = !self.sidebar_visible;
                        self.activity_bar.sidebar_visible = self.sidebar_visible;
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
                                    let msg = format!("{}: {}", t::server_started(), port);
                                    self.status_bar.set_message(
                                        msg,
                                        StatusLevel::Success,
                                    );
                                }
                                Err(e) => {
                                    let msg = format!("{}: {}", t::server_start_failed(), e);
                                    self.status_bar.set_message(
                                        msg,
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
                                ExplorerAction::OpenLocalFile(path) => {
                                    if let Err(e) = self.open_file(path.clone(), None) {
                                        self.status_bar.set_message(
                                            format!("{}: {}", t::file_open_failed(), e),
                                            StatusLevel::Error,
                                        );
                                    }
                                }
                                ExplorerAction::OpenRemoteStream(stream) => {
                                    if let Err(e) = self.open_remote_stream(stream.project_name.clone(), stream.cache_path.clone()) {
                                        self.status_bar.set_message(
                                            format!("{}: {}", t::remote_stream_failed(), e),
                                            StatusLevel::Error,
                                        );
                                    }
                                }
                                ExplorerAction::OpenFileDialog => {
                                    action = Some(AppAction::OpenFileDialog);
                                }
                                ExplorerAction::OpenInSplit(path) => {
                                    if let Err(e) = self.open_file_in_split(path.clone()) {
                                        self.status_bar.set_message(
                                            format!("{}: {}", t::file_open_in_split_failed(), e),
                                            StatusLevel::Error,
                                        );
                                    }
                                }
                                ExplorerAction::CopyAbsolutePath(path) => {
                                    let abs_path = path.display().to_string();
                                    ui.ctx().copy_text(abs_path.clone());
                                    self.status_bar.set_message(
                                        format!("{}: {}", t::absolute_path_copied(), abs_path),
                                        StatusLevel::Info,
                                    );
                                }
                                ExplorerAction::CopyRelativePath(path) => {
                                    let rel_path = if let Ok(current_dir) = std::env::current_dir() {
                                        path.strip_prefix(&current_dir)
                                            .unwrap_or(&path)
                                            .display()
                                            .to_string()
                                    } else {
                                        path.display().to_string()
                                    };
                                    ui.ctx().copy_text(rel_path.clone());
                                    self.status_bar.set_message(
                                        format!("{}: {}", t::relative_path_copied(), rel_path),
                                        StatusLevel::Info,
                                    );
                                }
                                ExplorerAction::CopyFilename(path) => {
                                    let filename = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path.display().to_string());
                                    ui.ctx().copy_text(filename.clone());
                                    self.status_bar.set_message(
                                        format!("{}: {}", t::filename_copied(), filename),
                                        StatusLevel::Info,
                                    );
                                }
                                ExplorerAction::RevealInFinder(path) => {
                                    #[cfg(target_os = "macos")]
                                    {
                                        if let Err(e) = std::process::Command::new("open")
                                            .arg("-R")
                                            .arg(&path)
                                            .spawn()
                                        {
                                            self.status_bar.set_message(
                                                format!("{}: {}", t::finder_open_failed(), e),
                                                StatusLevel::Error,
                                            );
                                        } else {
                                            self.status_bar.set_message(
                                                "已在访达中显示".to_string(),
                                                StatusLevel::Info,
                                            );
                                        }
                                    }
                                    #[cfg(target_os = "windows")]
                                    {
                                        if let Err(e) = std::process::Command::new("explorer")
                                            .arg("/select,")
                                            .arg(&path)
                                            .spawn()
                                        {
                                            self.status_bar.set_message(
                                                format!("{}: {}", t::file_manager_open_failed(), e),
                                                StatusLevel::Error,
                                            );
                                        } else {
                                            self.status_bar.set_message(
                                                "已在资源管理器中显示".to_string(),
                                                StatusLevel::Info,
                                            );
                                        }
                                    }
                                    #[cfg(target_os = "linux")]
                                    {
                                        // Try xdg-open on Linux
                                        if let Some(parent) = path.parent() {
                                            if let Err(e) = std::process::Command::new("xdg-open")
                                                .arg(parent)
                                                .spawn()
                                            {
                                                self.status_bar.set_message(
                                                    format!("{}: {}", t::file_manager_open_failed(), e),
                                                    StatusLevel::Error,
                                                );
                                            } else {
                                                self.status_bar.set_message(
                                                    "已在文件管理器中显示".to_string(),
                                                    StatusLevel::Info,
                                                );
                                            }
                                        }
                                    }
                                }
                                ExplorerAction::RemoveFromRecent(path) => {
                                    self.explorer_panel.local_files.retain(|p| p != &path);
                                    self.config.recent_files.retain(|p| p != &path);
                                    if let Err(e) = self.config.save() {
                                        self.status_bar.set_message(
                                            format!("{}: {}", t::config_save_failed(), e),
                                            StatusLevel::Error,
                                        );
                                    } else {
                                        self.status_bar.set_message(
                                            "已从最近文件中移除".to_string(),
                                            StatusLevel::Info,
                                        );
                                    }
                                }
                                ExplorerAction::ClearRecentFiles => {
                                    self.explorer_panel.local_files.clear();
                                    self.config.recent_files.clear();
                                    if let Err(e) = self.config.save() {
                                        self.status_bar.set_message(
                                            format!("{}: {}", t::config_save_failed(), e),
                                            StatusLevel::Error,
                                        );
                                    } else {
                                        self.status_bar.set_message(
                                            "已清空最近文件列表".to_string(),
                                            StatusLevel::Info,
                                        );
                                    }
                                }
                                ExplorerAction::None => {}
                            }
                        }
                        ActivityView::Search => {
                            // Global search view
                            if let Some(state) = self.tab_manager.get_active_state() {
                                match self.global_search_panel.show(ui, &state.buffer) {
                                    GlobalSearchAction::JumpToLine(buffer_index) => {
                                        // Jump to the line in main view (need mut access)
                                        if let Some(state) = self.tab_manager.get_active_state_mut() {
                                            state.main_view.scroll_to_line(buffer_index);
                                            state.main_view.set_selection(buffer_index, buffer_index);
                                        }
                                    }
                                    GlobalSearchAction::None => {}
                                }
                            }
                        }
                        ActivityView::Filters => {
                            // Advanced filters view
                            if let Some(state) = self.tab_manager.get_active_state_mut() {
                                if self.advanced_filters_panel.show(ui, &mut state.filter.filter) {
                                    // Filter changed, update the view
                                    state.filter.mark_dirty();
                                    state.update_filter();
                                }
                            }
                        }
                        ActivityView::Grok => {
                            // Update grok panel with current file info
                            if let Some(state) = self.tab_manager.get_active_state() {
                                self.grok_panel.current_file_path = Some(state.path.clone());
                                self.grok_panel.use_file_specific = state.grok_config.is_some();
                            } else {
                                self.grok_panel.current_file_path = None;
                            }

                            // Grok parser panel
                            match self.grok_panel.show(ui, &mut self.grok_parser) {
                                GrokPanelAction::PatternChanged => {
                                    // Pattern changed, reset grok parse progress for all tabs
                                    for state in self.tab_manager.states.values_mut() {
                                        state.grok_parse_progress = 0;
                                        // Clear existing grok fields
                                        for entry in state.buffer.iter_mut() {
                                            entry.clear_grok_fields();
                                        }
                                    }
                                    // Save config
                                    self.grok_panel.save_to_config(&mut self.config.grok);
                                    self.config.grok.custom_patterns = self.grok_parser.export_custom_patterns();
                                    if let Err(e) = self.config.save() {
                                        tracing::error!("Failed to save config: {}", e);
                                    }
                                }
                                GrokPanelAction::ConfigChanged => {
                                    // Config changed, save
                                    self.grok_panel.save_to_config(&mut self.config.grok);
                                    self.config.grok.custom_patterns = self.grok_parser.export_custom_patterns();
                                    if let Err(e) = self.config.save() {
                                        tracing::error!("Failed to save config: {}", e);
                                    }
                                }
                                GrokPanelAction::FilePatternChanged { path, config } => {
                                    tracing::info!("FilePatternChanged received for path: {:?}", path);
                                    tracing::info!("FilePatternChanged config: {:?}", config);
                                    
                                    // Save per-file grok config
                                    self.config.set_file_grok_config(path.clone(), config.clone());
                                    
                                    // Update the tab state with the parser that was configured in grok_panel
                                    let state_found = self.tab_manager.get_state_by_path_mut(&path).is_some();
                                    tracing::info!("Tab state found for path: {}", state_found);
                                    
                                    if let Some(state) = self.tab_manager.get_state_by_path_mut(&path) {
                                        // Create a new parser for this tab with the same configuration
                                        if let Some(ref config) = config {
                                            tracing::info!("Config enabled: {}, pattern_type: {}", config.enabled, config.pattern_type);
                                            if config.enabled {
                                                use crate::grok_parser::GrokParser;
                                                
                                                let mut tab_parser = GrokParser::new();
                                                // Import custom patterns from global config
                                                tab_parser.import_custom_patterns(self.config.grok.custom_patterns.clone());
                                                for (name, pat) in &self.config.grok.custom_definitions {
                                                    tab_parser.add_pattern_definition(name, pat);
                                                }
                                                
                                                // Set the pattern based on config type
                                                let pattern_set = match config.pattern_type.as_str() {
                                                    "inline" => {
                                                        tracing::info!("Processing inline pattern");
                                                        if let Some(ref inline) = config.inline_pattern {
                                                            tracing::info!("Inline pattern found: name={}, pre_processor={:?}", inline.name, inline.pre_processor);
                                                            let template = if inline.display_template.is_empty() {
                                                                None
                                                            } else {
                                                                Some(inline.display_template.as_str())
                                                            };
                                                            if tab_parser.set_custom_pattern_with_template(&inline.name, &inline.pattern, template).is_ok() {
                                                                // Set pre-processor from inline pattern or config
                                                                let pre_processor = inline.pre_processor.clone();
                                                                tracing::info!("Setting pre_processor for tab: {:?}", pre_processor);
                                                                tab_parser.set_pre_processor(pre_processor);
                                                                true
                                                            } else {
                                                                tracing::error!("Failed to set custom pattern");
                                                                false
                                                            }
                                                        } else {
                                                            tracing::warn!("No inline pattern in config");
                                                            false
                                                        }
                                                    }
                                                    other => {
                                                        tracing::info!("Pattern type '{}' not handled in FilePatternChanged", other);
                                                        false
                                                    }
                                                };
                                                
                                                if pattern_set {
                                                    tracing::info!("Tab parser configured successfully for path: {:?}", path);
                                                    state.grok_parser = Some(tab_parser);
                                                } else {
                                                    tracing::warn!("Pattern was not set for tab");
                                                }
                                            }
                                        }
                                        
                                        state.grok_config = config;
                                        state.grok_parse_progress = 0;
                                        // Clear existing grok fields to reparse
                                        for entry in state.buffer.iter_mut() {
                                            entry.clear_grok_fields();
                                        }
                                    }
                                    
                                    if let Err(e) = self.config.save() {
                                        tracing::error!("Failed to save config: {}", e);
                                    }
                                }
                                GrokPanelAction::RequestSampleLines => {
                                    // Get sample lines from current tab
                                    if let Some(state) = self.tab_manager.get_active_state() {
                                        let sample_lines: Vec<String> = state.buffer
                                            .iter()
                                            .take(20)
                                            .map(|e| e.content.clone())
                                            .collect();
                                        self.grok_panel.set_sample_lines(sample_lines);
                                    }
                                }
                                GrokPanelAction::None => {}
                            }
                        }
                        ActivityView::Bookmarks => {
                            // Bookmarks view
                            if let Some(state) = self.tab_manager.get_active_state_mut() {
                                match self.bookmarks_panel.show(ui, &state.buffer) {
                                    BookmarkAction::JumpToLine(line_number) => {
                                        // Find the buffer index for this line number
                                        let index = state
                                            .buffer
                                            .iter()
                                            .position(|e| e.line_number == line_number);

                                        if let Some(index) = index {
                                            state.main_view.scroll_to_line(index);
                                            state.main_view.set_selection(index, index);
                                        }
                                    }
                                    BookmarkAction::RemoveBookmark(line_number) => {
                                        // Find and remove bookmark
                                        let index = state
                                            .buffer
                                            .iter()
                                            .position(|e| e.line_number == line_number);

                                        if let Some(index) = index {
                                            state.buffer.toggle_bookmark(index);
                                            if state.filter.filter.bookmarks_only {
                                                state.filter.mark_dirty();
                                                state.update_filter();
                                            }
                                        }
                                        // Auto-save bookmarks
                                        if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                                            self.tab_manager.save_bookmarks(tab_id, &mut self.bookmarks_store);
                                        }
                                    }
                                    BookmarkAction::RemoveSegment(indices) => {
                                        // Remove all bookmarks in the segment
                                        let count = state.buffer.toggle_bookmarks(&indices);
                                        if count > 0
                                            && state.filter.filter.bookmarks_only {
                                                state.filter.mark_dirty();
                                                state.update_filter();
                                            }
                                        // Auto-save bookmarks
                                        if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                                            self.tab_manager.save_bookmarks(tab_id, &mut self.bookmarks_store);
                                        }
                                    }
                                    BookmarkAction::ClearAll => {
                                        // Clear all bookmarks
                                        let all_indices: Vec<usize> = state
                                            .buffer
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, e)| e.bookmarked)
                                            .map(|(i, _)| i)
                                            .collect();
                                        if !all_indices.is_empty() {
                                            state.buffer.toggle_bookmarks(&all_indices);
                                            if state.filter.filter.bookmarks_only {
                                                state.filter.mark_dirty();
                                                state.update_filter();
                                            }
                                            self.status_bar
                                                .set_message("所有书签已清除", StatusLevel::Info);
                                        }
                                        // Auto-save bookmarks
                                        if let Some(tab_id) = self.tab_manager.tab_bar.active_tab {
                                            self.tab_manager.save_bookmarks(tab_id, &mut self.bookmarks_store);
                                        }
                                    }
                                    BookmarkAction::None => {}
                                }
                            }
                        }
                        ActivityView::Settings => {
                            match self.settings_panel.show(ui) {
                                SettingsAction::ThemeChanged(dark) => {
                                    self.config.theme =
                                        if dark { Theme::Dark } else { Theme::Light };
                                    self.toolbar_state.dark_theme = dark;
                                    self.tab_manager.set_dark_theme(dark);
                                    self.global_search_panel.set_dark_theme(dark);
                                    // Update titlebar theme
                                    self.title_bar.update_theme_mode(if dark {
                                        ThemeMode::Dark
                                    } else {
                                        ThemeMode::Light
                                    });
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
            // Check if split view is active
            if self.tab_manager.is_split() {
                // Split view mode - show two panes
                let (left_rect, right_rect_opt, split_action) = self.tab_manager.split_view.show(ui);
                
                // Handle split action
                if split_action != crate::ui::split_view::SplitAction::None {
                    self.tab_manager.handle_split_action(split_action);
                }

                // Detect which pane the mouse is in for active pane switching
                if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                    if ui.ctx().input(|i| i.pointer.any_click()) {
                        if left_rect.contains(pointer_pos) {
                            if self.tab_manager.split_view.active_pane() != crate::ui::split_view::SplitPane::Left {
                                self.tab_manager.split_view.set_active_pane(crate::ui::split_view::SplitPane::Left);
                                if let Some(left_id) = self.tab_manager.split_view.get_pane_tab(crate::ui::split_view::SplitPane::Left) {
                                    self.tab_manager.tab_bar.active_tab = Some(left_id);
                                    // Update toolbar state from the newly selected tab
                                    if let Some(state) = self.tab_manager.states.get(&left_id) {
                                        self.toolbar_state.auto_scroll = state.main_view.virtual_scroll.state.auto_scroll;
                                        self.toolbar_state.reverse_order = state.main_view.virtual_scroll.state.reverse_order;
                                    }
                                }
                            }
                        } else if let Some(right_rect) = right_rect_opt {
                            if right_rect.contains(pointer_pos)
                                && self.tab_manager.split_view.active_pane() != crate::ui::split_view::SplitPane::Right {
                                    self.tab_manager.split_view.set_active_pane(crate::ui::split_view::SplitPane::Right);
                                    if let Some(right_id) = self.tab_manager.split_view.get_pane_tab(crate::ui::split_view::SplitPane::Right) {
                                        self.tab_manager.tab_bar.active_tab = Some(right_id);
                                        // Update toolbar state from the newly selected tab
                                        if let Some(state) = self.tab_manager.states.get(&right_id) {
                                            self.toolbar_state.auto_scroll = state.main_view.virtual_scroll.state.auto_scroll;
                                            self.toolbar_state.reverse_order = state.main_view.virtual_scroll.state.reverse_order;
                                        }
                                    }
                                }
                        }
                    }
                }

                // Collect context actions to handle after rendering
                let mut left_context_action: Option<ContextMenuAction> = None;
                let mut right_context_action: Option<ContextMenuAction> = None;

                // Render left pane with unique ID scope
                if let Some(left_id) = self.tab_manager.split_view.get_pane_tab(crate::ui::split_view::SplitPane::Left) {
                    if let Some(state) = self.tab_manager.states.get_mut(&left_id) {
                        let filtered = if state.filter_active {
                            Some(state.filtered_indices.as_slice())
                        } else {
                            None
                        };

                        // Get grok pattern from this tab's parser
                        let grok_pattern = if self.display_config.show_grok_fields {
                            state.grok_parser.as_ref().and_then(|p| p.active_pattern())
                        } else {
                            None
                        };

                        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect).id_salt("left_pane"), |ui| {
                            let (_, context_action) = state.main_view.show(
                                ui,
                                &state.buffer,
                                filtered,
                                &state.filter.search,
                                &self.display_config,
                                grok_pattern.as_ref(),
                            );
                            left_context_action = context_action;
                        });
                    }
                }

                // Render right pane with unique ID scope
                if let Some(right_rect) = right_rect_opt {
                    if let Some(right_id) = self.tab_manager.split_view.get_pane_tab(crate::ui::split_view::SplitPane::Right) {
                        if let Some(state) = self.tab_manager.states.get_mut(&right_id) {
                            let filtered = if state.filter_active {
                                Some(state.filtered_indices.as_slice())
                            } else {
                                None
                            };

                            // Get grok pattern from this tab's parser
                            let grok_pattern = if self.display_config.show_grok_fields {
                                state.grok_parser.as_ref().and_then(|p| p.active_pattern())
                            } else {
                                None
                            };

                            ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect).id_salt("right_pane"), |ui| {
                                let (_, context_action) = state.main_view.show(
                                    ui,
                                    &state.buffer,
                                    filtered,
                                    &state.filter.search,
                                    &self.display_config,
                                    grok_pattern.as_ref(),
                                );
                                right_context_action = context_action;
                            });
                        }
                    }
                }

                // Handle context menu actions after rendering
                if let Some(ctx_action) = left_context_action {
                    self.handle_context_menu_action(ctx_action, ctx.clone());
                }
                if let Some(ctx_action) = right_context_action {
                    self.handle_context_menu_action(ctx_action, ctx.clone());
                }
            } else {
                // Single pane mode
                if let Some(state) = self.tab_manager.get_active_state_mut() {
                    let filtered = if state.filter_active {
                        Some(state.filtered_indices.as_slice())
                    } else {
                        None
                    };

                    // Get grok pattern from this tab's parser
                    let grok_pattern = if self.display_config.show_grok_fields {
                        state.grok_parser.as_ref().and_then(|p| p.active_pattern())
                    } else {
                        None
                    };

                    let (_, context_action) = state.main_view.show(
                        ui,
                        &state.buffer,
                        filtered,
                        &state.filter.search,
                        &self.display_config,
                        grok_pattern.as_ref(),
                    );

                    // Handle context menu actions
                    if let Some(ctx_action) = context_action {
                        self.handle_context_menu_action(ctx_action, ctx.clone());
                    }
                } else {
                    // No tab open - show welcome message with hints
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                
                                // Main title
                                ui.heading(RichText::new(t::welcome_title()).size(32.0).strong());
                                ui.add_space(8.0);
                                ui.label(RichText::new(t::no_open_tabs()).size(15.0).weak());
                                ui.add_space(30.0);
                                
                                // Open file button
                                let button = egui::Button::new(RichText::new(t::open_file_button()).size(16.0))
                                    .min_size(egui::vec2(180.0, 40.0));
                                if ui.add(button).clicked() {
                                    self.file_picker_dialog.show_dialog();
                                }
                                
                                ui.add_space(40.0);
                                
                                // Hints section - use horizontal layout with equal height panels
                                ui.horizontal_top(|ui| {
                                    // Calculate available width for centering
                                    let available_width = ui.available_width();
                                    let panel_width = 350.0;
                                    let spacing = 20.0;
                                    let total_content_width = panel_width * 2.0 + spacing;
                                    let left_margin = ((available_width - total_content_width) / 2.0).max(20.0);
                                    
                                    ui.add_space(left_margin);
                                    
                                    // Keyboard shortcuts panel
                                    ui.vertical(|ui| {
                                        ui.group(|ui| {
                                            ui.set_width(panel_width);
                                            ui.vertical(|ui| {
                                                ui.add_space(12.0);
                                                ui.label(RichText::new(t::keyboard_shortcuts_title()).strong().size(16.0));
                                                ui.add_space(16.0);
                                                
                                                // Shortcuts list with consistent spacing
                                                let shortcuts = [
                                                    t::shortcut_open_file(),
                                                    t::shortcut_find(),
                                                    t::shortcut_goto_line(),
                                                    t::shortcut_reload(),
                                                    t::shortcut_clear(),
                                                    t::shortcut_bookmark(),
                                                    t::shortcut_auto_scroll(),
                                                ];
                                                
                                                for shortcut in shortcuts {
                                                    ui.label(RichText::new(shortcut).size(14.0));
                                                    ui.add_space(8.0);
                                                }
                                                
                                                ui.add_space(4.0);
                                            });
                                        });
                                    });
                                    
                                    ui.add_space(spacing);
                                    
                                    // Agent usage panel
                                    ui.vertical(|ui| {
                                        ui.group(|ui| {
                                            ui.set_width(panel_width);
                                            ui.vertical(|ui| {
                                                ui.add_space(12.0);
                                                ui.label(RichText::new(t::agent_usage_title()).strong().size(16.0));
                                                ui.add_space(16.0);
                                                
                                                // Show local IP addresses if server is running
                                                if self.remote_server.is_running() {
                                                    ui.label(RichText::new(t::local_network_addresses()).size(14.0).strong());
                                                    ui.add_space(6.0);
                                                    
                                                    let local_ips = crate::remote_server::get_local_ip_addresses();
                                                    let port = self.remote_server.port();
                                                    
                                                    if local_ips.is_empty() {
                                                        ui.label(RichText::new(format!("  127.0.0.1:{}", port)).size(13.0).monospace());
                                                    } else {
                                                        for ip in local_ips {
                                                            ui.label(RichText::new(format!("  {}:{}", ip, port)).size(13.0).monospace());
                                                        }
                                                    }
                                                    ui.add_space(12.0);
                                                }
                                                
                                                ui.label(RichText::new(t::agent_install_command()).size(14.0));
                                                ui.add_space(6.0);
                                                egui::ScrollArea::horizontal()
                                                    .id_salt("agent_install_scroll")
                                                    .auto_shrink([false, true])
                                                    .show(ui, |ui| {
                                                        ui.code("cargo install --git https://github.com/zibo-chen/logline-agent");
                                                    });
                                                
                                                ui.add_space(12.0);
                                                ui.label(RichText::new(t::agent_basic_usage()).size(14.0));
                                                ui.add_space(6.0);
                                                egui::ScrollArea::horizontal()
                                                    .id_salt("agent_usage_scroll")
                                                    .auto_shrink([false, true])
                                                    .show(ui, |ui| {
                                                        ui.vertical(|ui| {
                                                            ui.code("logline-agent --name \"my-service\" \\");
                                                            ui.code("  --server \"<IP>:12500\" \\");
                                                            ui.code("  --file \"/var/log/app.log\"");
                                                        });
                                                    });
                                                
                                                ui.add_space(12.0);
                                                ui.label(RichText::new(t::agent_server_address()).weak().size(13.0));
                                                ui.add_space(6.0);
                                                ui.label(RichText::new(t::agent_more_info()).weak().size(13.0));
                                                ui.add_space(4.0);
                                            });
                                        });
                                    });
                                });
                                
                                ui.add_space(40.0);
                            });
                        });
                }
            }
        });

        // Go to line dialog
        if self.goto_dialog.open {
            self.show_goto_dialog(ctx);
        }

        // File picker dialog
        match self.file_picker_dialog.show(ctx) {
            FilePickerAction::OpenFile(path, encoding) => {
                if let Err(e) = self.open_file(path.clone(), encoding) {
                    self.status_bar
                        .set_message(format!("{}: {}", t::file_open_failed(), e), StatusLevel::Error);
                }
            }
            FilePickerAction::Cancel => {}
            FilePickerAction::None => {}
        }

        // Handle actions
        if let Some(action) = action {
            match action {
                AppAction::OpenFileDialog => {
                    // Use recent files from config
                    self.file_picker_dialog.set_recent_files(self.config.recent_files.clone());

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
        if self.tab_manager.any_watching() {
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
        tracing::info!("Application exit started");
        
        // Save configuration
        let _ = self.config.save();

        // Stop MCP server first (before shutting down tokio runtime)
        if let Some(mut server) = self.mcp_server.take() {
            tracing::info!("Stopping MCP server");
            server.stop();
        }

        // Stop remote server
        tracing::info!("Stopping remote server");
        self.remote_server.stop();

        // Close all tabs (this will stop file watchers)
        tracing::info!("Closing all tabs");
        self.tab_manager.handle_action(TabBarAction::CloseAllTabs, &mut self.bookmarks_store);

        // Explicitly shutdown tokio runtime with timeout
        if let Some(runtime) = self.tokio_runtime.take() {
            tracing::info!("Shutting down tokio runtime");
            // Drop the runtime which will wait for tasks to complete
            // This is blocking but necessary for clean shutdown
            std::thread::spawn(move || {
                // Set a timeout for runtime shutdown on a separate thread
                drop(runtime);
            });
            // Give it a short time to cleanup, then continue
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        tracing::info!("Application exit completed");
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
                        // Find index for this line number in active tab
                        if let Some(state) = self.tab_manager.get_active_state_mut() {
                            if let Some((idx, _)) = state
                                .buffer
                                .iter()
                                .enumerate()
                                .find(|(_, e)| e.line_number == line)
                            {
                                state.main_view.scroll_to_line(idx);
                            }
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
