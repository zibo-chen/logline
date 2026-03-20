//! Settings Panel - Application configuration UI
//!
//! Provides UI for configuring server port, theme, language, display settings and more.

use crate::config::{CloseButtonBehavior, DisplayConfig};
use crate::i18n::{Language, Translations as t};
use egui::{RichText, Ui};

/// Update-related UI state supplied by the app
pub struct UpdateUiState<'a> {
    pub current_version: &'a str,
    pub latest_version: Option<&'a str>,
    pub status_text: &'a str,
    pub checking: bool,
    pub installing: bool,
    pub can_install: bool,
    pub release_url: Option<&'a str>,
}

/// Settings panel state
pub struct SettingsPanel {
    /// Server port (editable)
    pub server_port: String,
    /// Dark theme enabled
    pub dark_theme: bool,
    /// Enable remote service
    pub enable_remote_service: bool,
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
    /// Close button behavior
    pub close_button_behavior: CloseButtonBehavior,
    /// Whether to auto-check updates on startup
    pub auto_check_updates: bool,
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
            enable_remote_service: false,
            cache_dir,
            language: Language::default(),
            display_config: DisplayConfig::default(),
            mcp_enabled: false,
            mcp_port: "12600".to_string(),
            close_button_behavior: CloseButtonBehavior::Ask,
            auto_check_updates: true,
        }
    }

    /// Render the settings panel
    pub fn show(&mut self, ui: &mut Ui, update_ui: UpdateUiState<'_>) -> SettingsAction {
        let mut action = SettingsAction::None;

        // Set minimum width to prevent panel from shrinking
        ui.set_min_width(200.0);

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

            ui.horizontal(|ui| {
                ui.label(t::letter_spacing());
                if ui
                    .add(egui::Slider::new(
                        &mut self.display_config.letter_spacing,
                        -2.0..=10.0,
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
                .checkbox(
                    &mut self.display_config.show_row_separator,
                    t::show_row_separator(),
                )
                .changed()
            {
                action = SettingsAction::DisplayConfigChanged;
            }

            if ui
                .checkbox(
                    &mut self.display_config.show_grok_fields,
                    t::show_grok_fields(),
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

            // Window settings
            ui.label(RichText::new(format!("🪟 {}", t::window_settings())).strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(format!("{}:", t::close_button_behavior()));
                egui::ComboBox::from_id_salt("close_behavior_selector")
                    .selected_text(match self.close_button_behavior {
                        CloseButtonBehavior::Exit => t::close_behavior_exit(),
                        CloseButtonBehavior::MinimizeToTray => t::close_behavior_minimize(),
                        CloseButtonBehavior::Ask => t::close_behavior_ask(),
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.close_button_behavior,
                                CloseButtonBehavior::Exit,
                                t::close_behavior_exit(),
                            )
                            .clicked()
                        {
                            action = SettingsAction::CloseButtonBehaviorChanged(
                                self.close_button_behavior,
                            );
                        }
                        if ui
                            .selectable_value(
                                &mut self.close_button_behavior,
                                CloseButtonBehavior::MinimizeToTray,
                                t::close_behavior_minimize(),
                            )
                            .clicked()
                        {
                            action = SettingsAction::CloseButtonBehaviorChanged(
                                self.close_button_behavior,
                            );
                        }
                        if ui
                            .selectable_value(
                                &mut self.close_button_behavior,
                                CloseButtonBehavior::Ask,
                                t::close_behavior_ask(),
                            )
                            .clicked()
                        {
                            action = SettingsAction::CloseButtonBehaviorChanged(
                                self.close_button_behavior,
                            );
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
                .checkbox(&mut self.enable_remote_service, t::enable_remote_service())
                .changed()
            {
                action = SettingsAction::RemoteServiceEnabledChanged;
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
            ui.label(RichText::new(format!("✨ {}", t::mcp_service())).strong());
            ui.add_space(4.0);

            if ui
                .checkbox(&mut self.mcp_enabled, t::enable_mcp_service())
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

            // Update settings
            ui.label(RichText::new(format!("⬆ {}", t::updates())).strong());
            ui.add_space(4.0);

            ui.label(format!("{} {}", t::current_version(), update_ui.current_version));

            if let Some(latest_version) = update_ui.latest_version {
                ui.label(format!("{} {}", t::latest_version(), latest_version));
            }

            if ui
                .checkbox(&mut self.auto_check_updates, t::auto_check_updates())
                .changed()
            {
                action = SettingsAction::AutoCheckUpdatesChanged(self.auto_check_updates);
            }

            ui.label(RichText::new(update_ui.status_text).weak());

            ui.horizontal(|ui| {
                let check_label = if update_ui.checking {
                    t::checking_for_updates()
                } else {
                    t::check_for_updates()
                };
                if ui
                    .add_enabled(
                        !update_ui.checking && !update_ui.installing,
                        egui::Button::new(check_label),
                    )
                    .clicked()
                {
                    action = SettingsAction::CheckForUpdates;
                }

                if ui
                    .add_enabled(
                        update_ui.can_install && !update_ui.installing,
                        egui::Button::new(if update_ui.installing {
                            t::downloading_update()
                        } else {
                            t::install_update()
                        }),
                    )
                    .clicked()
                {
                    action = SettingsAction::InstallUpdate;
                }

                if let Some(url) = update_ui.release_url {
                    if ui.button(t::view_release_notes()).clicked() {
                        action = SettingsAction::OpenReleasePage(url.to_string());
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // About section
            ui.label(RichText::new(format!("ℹ {}", t::about())).strong());
            ui.add_space(4.0);

            ui.label(format!("Logline v{}", update_ui.current_version));
            ui.label(RichText::new(t::app_description()).weak());

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.link("GitHub").clicked() {
                    action = SettingsAction::OpenReleasePage(crate::updater::REPOSITORY_URL.to_string());
                }
                ui.label(" | ");
                if ui.link(t::documentation()).clicked() {
                    action = SettingsAction::OpenReleasePage(crate::updater::REPOSITORY_URL.to_string());
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
    RemoteServiceEnabledChanged,
    ThemeChanged(bool),
    BrowseCacheDir,
    LanguageChanged(Language),
    DisplayConfigChanged,
    McpEnabledChanged(bool),
    McpPortChanged,
    CloseButtonBehaviorChanged(CloseButtonBehavior),
    AutoCheckUpdatesChanged(bool),
    CheckForUpdates,
    InstallUpdate,
    OpenReleasePage(String),
}
