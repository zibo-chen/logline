//! Search bar component

use crate::i18n::Translations as t;
use crate::search::SearchEngine;
use egui::{self, Color32, Key, RichText, Ui};

/// Search bar state
pub struct SearchBar {
    /// Whether the search bar is visible
    pub visible: bool,
    /// Current search input
    pub input: String,
    /// Whether input should be focused
    pub focus_input: bool,
}

impl SearchBar {
    /// Create a new search bar
    pub fn new() -> Self {
        Self {
            visible: false,
            input: String::new(),
            focus_input: false,
        }
    }

    /// Show the search bar
    pub fn show(&mut self, ui: &mut Ui, search: &mut SearchEngine) -> SearchBarAction {
        if !self.visible {
            return SearchBarAction::None;
        }

        let mut action = SearchBarAction::None;

        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Search icon
            ui.label(RichText::new("🔍").size(14.0));

            // Search input
            let response = ui.add_sized(
                [300.0, 24.0],
                egui::TextEdit::singleline(&mut self.input)
                    .hint_text(t::search_placeholder())
                    .font(egui::TextStyle::Monospace),
            );

            // Focus handling
            if self.focus_input {
                response.request_focus();
                self.focus_input = false;
            }

            // Handle input changes
            if response.changed() {
                search.set_query(self.input.clone());
                action = SearchBarAction::SearchChanged;
            }

            // Handle Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                action = SearchBarAction::FindNext;
            }

            // Handle Escape key
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                action = SearchBarAction::Close;
            }

            ui.add_space(8.0);

            // Case sensitivity toggle
            let case_btn = ui.selectable_label(
                search.config.case_sensitive,
                RichText::new("Aa").monospace(),
            );
            if case_btn.clicked() {
                search.set_case_sensitive(!search.config.case_sensitive);
                action = SearchBarAction::SearchChanged;
            }
            case_btn.on_hover_text(t::case_sensitive());

            // Regex toggle
            let regex_btn =
                ui.selectable_label(search.config.use_regex, RichText::new(".*").monospace());
            if regex_btn.clicked() {
                search.set_use_regex(!search.config.use_regex);
                action = SearchBarAction::SearchChanged;
            }
            regex_btn.on_hover_text(t::use_regex());

            // Whole word toggle
            let word_btn =
                ui.selectable_label(search.config.whole_word, RichText::new("\\b").monospace());
            if word_btn.clicked() {
                search.set_whole_word(!search.config.whole_word);
                action = SearchBarAction::SearchChanged;
            }
            word_btn.on_hover_text(t::match_whole_word());

            ui.add_space(16.0);

            // Navigation buttons
            if ui
                .button("◀")
                .on_hover_text(t::previous_match_tooltip())
                .clicked()
            {
                action = SearchBarAction::FindPrev;
            }

            if ui
                .button("▶")
                .on_hover_text(t::next_match_tooltip())
                .clicked()
            {
                action = SearchBarAction::FindNext;
            }

            // Result count
            let result_text = if search.result_count() > 0 {
                format!(
                    "{} / {}",
                    search.current_result_number().unwrap_or(0),
                    search.result_count()
                )
            } else if !self.input.is_empty() {
                t::no_results().to_string()
            } else {
                String::new()
            };

            let text_color = if search.result_count() == 0 && !self.input.is_empty() {
                Color32::from_rgb(255, 100, 100)
            } else {
                Color32::GRAY
            };

            ui.label(RichText::new(result_text).color(text_color).size(12.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Close button
                if ui.button("✕").on_hover_text(t::close_tooltip()).clicked() {
                    action = SearchBarAction::Close;
                }
            });
        });

        action
    }

    /// Open the search bar
    pub fn open(&mut self) {
        self.visible = true;
        self.focus_input = true;
    }

    /// Close the search bar
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }

    /// Clear the search
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.input.clear();
    }

    /// Set search text
    #[allow(dead_code)]
    pub fn set_text(&mut self, text: &str) {
        self.input = text.to_string();
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions triggered by the search bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBarAction {
    None,
    SearchChanged,
    FindNext,
    FindPrev,
    Close,
}
