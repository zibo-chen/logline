//! Status bar component

use crate::grok_parser::BuiltinPattern;
use crate::i18n::Translations as t;
use crate::log_buffer::LogBuffer;
use crate::log_reader::LogReader;
use egui::{self, Color32, RichText, Ui};
use std::path::Path;

/// Grok pattern selection in status bar
#[derive(Debug, Clone)]
pub enum GrokPatternSelection {
    /// No pattern (disable grok)
    None,
    /// Built-in pattern
    Builtin(BuiltinPattern),
    /// Custom pattern by name
    Custom(String),
}

/// Action returned by status bar
#[derive(Debug, Clone)]
pub enum StatusBarAction {
    /// Change encoding
    ChangeEncoding(Option<&'static encoding_rs::Encoding>),
    /// Change grok pattern
    ChangeGrokPattern(GrokPatternSelection),
}

/// Grok pattern info for display in status bar
#[derive(Debug, Clone, Default)]
pub struct GrokPatternInfo {
    /// Whether grok is enabled
    pub enabled: bool,
    /// Current pattern name (if any)
    pub current_pattern_name: Option<String>,
    /// Available builtin patterns
    pub builtin_patterns: Vec<(BuiltinPattern, &'static str)>,
    /// Available custom pattern names
    pub custom_pattern_names: Vec<String>,
}

/// Status bar component
pub struct StatusBar {
    /// Current status message
    pub message: Option<StatusMessage>,
    /// File read progress (0.0 - 1.0)
    pub progress: Option<f32>,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            message: None,
            progress: None,
        }
    }

    /// Show the status bar
    pub fn show(
        &mut self,
        ui: &mut Ui,
        file_path: Option<&Path>,
        buffer: &LogBuffer,
        reader: Option<&LogReader>,
        auto_scroll: bool,
        filtered_count: Option<usize>,
        selected_lines: usize,
        grok_info: Option<&GrokPatternInfo>,
    ) -> Option<StatusBarAction> {
        let is_dark = ui.ctx().style().visuals.dark_mode;
        let text_color = if is_dark {
            Color32::LIGHT_GRAY
        } else {
            Color32::DARK_GRAY
        };
        let dim_color = if is_dark {
            Color32::GRAY
        } else {
            Color32::from_rgb(100, 100, 100)
        };

        let mut action = None;

        ui.horizontal(|ui| {
            // File info
            if let Some(path) = file_path {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");

                ui.label(
                    RichText::new(format!("📄 {}", file_name))
                        .color(text_color)
                        .small(),
                );

                if ui
                    .small_button("📋")
                    .on_hover_text(t::copy_path())
                    .clicked()
                {
                    ui.ctx().copy_text(path.display().to_string());
                }

                ui.separator();
            }

            // Line count
            let total_lines = buffer.total_lines();
            let displayed_lines = filtered_count.unwrap_or(buffer.len());

            if filtered_count.is_some() && displayed_lines != total_lines {
                ui.label(
                    RichText::new(format!(
                        "{} / {} {}",
                        displayed_lines,
                        total_lines,
                        t::lines()
                    ))
                    .color(text_color)
                    .small(),
                );
            } else {
                ui.label(
                    RichText::new(format!("{} {}", total_lines, t::lines()))
                        .color(text_color)
                        .small(),
                );
            }

            // Selection count
            if selected_lines > 0 {
                ui.separator();
                ui.label(
                    RichText::new(format!("✓ {} {}", selected_lines, t::selected()))
                        .color(Color32::from_rgb(100, 180, 255))
                        .small(),
                );
            }

            ui.separator();

            // File size
            if let Some(reader) = reader {
                let size = reader.file_size();
                let size_str = format_size(size);
                ui.label(RichText::new(size_str).color(text_color).small());

                ui.separator();

                // Encoding selector
                let current_encoding = reader.encoding_name();
                egui::ComboBox::from_id_salt("encoding_selector")
                    .selected_text(RichText::new(current_encoding).color(dim_color).small())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(current_encoding == "Auto", t::auto())
                            .clicked()
                        {
                            action = Some(StatusBarAction::ChangeEncoding(None));
                        }
                        if ui
                            .selectable_label(current_encoding == "UTF-8", "UTF-8")
                            .clicked()
                        {
                            action =
                                Some(StatusBarAction::ChangeEncoding(Some(encoding_rs::UTF_8)));
                        }
                        if ui
                            .selectable_label(current_encoding == "GBK", "GBK")
                            .clicked()
                        {
                            action = Some(StatusBarAction::ChangeEncoding(Some(encoding_rs::GBK)));
                        }
                        if ui
                            .selectable_label(current_encoding == "GB18030", "GB18030")
                            .clicked()
                        {
                            action =
                                Some(StatusBarAction::ChangeEncoding(Some(encoding_rs::GB18030)));
                        }
                        if ui
                            .selectable_label(current_encoding == "Big5", "Big5")
                            .clicked()
                        {
                            action = Some(StatusBarAction::ChangeEncoding(Some(encoding_rs::BIG5)));
                        }
                        if ui
                            .selectable_label(current_encoding == "Shift_JIS", "Shift_JIS")
                            .clicked()
                        {
                            action = Some(StatusBarAction::ChangeEncoding(Some(
                                encoding_rs::SHIFT_JIS,
                            )));
                        }
                        if ui
                            .selectable_label(current_encoding == "EUC-KR", "EUC-KR")
                            .clicked()
                        {
                            action =
                                Some(StatusBarAction::ChangeEncoding(Some(encoding_rs::EUC_KR)));
                        }
                        if ui
                            .selectable_label(current_encoding == "ISO-8859-1", "ISO-8859-1")
                            .clicked()
                        {
                            action = Some(StatusBarAction::ChangeEncoding(Some(
                                encoding_rs::WINDOWS_1252,
                            )));
                        }
                    });

                ui.separator();
            }

            // Grok pattern selector
            if let Some(grok_info) = grok_info {
                let current_pattern = grok_info
                    .current_pattern_name
                    .as_deref()
                    .unwrap_or(t::grok_none());
                let display_text = if grok_info.enabled && grok_info.current_pattern_name.is_some()
                {
                    format!("⚡ {}", current_pattern)
                } else {
                    format!("⚡ {}", t::grok_none())
                };

                let pattern_color = if grok_info.enabled && grok_info.current_pattern_name.is_some()
                {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    dim_color
                };

                egui::ComboBox::from_id_salt("grok_pattern_selector")
                    .selected_text(RichText::new(&display_text).color(pattern_color).small())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        // None option
                        let is_none =
                            !grok_info.enabled || grok_info.current_pattern_name.is_none();
                        if ui.selectable_label(is_none, t::grok_none()).clicked() {
                            action = Some(StatusBarAction::ChangeGrokPattern(
                                GrokPatternSelection::None,
                            ));
                        }

                        ui.separator();

                        // Built-in patterns
                        if !grok_info.builtin_patterns.is_empty() {
                            ui.label(RichText::new(t::grok_builtin_patterns()).weak().small());
                            for (pattern, display_name) in &grok_info.builtin_patterns {
                                let is_selected = grok_info.enabled
                                    && grok_info.current_pattern_name.as_deref()
                                        == Some(*display_name);
                                if ui.selectable_label(is_selected, *display_name).clicked() {
                                    action = Some(StatusBarAction::ChangeGrokPattern(
                                        GrokPatternSelection::Builtin(*pattern),
                                    ));
                                }
                            }
                        }

                        // Custom patterns
                        if !grok_info.custom_pattern_names.is_empty() {
                            ui.separator();
                            ui.label(RichText::new(t::grok_custom_patterns()).weak().small());
                            for name in &grok_info.custom_pattern_names {
                                let is_selected = grok_info.enabled
                                    && grok_info.current_pattern_name.as_deref()
                                        == Some(name.as_str());
                                if ui.selectable_label(is_selected, name).clicked() {
                                    action = Some(StatusBarAction::ChangeGrokPattern(
                                        GrokPatternSelection::Custom(name.clone()),
                                    ));
                                }
                            }
                        }
                    });

                ui.separator();
            }

            // Auto-scroll indicator
            let scroll_text = if auto_scroll {
                format!("⬇ {}", t::auto())
            } else {
                format!("⏸ {}", t::manual())
            };
            let scroll_color = if auto_scroll {
                Color32::from_rgb(100, 200, 100)
            } else {
                dim_color
            };
            ui.label(RichText::new(scroll_text).color(scroll_color).small());

            // Progress bar
            if let Some(progress) = self.progress {
                ui.separator();
                let progress_bar = egui::ProgressBar::new(progress)
                    .desired_width(100.0)
                    .show_percentage();
                ui.add(progress_bar);
            }

            // Status message (right aligned)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(msg) = &self.message {
                    let color = match msg.level {
                        StatusLevel::Info => text_color,
                        StatusLevel::Success => Color32::from_rgb(100, 200, 100),
                        StatusLevel::Warning => Color32::from_rgb(255, 200, 100),
                        StatusLevel::Error => Color32::from_rgb(255, 100, 100),
                    };

                    ui.label(RichText::new(&msg.text).color(color).small());
                }

                // Memory usage
                let memory = buffer.memory_usage();
                ui.label(
                    RichText::new(format!("{}: {}", t::memory(), format_size(memory as u64)))
                        .color(dim_color)
                        .small(),
                );
            });
        });

        action
    }

    /// Set a status message
    pub fn set_message(&mut self, text: impl Into<String>, level: StatusLevel) {
        self.message = Some(StatusMessage {
            text: text.into(),
            level,
        });
    }

    /// Clear the status message
    #[allow(dead_code)]
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Set progress
    #[allow(dead_code)]
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = Some(progress.clamp(0.0, 1.0));
    }

    /// Clear progress
    #[allow(dead_code)]
    pub fn clear_progress(&mut self) {
        self.progress = None;
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Status message
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
}

/// Status level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Format file size in human readable form
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
