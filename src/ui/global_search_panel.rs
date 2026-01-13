//! Global search panel for sidebar

use crate::i18n::Translations;
use crate::log_buffer::LogBuffer;
use crate::log_entry::LogLevel;
use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit, Ui};
use regex::Regex;

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    /// Line number in the file
    pub line_number: usize,
    /// Buffer index
    pub buffer_index: usize,
    /// Line content
    pub content: String,
    /// Log level if detected
    pub level: Option<LogLevel>,
    /// Match positions (start, end) in content
    pub match_positions: Vec<(usize, usize)>,
    /// Whether this line is bookmarked
    pub bookmarked: bool,
}

/// Global search panel state
pub struct GlobalSearchPanel {
    /// Search query
    pub query: String,
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regex
    pub use_regex: bool,
    /// Whole word match
    pub whole_word: bool,
    /// Include bookmarks only
    pub bookmarks_only: bool,
    /// Filter by log levels
    pub level_filter: Vec<LogLevel>,
    /// Show level filter dropdown
    #[allow(dead_code)]
    show_level_filter: bool,
    /// Search results
    pub results: Vec<SearchResultItem>,
    /// Selected result index
    pub selected_index: Option<usize>,
    /// Whether search needs to be executed
    pub dirty: bool,
    /// Maximum results to show
    pub max_results: usize,
    /// Whether dark theme is enabled
    pub dark_theme: bool,
}

impl Default for GlobalSearchPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalSearchPanel {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            bookmarks_only: false,
            level_filter: vec![],
            show_level_filter: false,
            results: Vec::new(),
            selected_index: None,
            dirty: false,
            max_results: 1000,
            dark_theme: true,
        }
    }

    /// Set dark theme
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.dark_theme = dark;
    }

    /// Execute search on buffer
    pub fn search(&mut self, buffer: &LogBuffer) {
        self.results.clear();
        self.selected_index = None;

        if self.query.is_empty() {
            self.dirty = false;
            return;
        }

        // Build regex
        let regex = self.build_regex();
        let regex = match regex {
            Some(r) => r,
            None => {
                self.dirty = false;
                return;
            }
        };

        for (idx, entry) in buffer.iter().enumerate() {
            if self.results.len() >= self.max_results {
                break;
            }

            // Level filter
            if !self.level_filter.is_empty() {
                if let Some(level) = entry.level {
                    if !self.level_filter.contains(&level) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Bookmarks filter
            if self.bookmarks_only && !entry.bookmarked {
                continue;
            }

            // Search match
            let matches: Vec<(usize, usize)> = regex
                .find_iter(&entry.content)
                .map(|m| (m.start(), m.end()))
                .collect();

            if !matches.is_empty() {
                self.results.push(SearchResultItem {
                    line_number: entry.line_number,
                    buffer_index: idx,
                    content: entry.content.clone(),
                    level: entry.level,
                    match_positions: matches,
                    bookmarked: entry.bookmarked,
                });
            }
        }

        self.dirty = false;
    }

    /// Build regex from search config
    fn build_regex(&self) -> Option<Regex> {
        if self.query.is_empty() {
            return None;
        }

        let pattern = if self.use_regex {
            if self.whole_word {
                format!(r"\b{}\b", self.query)
            } else {
                self.query.clone()
            }
        } else {
            let escaped = regex::escape(&self.query);
            if self.whole_word {
                format!(r"\b{}\b", escaped)
            } else {
                escaped
            }
        };

        let pattern = if self.case_sensitive {
            pattern
        } else {
            format!("(?i){}", pattern)
        };

        Regex::new(&pattern).ok()
    }

    /// Show the panel UI
    pub fn show(&mut self, ui: &mut Ui, buffer: &LogBuffer) -> GlobalSearchAction {
        let mut action = GlobalSearchAction::None;

        ui.vertical(|ui| {
            ui.add_space(8.0);

            // Header
            ui.horizontal(|ui| {
                ui.heading("🔍");
                ui.heading(Translations::search());
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Search input
            let response = ui.add(
                TextEdit::singleline(&mut self.query)
                    .hint_text(Translations::global_search_placeholder())
                    .desired_width(ui.available_width()),
            );

            if response.changed() {
                self.dirty = true;
            }

            // Press Enter to search
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.search(buffer);
            }

            ui.add_space(4.0);

            // Search options
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(self.case_sensitive, "Aa")
                    .on_hover_text(Translations::case_sensitive())
                    .clicked()
                {
                    self.case_sensitive = !self.case_sensitive;
                    self.dirty = true;
                }

                if ui
                    .selectable_label(self.use_regex, ".*")
                    .on_hover_text(Translations::use_regex())
                    .clicked()
                {
                    self.use_regex = !self.use_regex;
                    self.dirty = true;
                }

                if ui
                    .selectable_label(self.whole_word, "\\b")
                    .on_hover_text(Translations::match_whole_word())
                    .clicked()
                {
                    self.whole_word = !self.whole_word;
                    self.dirty = true;
                }

                if ui
                    .selectable_label(self.bookmarks_only, "★")
                    .on_hover_text(Translations::bookmarks_only())
                    .clicked()
                {
                    self.bookmarks_only = !self.bookmarks_only;
                    self.dirty = true;
                }
            });

            ui.add_space(4.0);

            // Level filter
            ui.horizontal(|ui| {
                ui.label(Translations::level_filter());

                let level_text = if self.level_filter.is_empty() {
                    "全部".to_string()
                } else {
                    self.level_filter
                        .iter()
                        .map(|l| l.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                egui::ComboBox::from_id_salt("level_filter")
                    .selected_text(level_text)
                    .show_ui(ui, |ui| {
                        let levels = [
                            LogLevel::Error,
                            LogLevel::Warn,
                            LogLevel::Info,
                            LogLevel::Debug,
                            LogLevel::Trace,
                        ];

                        for level in levels {
                            let selected = self.level_filter.contains(&level);
                            if ui.selectable_label(selected, level.as_str()).clicked() {
                                if selected {
                                    self.level_filter.retain(|l| *l != level);
                                } else {
                                    self.level_filter.push(level);
                                }
                                self.dirty = true;
                            }
                        }

                        ui.separator();
                        if ui.button("清除筛选").clicked() {
                            self.level_filter.clear();
                            self.dirty = true;
                        }
                    });
            });

            ui.add_space(4.0);

            // Search button
            ui.horizontal(|ui| {
                if ui
                    .button(format!("🔍 {}", Translations::search()))
                    .clicked()
                    || self.dirty
                {
                    self.search(buffer);
                }

                if !self.results.is_empty() {
                    ui.label(format!(
                        "{} {}",
                        self.results.len(),
                        Translations::results()
                    ));
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Results list
            if self.results.is_empty() {
                if !self.query.is_empty() {
                    ui.label(RichText::new(Translations::global_no_results()).color(Color32::GRAY));
                } else {
                    ui.label(
                        RichText::new(Translations::enter_search_query()).color(Color32::GRAY),
                    );
                }
            } else {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (idx, result) in self.results.iter().enumerate() {
                            let is_selected = self.selected_index == Some(idx);

                            let response = ui
                                .push_id(idx, |ui| self.show_result_item(ui, result, is_selected))
                                .inner;

                            if response.clicked() {
                                self.selected_index = Some(idx);
                                action = GlobalSearchAction::JumpToLine(result.buffer_index);
                            }
                        }
                    });
            }
        });

        action
    }

    /// Show a single result item
    fn show_result_item(
        &self,
        ui: &mut Ui,
        result: &SearchResultItem,
        is_selected: bool,
    ) -> egui::Response {
        let bg_color = if is_selected {
            ui.visuals().selection.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        egui::Frame::none()
            .fill(bg_color)
            .inner_margin(4.0)
            .rounding(2.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.vertical(|ui| {
                    // Line number and level
                    ui.horizontal(|ui| {
                        // Bookmark indicator
                        if result.bookmarked {
                            ui.label(RichText::new("★").color(Color32::from_rgb(255, 193, 7)));
                        }

                        // Line number
                        ui.label(
                            RichText::new(format!("L{}", result.line_number))
                                .color(Color32::GRAY)
                                .small(),
                        );

                        // Log level badge
                        if let Some(level) = result.level {
                            let (bg, fg) = level_colors(level);
                            ui.label(
                                RichText::new(format!(" {} ", level.as_str()))
                                    .background_color(bg)
                                    .color(fg)
                                    .small(),
                            );
                        }
                    });

                    // Content with highlighted matches
                    let content = self.highlight_content(&result.content, &result.match_positions);
                    ui.add(egui::Label::new(content).wrap());
                });
            })
            .response
            .interact(egui::Sense::click())
    }

    /// Highlight matched text in content
    fn highlight_content(&self, content: &str, matches: &[(usize, usize)]) -> RichText {
        let truncated = truncate_string(content, 200);

        if matches.is_empty() {
            return RichText::new(truncated);
        }

        // Use theme-aware text color for search results
        let text_color = if self.dark_theme {
            Color32::from_rgb(212, 212, 212) // Light text for dark theme
        } else {
            Color32::from_rgb(36, 36, 36) // Dark text for light theme
        };

        RichText::new(truncated).color(text_color)
    }
}

/// Actions from global search panel
#[derive(Debug, Clone)]
pub enum GlobalSearchAction {
    None,
    JumpToLine(usize),
}

/// Get colors for log level
fn level_colors(level: LogLevel) -> (Color32, Color32) {
    match level {
        LogLevel::Trace => (Color32::from_rgb(100, 100, 100), Color32::WHITE),
        LogLevel::Debug => (Color32::from_rgb(33, 150, 243), Color32::WHITE),
        LogLevel::Info => (Color32::from_rgb(76, 175, 80), Color32::WHITE),
        LogLevel::Warn => (Color32::from_rgb(255, 152, 0), Color32::BLACK),
        LogLevel::Error => (Color32::from_rgb(244, 67, 54), Color32::WHITE),
        LogLevel::Fatal => (Color32::from_rgb(156, 39, 176), Color32::WHITE),
    }
}

/// Truncate string to max length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
