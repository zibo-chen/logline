//! Advanced Filters Panel
//!
//! Provides comprehensive filtering options in the sidebar.

use crate::i18n::Translations as t;
use crate::log_entry::LogLevel;
use crate::search::FilterConfig;
use egui::{self, Color32, RichText, Ui};

/// Pattern type for exclude patterns
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Plain text substring matching
    Text,
    /// Regular expression matching
    Regex,
}

/// Exclude pattern with type
#[derive(Debug, Clone)]
pub struct ExcludePattern {
    /// Pattern string
    pub pattern: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Whether the pattern is enabled
    pub enabled: bool,
}

/// Advanced Filters Panel component
pub struct AdvancedFiltersPanel {
    /// New exclude pattern input
    exclude_input: String,
    /// Selected pattern type for new pattern
    new_pattern_type: PatternType,
    /// Extended exclude patterns (with type and enabled state)
    exclude_patterns: Vec<ExcludePattern>,
}

impl Default for AdvancedFiltersPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedFiltersPanel {
    /// Create a new advanced filters panel
    pub fn new() -> Self {
        Self {
            exclude_input: String::new(),
            new_pattern_type: PatternType::Text,
            exclude_patterns: Vec::new(),
        }
    }

    /// Sync patterns from filter config
    pub fn sync_from_filter(&mut self, filter: &FilterConfig) {
        // Only sync if lengths differ or patterns changed
        if self.exclude_patterns.len() != filter.exclude_patterns.len() {
            self.exclude_patterns = filter
                .exclude_patterns
                .iter()
                .map(|p| ExcludePattern {
                    pattern: p.clone(),
                    pattern_type: PatternType::Text,
                    enabled: true,
                })
                .collect();
        }
    }

    /// Get enabled patterns to sync back to filter config
    pub fn get_enabled_patterns(&self) -> Vec<String> {
        self.exclude_patterns
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.pattern.clone())
            .collect()
    }

    /// Show the advanced filters panel
    pub fn show(&mut self, ui: &mut Ui, filter: &mut FilterConfig) -> bool {
        let mut changed = false;

        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.heading(RichText::new(t::advanced_filters()).strong());
            ui.add_space(8.0);

            // === Log Level Filters ===
            ui.group(|ui| {
                ui.label(RichText::new(t::log_levels()).strong());
                ui.add_space(4.0);

                // Grid layout for level buttons
                egui::Grid::new("level_filters_grid")
                    .num_columns(3)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (i, level) in [
                            LogLevel::Trace,
                            LogLevel::Debug,
                            LogLevel::Info,
                            LogLevel::Warn,
                            LogLevel::Error,
                            LogLevel::Fatal,
                        ]
                        .iter()
                        .enumerate()
                        {
                            let enabled = filter.is_level_enabled(*level);
                            let color = if enabled { level.color() } else { Color32::GRAY };

                            let btn = ui.selectable_label(
                                enabled,
                                RichText::new(level.as_str())
                                    .color(color)
                                    .size(13.0),
                            );

                            if btn.clicked() {
                                filter.toggle_level(*level);
                                changed = true;
                            }

                            if (i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(4.0);

                // Quick filter buttons
                ui.horizontal(|ui| {
                    if ui.button(t::all()).clicked() {
                        filter.enable_all_levels();
                        changed = true;
                    }

                    if ui.button(t::errors()).clicked() {
                        filter.errors_and_warnings_only();
                        changed = true;
                    }

                    if ui.button(t::clear()).clicked() {
                        filter.enable_all_levels();
                        changed = true;
                    }
                });
            });

            ui.add_space(8.0);

            // === Bookmarks Filter ===
            ui.group(|ui| {
                ui.label(RichText::new(t::bookmarks()).strong());
                ui.add_space(4.0);

                if ui
                    .checkbox(&mut filter.bookmarks_only, t::bookmarks_only())
                    .changed()
                {
                    changed = true;
                }
            });

            ui.add_space(8.0);

            // === Exclude Patterns ===
            ui.group(|ui| {
                ui.label(RichText::new(t::exclude_patterns()).strong());
                ui.add_space(4.0);

                // Sync patterns from filter config
                self.sync_from_filter(filter);

                // List existing patterns
                let mut to_remove = None;
                let mut toggled = Vec::new();

                if self.exclude_patterns.is_empty() {
                    ui.label(RichText::new(t::no_exclude_patterns()).weak().italics());
                } else {
                    for (i, pattern) in self.exclude_patterns.iter().enumerate() {
                        ui.horizontal(|ui| {
                            // Enable/disable checkbox
                            let mut enabled = pattern.enabled;
                            if ui.checkbox(&mut enabled, "").changed() {
                                toggled.push(i);
                            }

                            // Pattern type indicator
                            let type_icon = match pattern.pattern_type {
                                PatternType::Text => "T",
                                PatternType::Regex => "R",
                            };
                            let type_color = match pattern.pattern_type {
                                PatternType::Text => Color32::from_rgb(100, 149, 237),
                                PatternType::Regex => Color32::from_rgb(255, 193, 7),
                            };
                            ui.label(
                                RichText::new(type_icon)
                                    .color(type_color)
                                    .monospace()
                                    .strong(),
                            )
                            .on_hover_text(match pattern.pattern_type {
                                PatternType::Text => t::text_pattern(),
                                PatternType::Regex => t::regex_pattern(),
                            });

                            // Pattern text
                            let text_color = if pattern.enabled {
                                ui.visuals().text_color()
                            } else {
                                Color32::GRAY
                            };
                            ui.label(RichText::new(&pattern.pattern).color(text_color));

                            // Delete button
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").clicked() {
                                        to_remove = Some(i);
                                    }
                                },
                            );
                        });
                    }
                }

                // Handle toggle
                for idx in toggled {
                    if let Some(pattern) = self.exclude_patterns.get_mut(idx) {
                        pattern.enabled = !pattern.enabled;
                        changed = true;
                    }
                }

                // Handle removal
                if let Some(idx) = to_remove {
                    self.exclude_patterns.remove(idx);
                    changed = true;
                }

                ui.add_space(8.0);

                // Add new pattern section
                ui.label(RichText::new(t::add_pattern()).size(12.0));

                // Pattern type selector
                ui.horizontal(|ui| {
                    ui.label(t::pattern_type());
                    ui.radio_value(
                        &mut self.new_pattern_type,
                        PatternType::Text,
                        t::text(),
                    );
                    ui.radio_value(
                        &mut self.new_pattern_type,
                        PatternType::Regex,
                        t::regex(),
                    );
                });

                // Pattern input
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.exclude_input)
                            .hint_text(match self.new_pattern_type {
                                PatternType::Text => t::exclude_pattern_hint(),
                                PatternType::Regex => t::exclude_regex_hint(),
                            })
                            .desired_width(ui.available_width() - 60.0),
                    );

                    let add_enabled = !self.exclude_input.is_empty();
                    if ui
                        .add_enabled(add_enabled, egui::Button::new(t::add()))
                        .clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && add_enabled)
                    {
                        self.exclude_patterns.push(ExcludePattern {
                            pattern: self.exclude_input.clone(),
                            pattern_type: self.new_pattern_type.clone(),
                            enabled: true,
                        });
                        self.exclude_input.clear();
                        changed = true;
                    }
                });

                ui.add_space(4.0);

                // Pattern syntax help
                if self.new_pattern_type == PatternType::Regex {
                    ui.label(
                        RichText::new(t::regex_help())
                            .weak()
                            .small()
                            .italics(),
                    );
                }
            });

            ui.add_space(8.0);

            // === Clear All Button ===
            if ui
                .button(RichText::new(t::clear_all_filters()).color(Color32::from_rgb(244, 67, 54)))
                .clicked()
            {
                filter.enable_all_levels();
                self.exclude_patterns.clear();
                filter.bookmarks_only = false;
                changed = true;
            }
        });

        // Sync enabled patterns back to filter
        if changed {
            filter.exclude_patterns = self.get_enabled_patterns();
        }

        changed
    }
}
