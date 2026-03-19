//! Toolbar component

use crate::i18n::Translations as t;
use crate::log_entry::LogLevel;
use crate::search::FilterConfig;
use egui::{self, Color32, CornerRadius, RichText, Stroke, Ui, Vec2};

/// Toolbar component
pub struct Toolbar;

impl Toolbar {
    /// Show the toolbar
    /// Returns (action, filter_changed) tuple
    pub fn show(
        ui: &mut Ui,
        state: &mut ToolbarState,
        filter: Option<&mut FilterConfig>,
    ) -> (ToolbarAction, bool) {
        let mut action = ToolbarAction::None;
        let mut filter_changed = false;
        let is_dark = state.dark_theme;

        // Theme-based colors
        let button_bg = if is_dark {
            Color32::from_rgb(55, 55, 58)
        } else {
            Color32::from_rgb(240, 240, 240)
        };

        let button_hover_bg = if is_dark {
            Color32::from_rgb(70, 70, 75)
        } else {
            Color32::from_rgb(225, 225, 225)
        };

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(50, 50, 50)
        };

        let inactive_color = if is_dark {
            Color32::from_rgb(160, 160, 160)
        } else {
            Color32::from_rgb(100, 100, 100)
        };

        let separator_color = if is_dark {
            Color32::from_rgb(60, 60, 64)
        } else {
            Color32::from_rgb(220, 220, 225)
        };

        // Apply custom button style
        let button_corner_radius = CornerRadius::same(4);
        let _button_padding = Vec2::new(8.0, 4.0);

        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Helper macro for styled buttons
            let styled_button = |ui: &mut Ui,
                                 icon: &str,
                                 label: &str,
                                 tooltip: &str,
                                 active: bool,
                                 active_color: Option<Color32>|
             -> bool {
                let color = if active {
                    active_color.unwrap_or(Color32::from_rgb(100, 200, 100))
                } else {
                    text_color
                };

                // Create button with custom style that includes hover effect
                let button = egui::Button::new(
                    RichText::new(format!("{} {}", icon, label))
                        .color(color)
                        .size(13.0),
                )
                .corner_radius(button_corner_radius)
                .min_size(Vec2::new(0.0, 26.0));

                // Use ui.scope to apply custom widget visuals
                let response = ui
                    .scope(|ui| {
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.weak_bg_fill = button_bg;
                        visuals.widgets.inactive.bg_fill = button_bg;
                        visuals.widgets.hovered.weak_bg_fill = button_hover_bg;
                        visuals.widgets.hovered.bg_fill = button_hover_bg;
                        visuals.widgets.active.weak_bg_fill = button_hover_bg;
                        visuals.widgets.active.bg_fill = button_hover_bg;

                        ui.add(button)
                    })
                    .inner;

                response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(tooltip)
                    .clicked()
            };

            // Open file button
            if styled_button(ui, "📂", t::open(), t::open_file_tooltip(), false, None) {
                action = ToolbarAction::OpenFile;
            }

            // Custom separator
            ui.add_space(4.0);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                .0;
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
            ui.add_space(4.0);

            // Monitoring toggle
            let monitoring_text = if state.auto_scroll {
                t::stop()
            } else {
                t::start()
            };
            let monitoring_icon = if state.auto_scroll { "⏹" } else { "▶" };
            let monitoring_color = if state.auto_scroll {
                Some(Color32::from_rgb(76, 175, 80))
            } else {
                None
            };

            if styled_button(
                ui,
                monitoring_icon,
                monitoring_text,
                t::toggle_monitoring_tooltip(),
                state.auto_scroll,
                monitoring_color,
            ) {
                action = ToolbarAction::ToggleAutoScroll;
            }

            // Clear button
            if styled_button(ui, "🗑", t::clear(), t::clear_display_tooltip(), false, None) {
                action = ToolbarAction::Clear;
            }

            // Reload button
            if styled_button(ui, "🔄", t::reload(), t::reload_file_tooltip(), false, None) {
                action = ToolbarAction::ReloadFile;
            }

            // Separator
            ui.add_space(4.0);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                .0;
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
            ui.add_space(4.0);

            // Reverse order toggle
            let reverse_text = if state.reverse_order {
                t::newest_first()
            } else {
                t::oldest_first()
            };
            let reverse_icon = if state.reverse_order { "🔼" } else { "🔽" };
            let reverse_color = if state.reverse_order {
                Some(Color32::from_rgb(255, 183, 77))
            } else {
                None
            };

            if styled_button(
                ui,
                reverse_icon,
                reverse_text,
                t::toggle_order_tooltip(),
                state.reverse_order,
                reverse_color,
            ) {
                action = ToolbarAction::ToggleReverseOrder;
            }

            // Separator
            ui.add_space(4.0);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                .0;
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
            ui.add_space(4.0);

            // Search button
            let search_color = if state.search_visible {
                Some(Color32::from_rgb(66, 165, 245))
            } else {
                None
            };

            if styled_button(
                ui,
                "🔍",
                t::search(),
                t::toggle_search_tooltip(),
                state.search_visible,
                search_color,
            ) {
                action = ToolbarAction::ToggleSearch;
            }

            // Go to line button
            if styled_button(ui, "↓", t::go_to(), t::go_to_line_tooltip(), false, None) {
                action = ToolbarAction::GoToLine;
            }

            // Separator
            ui.add_space(4.0);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                .0;
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
            ui.add_space(4.0);

            // Split view toggle button
            let split_icon = if state.split_view_active {
                "⧉"
            } else {
                "⬓"
            };
            let split_text = if state.split_view_active {
                t::close_split()
            } else {
                t::split_view()
            };
            let split_color = if state.split_view_active {
                Some(Color32::from_rgb(76, 175, 80))
            } else {
                None
            };

            if styled_button(
                ui,
                split_icon,
                split_text,
                t::toggle_split_tooltip(),
                state.split_view_active,
                split_color,
            ) {
                action = ToolbarAction::ToggleSplitView;
            }

            // Separator
            ui.add_space(4.0);
            let sep_rect = ui
                .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                .0;
            ui.painter()
                .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
            ui.add_space(4.0);

            // Level filters (if filter config is provided)
            if let Some(filter) = filter {
                ui.label(RichText::new(t::levels()).size(12.0).color(inactive_color));
                ui.add_space(4.0);

                for level in [
                    LogLevel::Trace,
                    LogLevel::Debug,
                    LogLevel::Info,
                    LogLevel::Warn,
                    LogLevel::Error,
                    LogLevel::Fatal,
                ] {
                    let enabled = filter.is_level_enabled(level);
                    let color = if enabled {
                        level.color()
                    } else {
                        Color32::from_rgb(100, 100, 100)
                    };

                    let btn_fill = if enabled {
                        if is_dark {
                            Color32::from_rgba_unmultiplied(
                                level.color().r(),
                                level.color().g(),
                                level.color().b(),
                                30,
                            )
                        } else {
                            Color32::from_rgba_unmultiplied(
                                level.color().r(),
                                level.color().g(),
                                level.color().b(),
                                40,
                            )
                        }
                    } else {
                        Color32::TRANSPARENT
                    };

                    let btn = ui.add(
                        egui::Button::new(
                            RichText::new(level.as_str())
                                .color(color)
                                .size(11.0)
                                .strong(),
                        )
                        .fill(btn_fill)
                        .stroke(if enabled {
                            Stroke::new(1.0, color)
                        } else {
                            Stroke::NONE
                        })
                        .corner_radius(CornerRadius::same(3))
                        .min_size(Vec2::new(0.0, 22.0)),
                    );

                    if btn.clicked() {
                        filter.toggle_level(level);
                        filter_changed = true;
                    }
                }

                ui.add_space(4.0);

                // Quick filter buttons with pill style
                let pill_btn = |ui: &mut Ui, label: &str, tooltip: &str| -> bool {
                    ui.add(
                        egui::Button::new(RichText::new(label).size(11.0).color(inactive_color))
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(CornerRadius::same(10))
                            .min_size(Vec2::new(0.0, 20.0)),
                    )
                    .on_hover_text(tooltip)
                    .clicked()
                };

                if pill_btn(ui, t::all(), t::show_all_levels()) {
                    filter.enable_all_levels();
                    filter_changed = true;
                }

                if pill_btn(ui, t::errors(), t::errors_and_warnings_only()) {
                    filter.errors_and_warnings_only();
                    filter_changed = true;
                }

                // Separator
                ui.add_space(4.0);
                let sep_rect = ui
                    .allocate_exact_size(Vec2::new(1.0, 20.0), egui::Sense::hover())
                    .0;
                ui.painter()
                    .rect_filled(sep_rect, CornerRadius::ZERO, separator_color);
                ui.add_space(4.0);
            }

            // Navigation buttons with icon-only style
            let nav_btn = |ui: &mut Ui, icon: &str, tooltip: &str| -> bool {
                ui.add(
                    egui::Button::new(RichText::new(icon).size(14.0).color(text_color))
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(28.0, 26.0)),
                )
                .on_hover_text(tooltip)
                .clicked()
            };

            if nav_btn(ui, "⏮", t::go_to_top_tooltip()) {
                action = ToolbarAction::GoToTop;
            }

            if nav_btn(ui, "⏭", t::go_to_bottom_tooltip()) {
                action = ToolbarAction::GoToBottom;
            }

            // Right side
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                // Settings button
                let settings_btn = ui.add(
                    egui::Button::new(RichText::new("⚙").size(16.0).color(text_color))
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(30.0, 26.0)),
                );
                if settings_btn.on_hover_text(t::settings()).clicked() {
                    action = ToolbarAction::OpenSettings;
                }

                // Theme toggle with icon
                let theme_icon = if state.dark_theme { "🌙" } else { "☀" };
                let theme_btn = ui.add(
                    egui::Button::new(RichText::new(theme_icon).size(16.0).color(
                        if state.dark_theme {
                            Color32::from_rgb(255, 213, 79)
                        } else {
                            Color32::from_rgb(255, 152, 0)
                        },
                    ))
                    .fill(Color32::TRANSPARENT)
                    .corner_radius(CornerRadius::same(4))
                    .min_size(Vec2::new(30.0, 26.0)),
                );
                if theme_btn.on_hover_text(t::toggle_theme()).clicked() {
                    action = ToolbarAction::ToggleTheme;
                }
            });
        });

        (action, filter_changed)
    }
}

/// Toolbar state
#[derive(Debug, Clone)]
pub struct ToolbarState {
    pub auto_scroll: bool,
    pub search_visible: bool,
    pub dark_theme: bool,
    pub reverse_order: bool,
    pub split_view_active: bool,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            auto_scroll: true,
            search_visible: false,
            dark_theme: true,
            reverse_order: false,
            split_view_active: false,
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
    ToggleSplitView,
}
