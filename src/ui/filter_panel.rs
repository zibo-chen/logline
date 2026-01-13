//! Filter panel component

use crate::i18n::Translations as t;
use crate::log_entry::LogLevel;
use crate::search::FilterConfig;
use egui::{self, Color32, RichText, Ui};

/// Filter panel component
pub struct FilterPanel {
    /// Whether the panel is expanded
    pub expanded: bool,
    /// New exclude pattern input
    exclude_input: String,
}

impl FilterPanel {
    /// Create a new filter panel
    pub fn new() -> Self {
        Self {
            expanded: false,
            exclude_input: String::new(),
        }
    }

    /// Show the filter panel
    pub fn show(&mut self, ui: &mut Ui, filter: &mut FilterConfig) -> bool {
        let mut changed = false;

        // Compact level filters (always visible)
        ui.horizontal(|ui| {
            ui.label(t::levels());
            
            for level in [
                LogLevel::Trace,
                LogLevel::Debug,
                LogLevel::Info,
                LogLevel::Warn,
                LogLevel::Error,
                LogLevel::Fatal,
            ] {
                let enabled = filter.is_level_enabled(level);
                let color = if enabled { level.color() } else { Color32::GRAY };
                
                let btn = ui.selectable_label(
                    enabled,
                    RichText::new(level.as_str()).color(color).small(),
                );
                
                if btn.clicked() {
                    filter.toggle_level(level);
                    changed = true;
                }
            }
            
            ui.separator();
            
            // Quick filter buttons
            if ui.small_button(t::all()).on_hover_text(t::show_all_levels()).clicked() {
                filter.enable_all_levels();
                changed = true;
            }
            
            if ui.small_button(t::errors()).on_hover_text(t::errors_and_warnings_only()).clicked() {
                filter.errors_and_warnings_only();
                changed = true;
            }
            
            // Expand/collapse advanced filters
            let expand_text = if self.expanded { 
                format!("▼ {}", t::less()) 
            } else { 
                format!("▶ {}", t::more()) 
            };
            if ui.small_button(expand_text).clicked() {
                self.expanded = !self.expanded;
            }
        });

        // Expanded advanced filters
        if self.expanded {
            ui.add_space(8.0);
            
            ui.group(|ui| {
                ui.label(RichText::new(t::advanced_filters()).strong());
                
                ui.add_space(4.0);
                
                // Bookmarks only
                if ui.checkbox(&mut filter.bookmarks_only, t::bookmarks_only()).changed() {
                    changed = true;
                }
                
                ui.add_space(8.0);
                
                // Exclude patterns
                ui.label(t::exclude_patterns());
                
                // List existing patterns
                let mut to_remove = None;
                for (i, pattern) in filter.exclude_patterns.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {}", pattern));
                        if ui.small_button("✕").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                
                if let Some(idx) = to_remove {
                    filter.remove_exclude(idx);
                    changed = true;
                }
                
                // Add new pattern
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.exclude_input)
                            .hint_text(t::exclude_pattern_hint())
                            .desired_width(200.0),
                    );
                    
                    let add_enabled = !self.exclude_input.is_empty();
                    if ui.add_enabled(add_enabled, egui::Button::new(t::add())).clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && add_enabled)
                    {
                        filter.add_exclude(self.exclude_input.clone());
                        self.exclude_input.clear();
                        changed = true;
                    }
                });
                
                ui.add_space(4.0);
                
                // Clear all filters
                if ui.button(t::clear_all_filters()).clicked() {
                    filter.enable_all_levels();
                    filter.exclude_patterns.clear();
                    filter.bookmarks_only = false;
                    changed = true;
                }
            });
        }

        changed
    }

    /// Show compact inline level filters
    #[allow(dead_code)]
    pub fn show_compact(&mut self, ui: &mut Ui, filter: &mut FilterConfig) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            for level in [
                LogLevel::Error,
                LogLevel::Warn,
                LogLevel::Info,
                LogLevel::Debug,
                LogLevel::Trace,
            ] {
                let enabled = filter.is_level_enabled(level);
                let color = if enabled { level.color() } else { Color32::from_gray(80) };
                
                let text = match level {
                    LogLevel::Error => "E",
                    LogLevel::Warn => "W",
                    LogLevel::Info => "I",
                    LogLevel::Debug => "D",
                    LogLevel::Trace => "T",
                    LogLevel::Fatal => "F",
                };
                
                let btn = ui.selectable_label(
                    enabled,
                    RichText::new(text).color(color).monospace(),
                );
                
                if btn.on_hover_text(level.as_str()).clicked() {
                    filter.toggle_level(level);
                    changed = true;
                }
            }
        });

        changed
    }
}

impl Default for FilterPanel {
    fn default() -> Self {
        Self::new()
    }
}
