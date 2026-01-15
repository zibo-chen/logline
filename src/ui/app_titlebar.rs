//! Application-level title bar with search

use eframe::egui::{self, Color32, CornerRadius, Stroke, StrokeKind};

/// Application title bar with global search
pub struct AppTitleBar {
    /// Search query
    pub search_query: String,
    /// Whether search is focused
    search_focused: bool,
    /// Track last click time for double-click detection
    last_click_time: Option<std::time::Instant>,
}

impl AppTitleBar {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            search_focused: false,
            last_click_time: None,
        }
    }

    /// Show the app title bar
    /// Returns (search_query, should_toggle_maximize)
    pub fn show(&mut self, ctx: &egui::Context) -> (Option<String>, bool) {
        let mut search_triggered = None;
        let mut should_toggle_maximize = false;
        let is_dark = ctx.style().visuals.dark_mode;

        // Colors based on theme
        let bg_color = if is_dark {
            Color32::from_rgb(30, 30, 30)
        } else {
            Color32::from_rgb(248, 249, 250)
        };

        let search_bg = if is_dark {
            Color32::from_rgb(45, 45, 48)
        } else {
            Color32::from_rgb(255, 255, 255)
        };

        let search_border = if self.search_focused {
            if is_dark {
                Color32::from_rgb(0, 122, 204)
            } else {
                Color32::from_rgb(0, 120, 212)
            }
        } else if is_dark {
            Color32::from_rgb(60, 60, 64)
        } else {
            Color32::from_rgb(204, 206, 219)
        };

        let hint_color = if is_dark {
            Color32::from_rgb(140, 140, 140)
        } else {
            Color32::from_rgb(120, 120, 120)
        };

        let panel_inner = egui::TopBottomPanel::top("app_titlebar")
            .exact_height(36.0)
            .frame(
                egui::Frame::new()
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(0, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Left side: Reserve space for macOS traffic lights
                    #[cfg(target_os = "macos")]
                    ui.add_space(74.0);

                    #[cfg(not(target_os = "macos"))]
                    ui.add_space(16.0);

                    // Center: Search bar with custom styling
                    let available_width = ui.available_width();
                    let search_width = (available_width * 0.6).min(500.0).max(200.0);

                    // Center the search bar
                    let left_padding = (available_width - search_width) / 2.0 - 40.0;
                    if left_padding > 0.0 {
                        ui.add_space(left_padding);
                    }

                    // Custom styled search container
                    let (rect, _response) = ui
                        .allocate_exact_size(egui::vec2(search_width, 24.0), egui::Sense::hover());

                    // Draw search box background
                    let painter = ui.painter();
                    painter.rect(
                        rect,
                        CornerRadius::same(5),
                        search_bg,
                        Stroke::new(1.0, search_border),
                        StrokeKind::Inside,
                    );

                    // Search icon and text input
                    let inner_rect = rect.shrink(1.0);
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(inner_rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );

                    child_ui.add_space(8.0);

                    // Search icon
                    child_ui.label(egui::RichText::new("🔍").size(12.0).color(hint_color));

                    child_ui.add_space(5.0);

                    // Text input
                    let text_edit = egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text(egui::RichText::new("搜索...").color(hint_color))
                        .frame(false)
                        .desired_width(search_width - 80.0)
                        .margin(egui::vec2(0.0, 2.0));

                    let search_response = child_ui.add(text_edit);

                    // Track focus state
                    if search_response.gained_focus() {
                        self.search_focused = true;
                    }
                    if search_response.lost_focus() {
                        self.search_focused = false;
                    }

                    // Trigger search on Enter or when text changes
                    if search_response.changed()
                        || (search_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        if !self.search_query.is_empty() {
                            search_triggered = Some(self.search_query.clone());
                        }
                    }

                    // Show clear button when there's text
                    if !self.search_query.is_empty() {
                        let clear_btn = child_ui.add(
                            egui::Button::new(
                                egui::RichText::new("✕").size(11.0).color(hint_color),
                            )
                            .frame(false)
                            .min_size(egui::vec2(18.0, 18.0)),
                        );
                        if clear_btn.clicked() {
                            self.search_query.clear();
                            search_triggered = Some(String::new());
                        }
                    }

                    // Right side spacing
                    ui.add_space(16.0);
                });
            });

        // Detect double-click on the titlebar area
        let titlebar_rect = panel_inner.response.rect;

        // Check if clicked on titlebar (but not on search area or buttons)
        if ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary)) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if titlebar_rect.contains(pos) {
                    let now = std::time::Instant::now();
                    if let Some(last_time) = self.last_click_time {
                        if now.duration_since(last_time).as_millis() < 500 {
                            // Double-click detected
                            should_toggle_maximize = true;
                            self.last_click_time = None;
                        } else {
                            self.last_click_time = Some(now);
                        }
                    } else {
                        self.last_click_time = Some(now);
                    }
                }
            }
        }

        (search_triggered, should_toggle_maximize)
    }
}
