//! Settings Panel - Application configuration UI
//!
//! Provides UI for configuring server port, theme, language, display settings and more.

use crate::config::DisplayConfig;
use crate::i18n::{Language, Translations as t};
use egui::{RichText, Ui};

/// Settings panel state
pub struct SettingsPanel {
    /// Server port (editable)
    pub server_port: String,
    /// Dark theme enabled
    pub dark_theme: bool,
    /// Auto-start server
    pub auto_start_server: bool,
    /// Cache directory
    pub cache_dir: String,
    /// Current language
    pub language: Language,
    /// Display configuration
    pub display_config: DisplayConfig,
    /// MCP server enabled
    pub mcp_enabled: bool,
    /// MCP server port
    pub mcp_port: String,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    pub fn new() -> Self {
        let cache_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("logline")
            .join("cache")
            .display()
            .to_string();

        Self {
            server_port: "12500".to_string(),
            dark_theme: true,
            auto_start_server: false,
            cache_dir,
            language: Language::default(),
            display_config: DisplayConfig::default(),
            mcp_enabled: false,
            mcp_port: "12600".to_string(),
        }
    }

    /// Render the settings panel
    pub fn show(&mut self, ui: &mut Ui) -> SettingsAction {
        let mut action = SettingsAction::None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            ui.heading(t::settings_title());
            ui.add_space(16.0);

            // Display settings
            ui.label(RichText::new(format!("📝 {}", t::display())).strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(t::font_size());
                if ui
                    .add(egui::Slider::new(
                        &mut self.display_config.font_size,
                        8.0..=24.0,
                    ))
                    .changed()
                {
                    action = SettingsAction::DisplayConfigChanged;
                }
            });

            ui.horizontal(|ui| {
                ui.label(t::line_height());
                if ui
                    .add(egui::Slider::new(
                        &mut self.display_config.line_height,
                        1.0..=2.0,
                    ))
                    .changed()
                {
                    action = SettingsAction::DisplayConfigChanged;
                }
            });

            if ui
                .checkbox(
                    &mut self.display_config.show_line_numbers,
                    t::show_line_numbers(),
                )
                .changed()
            {
                action = SettingsAction::DisplayConfigChanged;
            }

            if ui
                .checkbox(&mut self.display_config.word_wrap, t::word_wrap())
                .changed()
            {
                action = SettingsAction::DisplayConfigChanged;
            }

            if ui
                .checkbox(
                    &mut self.display_config.show_row_separator,
                    t::show_row_separator(),
                )
                .changed()
            {
                action = SettingsAction::DisplayConfigChanged;
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Appearance settings
            ui.label(RichText::new(format!("🎨 {}", t::appearance())).strong());
            ui.add_space(4.0);

            if ui.checkbox(&mut self.dark_theme, t::dark_theme()).changed() {
                action = SettingsAction::ThemeChanged(self.dark_theme);
            }

            ui.add_space(8.0);

            // Language settings
            ui.horizontal(|ui| {
                ui.label(format!("{}:", t::language()));
                egui::ComboBox::from_id_salt("language_selector")
                    .selected_text(self.language.display_name())
                    .show_ui(ui, |ui| {
                        for lang in Language::all() {
                            if ui
                                .selectable_value(&mut self.language, *lang, lang.display_name())
                                .clicked()
                            {
                                action = SettingsAction::LanguageChanged(self.language);
                            }
                        }
                    });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Server settings
            ui.label(RichText::new(format!("🌐 {}", t::remote_service())).strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(t::listen_port());
                let response = ui.text_edit_singleline(&mut self.server_port);
                if response.changed() {
                    action = SettingsAction::PortChanged;
                }
            });

            ui.add_space(4.0);

            if ui
                .checkbox(&mut self.auto_start_server, t::auto_start_server())
                .changed()
            {
                action = SettingsAction::AutoStartChanged;
            }

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(t::cache_directory());
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.cache_dir)
                        .desired_width(200.0)
                        .interactive(false),
                );
                if ui.button("📂").clicked() {
                    action = SettingsAction::BrowseCacheDir;
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // MCP Server settings
            ui.label(RichText::new(format!("🤖 {}", t::mcp_service())).strong());
            ui.add_space(4.0);

            if ui
                .checkbox(&mut self.mcp_enabled, t::enable_mcp())
                .changed()
            {
                action = SettingsAction::McpEnabledChanged(self.mcp_enabled);
            }

            ui.horizontal(|ui| {
                ui.label(t::mcp_port());
                ui.add_enabled_ui(self.mcp_enabled, |ui| {
                    let response = ui.text_edit_singleline(&mut self.mcp_port);
                    if response.changed() {
                        action = SettingsAction::McpPortChanged;
                    }
                });
            });

            ui.add_space(4.0);
            if self.mcp_enabled {
                let endpoint = format!("http://127.0.0.1:{}/mcp", self.mcp_port);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t::mcp_endpoint()).weak());
                    ui.label(RichText::new(&endpoint).monospace().weak());
                });
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // About section
            ui.label(RichText::new(format!("ℹ {}", t::about())).strong());
            ui.add_space(4.0);

            ui.label("Logline v0.1.0");
            ui.label(RichText::new(t::app_description()).weak());

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.link("GitHub").clicked() {
                    // Open GitHub link
                }
                ui.label(" | ");
                if ui.link(t::documentation()).clicked() {
                    // Open docs link
                }
            });
        });

        action
    }

    /// Get parsed port number
    pub fn port(&self) -> u16 {
        self.server_port.parse().unwrap_or(12500)
    }

    /// Get parsed MCP port number
    pub fn mcp_port_number(&self) -> u16 {
        self.mcp_port.parse().unwrap_or(12600)
    }
}

/// Actions from the settings panel
#[derive(Debug, Clone)]
pub enum SettingsAction {
    None,
    PortChanged,
    AutoStartChanged,
    ThemeChanged(bool),
    BrowseCacheDir,
    LanguageChanged(Language),
    DisplayConfigChanged,
    McpEnabledChanged(bool),
    McpPortChanged,
}
