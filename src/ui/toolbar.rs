//! Toolbar component

use crate::i18n::Translations as t;
use egui::{self, Color32, RichText, Ui};

/// Toolbar component
pub struct Toolbar;

impl Toolbar {
    /// Show the toolbar
    pub fn show(ui: &mut Ui, state: &mut ToolbarState) -> ToolbarAction {
        let mut action = ToolbarAction::None;
        let is_dark = state.dark_theme;
        let inactive_color = if is_dark {
            Color32::LIGHT_GRAY
        } else {
            Color32::DARK_GRAY
        };

        ui.horizontal(|ui| {
            ui.add_space(4.0);

            // Open file button
            if ui
                .button(format!("📂 {}", t::open()))
                .on_hover_text(t::open_file_tooltip())
                .clicked()
            {
                action = ToolbarAction::OpenFile;
            }

            ui.separator();

            // Auto-scroll toggle
            let scroll_text = if state.auto_scroll {
                format!("⏸ {}", t::pause())
            } else {
                format!("▶ {}", t::follow())
            };
            let scroll_color = if state.auto_scroll {
                Color32::from_rgb(100, 200, 100)
            } else {
                inactive_color
            };

            if ui
                .button(RichText::new(scroll_text).color(scroll_color))
                .on_hover_text(t::toggle_auto_scroll_tooltip())
                .clicked()
            {
                action = ToolbarAction::ToggleAutoScroll;
            }

            // Clear button
            if ui
                .button(format!("🗑 {}", t::clear()))
                .on_hover_text(t::clear_display_tooltip())
                .clicked()
            {
                action = ToolbarAction::Clear;
            }

            // Reload button
            if ui
                .button(format!("🔄 {}", t::reload()))
                .on_hover_text(t::reload_file_tooltip())
                .clicked()
            {
                action = ToolbarAction::ReloadFile;
            }

            ui.separator();

            // Reverse order toggle
            let reverse_text = if state.reverse_order {
                format!("🔽 {}", t::newest_first())
            } else {
                format!("🔼 {}", t::oldest_first())
            };
            let reverse_color = if state.reverse_order {
                Color32::from_rgb(255, 200, 100)
            } else {
                inactive_color
            };

            if ui
                .button(RichText::new(reverse_text).color(reverse_color))
                .on_hover_text(t::toggle_order_tooltip())
                .clicked()
            {
                action = ToolbarAction::ToggleReverseOrder;
            }

            ui.separator();

            // Search button
            let search_color = if state.search_visible {
                Color32::from_rgb(100, 150, 255)
            } else {
                inactive_color
            };

            if ui
                .button(RichText::new(format!("🔍 {}", t::search())).color(search_color))
                .on_hover_text(t::toggle_search_tooltip())
                .clicked()
            {
                action = ToolbarAction::ToggleSearch;
            }

            // Go to line button
            if ui
                .button(format!("↓ {}", t::go_to()))
                .on_hover_text(t::go_to_line_tooltip())
                .clicked()
            {
                action = ToolbarAction::GoToLine;
            }

            ui.separator();

            // Navigation buttons
            if ui
                .button("⏮")
                .on_hover_text(t::go_to_top_tooltip())
                .clicked()
            {
                action = ToolbarAction::GoToTop;
            }

            if ui
                .button("⏭")
                .on_hover_text(t::go_to_bottom_tooltip())
                .clicked()
            {
                action = ToolbarAction::GoToBottom;
            }

            // Right side
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle
                let theme_icon = if state.dark_theme { "🌙" } else { "☀" };
                if ui
                    .button(theme_icon)
                    .on_hover_text(t::toggle_theme())
                    .clicked()
                {
                    action = ToolbarAction::ToggleTheme;
                }

                // Settings button
                if ui.button("⚙").on_hover_text(t::settings()).clicked() {
                    action = ToolbarAction::OpenSettings;
                }
            });
        });

        action
    }
}

/// Toolbar state
#[derive(Debug, Clone)]
pub struct ToolbarState {
    pub auto_scroll: bool,
    pub search_visible: bool,
    pub dark_theme: bool,
    pub reverse_order: bool,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            auto_scroll: true,
            search_visible: false,
            dark_theme: true,
            reverse_order: false,
        }
    }
}

/// Actions triggered by toolbar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    None,
    OpenFile,
    ReloadFile,
    ToggleAutoScroll,
    Clear,
    ToggleSearch,
    GoToLine,
    GoToTop,
    GoToBottom,
    ToggleTheme,
    OpenSettings,
    ToggleReverseOrder,
}
