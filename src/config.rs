//! Application configuration and persistence

use crate::grok_parser::GrokConfig;
use crate::i18n::Language;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Window configuration
    pub window: WindowConfig,
    /// Display configuration
    pub display: DisplayConfig,
    /// Buffer configuration
    pub buffer: BufferConfig,
    /// MCP server configuration
    pub mcp: McpServerConfig,
    /// Remote server configuration
    pub remote_server: RemoteServerConfig,
    /// Grok parser configuration
    pub grok: GrokConfig,
    /// Recent files list
    pub recent_files: Vec<PathBuf>,
    /// Maximum recent files to keep
    pub max_recent_files: usize,
    /// File encoding preferences (file path -> encoding name)
    pub file_encodings: HashMap<String, String>,
    /// Per-file Grok configuration (file path -> grok pattern config)
    pub file_grok_configs: HashMap<String, FileGrokConfig>,
    /// Current theme
    pub theme: Theme,
    /// Application language
    pub language: Language,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            display: DisplayConfig::default(),
            buffer: BufferConfig::default(),
            mcp: McpServerConfig::default(),
            remote_server: RemoteServerConfig::default(),
            grok: GrokConfig::default(),
            recent_files: Vec::new(),
            max_recent_files: 10,
            file_encodings: HashMap::new(),
            file_grok_configs: HashMap::new(),
            theme: Theme::Dark,
            language: Language::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path).context("Failed to read config file")?;

        let config: Self = toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get config file path
    fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("logline");

        Ok(dir.join("config.toml"))
    }

    /// Add a recent file
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);

        // Add to front
        self.recent_files.insert(0, path);

        // Trim to max size
        self.recent_files.truncate(self.max_recent_files);
    }

    /// Remove a recent file
    #[allow(dead_code)]
    pub fn remove_recent_file(&mut self, path: &PathBuf) {
        self.recent_files.retain(|p| p != path);
    }

    /// Clear recent files
    #[allow(dead_code)]
    pub fn clear_recent_files(&mut self) {
        self.recent_files.clear();
    }

    /// Get encoding for a file
    pub fn get_file_encoding(&self, path: &PathBuf) -> Option<&'static encoding_rs::Encoding> {
        let path_str = path.to_string_lossy().to_string();
        self.file_encodings
            .get(&path_str)
            .and_then(|name| encoding_rs::Encoding::for_label(name.as_bytes()))
    }

    /// Set encoding for a file
    pub fn set_file_encoding(
        &mut self,
        path: PathBuf,
        encoding: Option<&'static encoding_rs::Encoding>,
    ) {
        let path_str = path.to_string_lossy().to_string();
        if let Some(enc) = encoding {
            self.file_encodings.insert(path_str, enc.name().to_string());
        } else {
            self.file_encodings.remove(&path_str);
        }
    }

    /// Set grok config for a file
    pub fn set_file_grok_config(&mut self, path: PathBuf, config: Option<FileGrokConfig>) {
        let path_str = path.to_string_lossy().to_string();
        if let Some(cfg) = config {
            self.file_grok_configs.insert(path_str, cfg);
        } else {
            self.file_grok_configs.remove(&path_str);
        }
    }
}

/// Per-file Grok configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGrokConfig {
    /// Whether grok parsing is enabled for this file
    pub enabled: bool,
    /// Pattern type: "builtin" or "custom"
    pub pattern_type: String,
    /// For builtin patterns: the pattern name
    pub builtin_pattern: Option<String>,
    /// For custom patterns: the custom pattern name
    pub custom_pattern_name: Option<String>,
    /// Inline custom pattern (from AI assist, not in global custom patterns list)
    pub inline_pattern: Option<InlineGrokPattern>,
    /// Pre-processor to apply before Grok matching (e.g., extract "log" field from JSON)
    #[serde(default)]
    pub pre_processor: crate::grok_parser::PreProcessor,
}

/// Inline grok pattern (for AI-generated patterns that are file-specific)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineGrokPattern {
    /// Pattern name
    pub name: String,
    /// Grok pattern string
    pub pattern: String,
    /// Display template
    pub display_template: String,
    /// Pre-processor to apply before Grok matching
    #[serde(default)]
    pub pre_processor: crate::grok_parser::PreProcessor,
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    /// Window width
    pub width: f32,
    /// Window height
    pub height: f32,
    /// Window X position (None = centered)
    pub x: Option<f32>,
    /// Window Y position (None = centered)
    pub y: Option<f32>,
    /// Whether window is maximized
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            x: None,
            y: None,
            maximized: false,
        }
    }
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Font size
    pub font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
    /// Extra letter spacing in pixels
    pub letter_spacing: f32,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Line number width in characters
    pub line_number_width: usize,
    /// Show timestamp column
    pub show_timestamp: bool,
    /// Show log level column
    pub show_level: bool,
    /// Tab size (spaces)
    pub tab_size: usize,
    /// Show row separator lines
    pub show_row_separator: bool,
    /// Show grok parsed fields
    pub show_grok_fields: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            font_size: 13.0,
            line_height: 1.4,
            letter_spacing: 0.0,
            show_line_numbers: true,
            line_number_width: 6,
            show_timestamp: false,
            show_level: false,
            tab_size: 4,
            show_row_separator: true,
            show_grok_fields: true,
        }
    }
}

/// Buffer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BufferConfig {
    /// Maximum lines to keep in memory
    pub max_lines: usize,
    /// Auto-trim old entries
    pub auto_trim: bool,
    /// Batch update interval in milliseconds
    pub update_interval_ms: u64,
    /// Maximum batch size for UI updates
    pub max_batch_size: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_lines: 100_000,
            auto_trim: true,
            update_interval_ms: 16, // ~60 FPS
            max_batch_size: 1000,
        }
    }
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerConfig {
    /// Whether MCP server is enabled
    pub enabled: bool,
    /// Port for MCP SSE server
    pub port: u16,
    /// Bind address
    pub bind_address: String,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 12600,
            bind_address: "127.0.0.1".to_string(),
        }
    }
}

/// Remote server configuration for receiving logs from agents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteServerConfig {
    /// Port for remote log server
    pub port: u16,
    /// Whether to auto-start the server on application launch
    pub auto_start: bool,
}

impl Default for RemoteServerConfig {
    fn default() -> Self {
        Self {
            port: 12500,
            auto_start: false,
        }
    }
}

/// Application theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// Toggle between themes
    pub fn toggle(&mut self) {
        *self = match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
    }

    /// Get theme name
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }
}

/// Keyboard shortcuts configuration
#[derive(Debug, Clone)]
pub struct Shortcuts {
    pub open_file: egui::KeyboardShortcut,
    pub reload_file: egui::KeyboardShortcut,
    pub find: egui::KeyboardShortcut,
    pub find_next: egui::KeyboardShortcut,
    pub find_prev: egui::KeyboardShortcut,
    pub clear: egui::KeyboardShortcut,
    pub goto_line: egui::KeyboardShortcut,
    pub toggle_auto_scroll: egui::KeyboardShortcut,
    pub toggle_reverse_order: egui::KeyboardShortcut,
    pub goto_top: egui::KeyboardShortcut,
    pub goto_bottom: egui::KeyboardShortcut,
    pub copy: egui::KeyboardShortcut,
    pub toggle_bookmark: egui::KeyboardShortcut,
    pub select_all: egui::KeyboardShortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        use egui::{Key, KeyboardShortcut, Modifiers};

        Self {
            open_file: KeyboardShortcut::new(Modifiers::COMMAND, Key::O),
            reload_file: KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::R),
            find: KeyboardShortcut::new(Modifiers::COMMAND, Key::F),
            find_next: KeyboardShortcut::new(Modifiers::NONE, Key::F3),
            find_prev: KeyboardShortcut::new(Modifiers::SHIFT, Key::F3),
            clear: KeyboardShortcut::new(Modifiers::COMMAND, Key::L),
            goto_line: KeyboardShortcut::new(Modifiers::COMMAND, Key::G),
            toggle_auto_scroll: KeyboardShortcut::new(Modifiers::NONE, Key::Space),
            toggle_reverse_order: KeyboardShortcut::new(Modifiers::COMMAND, Key::R),
            goto_top: KeyboardShortcut::new(Modifiers::NONE, Key::Home),
            goto_bottom: KeyboardShortcut::new(Modifiers::NONE, Key::End),
            copy: KeyboardShortcut::new(Modifiers::COMMAND, Key::C),
            toggle_bookmark: KeyboardShortcut::new(Modifiers::COMMAND, Key::B),
            select_all: KeyboardShortcut::new(Modifiers::COMMAND, Key::A),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.window.width, config.window.width);
        assert_eq!(deserialized.theme, config.theme);
    }

    #[test]
    fn test_recent_files() {
        let mut config = AppConfig::default();
        config.max_recent_files = 3;

        config.add_recent_file(PathBuf::from("/a"));
        config.add_recent_file(PathBuf::from("/b"));
        config.add_recent_file(PathBuf::from("/c"));
        config.add_recent_file(PathBuf::from("/d"));

        assert_eq!(config.recent_files.len(), 3);
        assert_eq!(config.recent_files[0], PathBuf::from("/d"));
    }

    #[test]
    fn test_theme_toggle() {
        let mut theme = Theme::Dark;
        theme.toggle();
        assert_eq!(theme, Theme::Light);
        theme.toggle();
        assert_eq!(theme, Theme::Dark);
    }
}
