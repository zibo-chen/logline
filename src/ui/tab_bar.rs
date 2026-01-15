//! Tab bar component - VSCode-style tabs for multiple log files
//!
//! Provides a horizontal tab bar with drag-to-reorder, close buttons,
//! and visual indicators for active/modified tabs.

use crate::i18n::Translations as I18n;
use egui::{self, Color32, Rect, Sense, Stroke, Ui, Vec2};
use std::path::PathBuf;

/// Truncate text to fit within a given width, adding ellipsis if needed
fn truncate_text_to_width(ui: &Ui, text: &str, font_id: &egui::FontId, max_width: f32) -> String {
    // First check if the text fits
    let full_galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), font_id.clone(), Color32::WHITE);

    if full_galley.size().x <= max_width {
        return text.to_string();
    }

    // If not, use binary search to find the right length
    let ellipsis = "...";
    let ellipsis_galley =
        ui.painter()
            .layout_no_wrap(ellipsis.to_string(), font_id.clone(), Color32::WHITE);
    let ellipsis_width = ellipsis_galley.size().x;
    let target_width = max_width - ellipsis_width;

    if target_width <= 0.0 {
        return ellipsis.to_string();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut low = 0;
    let mut high = chars.len();
    let mut best_len = 0;

    while low <= high {
        let mid = (low + high) / 2;
        if mid == 0 {
            break;
        }

        let substr: String = chars.iter().take(mid).collect();
        let width = ui
            .painter()
            .layout_no_wrap(substr, font_id.clone(), Color32::WHITE)
            .size()
            .x;

        if width <= target_width {
            best_len = mid;
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }

    if best_len == 0 {
        ellipsis.to_string()
    } else {
        let truncated: String = chars.iter().take(best_len).collect();
        format!("{}{}", truncated, ellipsis)
    }
}

/// Unique identifier for a tab
pub type TabId = usize;

/// Information about a single tab
#[derive(Debug, Clone)]
pub struct Tab {
    /// Unique identifier
    pub id: TabId,
    /// Display name (usually filename)
    pub name: String,
    /// Full file path
    pub path: PathBuf,
    /// Whether this is a remote stream
    pub is_remote: bool,
    /// Whether the tab has unsaved changes / new content
    pub is_dirty: bool,
    /// Tooltip text (usually full path)
    pub tooltip: String,
}

impl Tab {
    /// Create a new local file tab
    pub fn new_local(id: TabId, path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let tooltip = path.display().to_string();

        Self {
            id,
            name,
            path,
            is_remote: false,
            is_dirty: false,
            tooltip,
        }
    }

    /// Create a new remote stream tab
    pub fn new_remote(id: TabId, project_name: String, cache_path: PathBuf) -> Self {
        let tooltip = format!("{} ({})", I18n::remote_stream(), cache_path.display());

        Self {
            id,
            name: project_name,
            path: cache_path,
            is_remote: true,
            is_dirty: false,
            tooltip,
        }
    }
}

/// Actions from the tab bar
#[derive(Debug, Clone, PartialEq)]
pub enum TabBarAction {
    /// No action
    None,
    /// Switch to a different tab
    SelectTab(TabId),
    /// Close a tab
    CloseTab(TabId),
    /// Close other tabs (keep the specified one)
    CloseOtherTabs(TabId),
    /// Close all tabs
    CloseAllTabs,
    /// Close tabs to the right
    CloseTabsToRight(TabId),
    /// Reorder tabs (drag from, to)
    ReorderTabs(usize, usize),
    /// Open tab in split view (right pane)
    OpenInSplit(TabId),
}

/// Tab bar widget state
pub struct TabBar {
    /// All open tabs
    pub tabs: Vec<Tab>,
    /// Currently active tab ID
    pub active_tab: Option<TabId>,
    /// Next tab ID counter
    next_id: TabId,
    /// Dragging state: (tab index, start x position)
    dragging: Option<(usize, f32)>,
    /// Hover state for close button
    hovered_close: Option<TabId>,
    /// Dark theme
    dark_theme: bool,
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl TabBar {
    /// Create a new tab bar
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: None,
            next_id: 0,
            dragging: None,
            hovered_close: None,
            dark_theme: true,
        }
    }

    /// Set theme
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.dark_theme = dark;
    }

    /// Add a new tab and return its ID
    pub fn add_tab(&mut self, tab: Tab) -> TabId {
        // Check if already open
        if let Some(existing) = self.tabs.iter().find(|t| t.path == tab.path) {
            let id = existing.id;
            self.active_tab = Some(id);
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;

        let mut new_tab = tab;
        new_tab.id = id;

        self.tabs.push(new_tab);
        self.active_tab = Some(id);

        id
    }

    /// Close a tab and return the closed tab
    pub fn close_tab(&mut self, id: TabId) -> Option<Tab> {
        let index = self.tabs.iter().position(|t| t.id == id)?;
        let tab = self.tabs.remove(index);

        // Update active tab if necessary
        if self.active_tab == Some(id) {
            self.active_tab = if self.tabs.is_empty() {
                None
            } else if index >= self.tabs.len() {
                Some(self.tabs[self.tabs.len() - 1].id)
            } else {
                Some(self.tabs[index].id)
            };
        }

        Some(tab)
    }

    /// Check if a path is already open
    pub fn find_by_path(&self, path: &PathBuf) -> Option<TabId> {
        self.tabs.iter().find(|t| &t.path == path).map(|t| t.id)
    }

    /// Get the number of tabs
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Check if there are no tabs
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Render the tab bar
    pub fn show(&mut self, ui: &mut Ui) -> TabBarAction {
        let mut action = TabBarAction::None;

        if self.tabs.is_empty() {
            return action;
        }

        let tab_height = 32.0;
        let tab_min_width = 120.0;
        let tab_max_width = 200.0;
        let close_button_size = 16.0;

        // Colors
        let (bg_color, active_bg, inactive_bg, border_color, text_color, text_inactive) =
            if self.dark_theme {
                (
                    Color32::from_rgb(30, 30, 30),
                    Color32::from_rgb(45, 45, 45),
                    Color32::from_rgb(35, 35, 35),
                    Color32::from_rgb(60, 60, 60),
                    Color32::from_rgb(220, 220, 220),
                    Color32::from_rgb(150, 150, 150),
                )
            } else {
                (
                    Color32::from_rgb(240, 240, 240),
                    Color32::WHITE,
                    Color32::from_rgb(230, 230, 230),
                    Color32::from_rgb(200, 200, 200),
                    Color32::from_rgb(40, 40, 40),
                    Color32::from_rgb(100, 100, 100),
                )
            };

        let available_width = ui.available_width();

        // Calculate individual tab widths based on content
        let font_id = egui::FontId::proportional(12.0);
        let padding = 60.0; // Icon + close button + margins
        let mut tab_widths = Vec::new();
        let mut total_desired_width = 8.0; // Initial padding

        for tab in &self.tabs {
            let display_name = if tab.is_dirty {
                format!("● {}", tab.name)
            } else {
                tab.name.clone()
            };

            let galley =
                ui.painter()
                    .layout_no_wrap(display_name.clone(), font_id.clone(), text_color);
            let text_width = galley.size().x;
            let desired_width = (text_width + padding).clamp(tab_min_width, tab_max_width);
            tab_widths.push(desired_width);
            total_desired_width += desired_width + 2.0; // +2 for spacing
        }

        // Use horizontal scroll area
        egui::ScrollArea::horizontal()
            .id_salt("tab_bar_scroll")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                // Draw tab bar background
                let (bar_rect, _) = ui.allocate_exact_size(
                    Vec2::new(total_desired_width.max(available_width), tab_height),
                    Sense::hover(),
                );
                ui.painter().rect_filled(bar_rect, 0.0, bg_color);

                // Track drop position for drag-and-drop
                let mut drop_index: Option<usize> = None;
                let mut current_x = bar_rect.min.x + 4.0;

                // Draw tabs
                for (index, tab) in self.tabs.iter().enumerate() {
                    let is_active = self.active_tab == Some(tab.id);
                    let tab_width = tab_widths[index];
                    let tab_rect = Rect::from_min_size(
                        egui::pos2(current_x, bar_rect.min.y + 2.0),
                        Vec2::new(tab_width - 2.0, tab_height - 4.0),
                    );

                    // Tab background
                    let bg = if is_active { active_bg } else { inactive_bg };
                    ui.painter().rect_filled(tab_rect, 4.0, bg);
                    ui.painter().rect_stroke(
                        tab_rect,
                        4.0,
                        Stroke::new(1.0, border_color),
                        egui::StrokeKind::Inside,
                    );

                    // Active indicator (bottom border)
                    if is_active {
                        let indicator_rect = Rect::from_min_size(
                            egui::pos2(tab_rect.min.x, tab_rect.max.y - 2.0),
                            Vec2::new(tab_rect.width(), 2.0),
                        );
                        ui.painter().rect_filled(
                            indicator_rect,
                            0.0,
                            Color32::from_rgb(0, 122, 204),
                        );
                    }

                    // Icon
                    let icon = if tab.is_remote { "📡" } else { "📄" };
                    let icon_pos = egui::pos2(tab_rect.min.x + 8.0, tab_rect.center().y);
                    ui.painter().text(
                        icon_pos,
                        egui::Align2::LEFT_CENTER,
                        icon,
                        egui::FontId::proportional(12.0),
                        text_color,
                    );

                    // Tab name with dirty indicator
                    let display_name = if tab.is_dirty {
                        format!("● {}", tab.name)
                    } else {
                        tab.name.clone()
                    };

                    // Truncate name if too long - force single line
                    let max_name_width = tab_width - 50.0; // Leave room for icon and close button
                    let text_color_for_tab = if is_active { text_color } else { text_inactive };

                    // Manually truncate text to prevent wrapping
                    let font_id = egui::FontId::proportional(12.0);
                    let truncated_text =
                        truncate_text_to_width(ui, &display_name, &font_id, max_name_width);

                    let text_pos = egui::pos2(tab_rect.min.x + 26.0, tab_rect.center().y);
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        truncated_text,
                        font_id,
                        text_color_for_tab,
                    );

                    // Close button
                    let close_rect = Rect::from_center_size(
                        egui::pos2(tab_rect.max.x - 14.0, tab_rect.center().y),
                        Vec2::splat(close_button_size),
                    );

                    let close_response =
                        ui.interact(close_rect, ui.id().with(("close", tab.id)), Sense::click());
                    let close_hovered = close_response.hovered();

                    if close_hovered {
                        self.hovered_close = Some(tab.id);
                        ui.painter().rect_filled(
                            close_rect,
                            4.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        );
                    }

                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "×",
                        egui::FontId::proportional(14.0),
                        if close_hovered {
                            text_color
                        } else {
                            text_inactive
                        },
                    );

                    if close_response.clicked() {
                        action = TabBarAction::CloseTab(tab.id);
                    }

                    // Tab click (excluding close button area)
                    let tab_click_rect = Rect::from_min_max(
                        tab_rect.min,
                        egui::pos2(tab_rect.max.x - close_button_size - 4.0, tab_rect.max.y),
                    );
                    let tab_response = ui.interact(
                        tab_click_rect,
                        ui.id().with(("tab", tab.id)),
                        Sense::click_and_drag(),
                    );

                    // Show tooltip - on_hover_text returns Self so we reassign
                    let tab_response = tab_response.on_hover_text(&tab.tooltip);

                    if tab_response.clicked() {
                        action = TabBarAction::SelectTab(tab.id);
                    }

                    // Context menu
                    tab_response.context_menu(|ui| {
                        ui.set_min_width(150.0);

                        if ui.button(I18n::close()).clicked() {
                            action = TabBarAction::CloseTab(tab.id);
                            ui.close();
                        }

                        if ui.button(I18n::close_others()).clicked() {
                            action = TabBarAction::CloseOtherTabs(tab.id);
                            ui.close();
                        }

                        if ui.button(I18n::close_tabs_to_right()).clicked() {
                            action = TabBarAction::CloseTabsToRight(tab.id);
                            ui.close();
                        }

                        ui.separator();

                        // Split view options
                        if ui.button(I18n::open_in_split()).clicked() {
                            action = TabBarAction::OpenInSplit(tab.id);
                            ui.close();
                        }

                        ui.separator();

                        if ui.button(I18n::close_all()).clicked() {
                            action = TabBarAction::CloseAllTabs;
                            ui.close();
                        }
                    });

                    // Handle drag for reordering
                    if tab_response.drag_started() {
                        self.dragging = Some((index, current_x));
                    }

                    if let Some((drag_index, _)) = self.dragging {
                        if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                            let tab_center = tab_rect.center().x;
                            // Determine drop position based on pointer position relative to tab center
                            if drag_index < index {
                                // Dragging from left to right
                                if pointer_pos.x > tab_center {
                                    drop_index = Some(index);
                                }
                            } else if drag_index > index {
                                // Dragging from right to left
                                if pointer_pos.x < tab_center {
                                    drop_index = Some(index);
                                }
                            }
                        }

                        if tab_response.drag_stopped() {
                            if let Some(target) = drop_index {
                                if drag_index != target {
                                    action = TabBarAction::ReorderTabs(drag_index, target);
                                }
                            }
                            self.dragging = None;
                        }
                    }

                    // Move to next tab position
                    current_x += tab_width + 2.0;
                }

                // Draw drop indicator
                if let (Some((drag_index, _)), Some(target)) = (self.dragging, drop_index) {
                    if drag_index != target {
                        // Calculate target position based on accumulated tab widths
                        let indicator_x = bar_rect.min.x
                            + 4.0
                            + tab_widths[..target].iter().sum::<f32>()
                            + (target as f32 * 2.0);
                        ui.painter().line_segment(
                            [
                                egui::pos2(indicator_x, bar_rect.min.y + 4.0),
                                egui::pos2(indicator_x, bar_rect.max.y - 4.0),
                            ],
                            Stroke::new(2.0, Color32::from_rgb(0, 122, 204)),
                        );
                    }
                }
            });

        action
    }

    /// Reorder tabs
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from < self.tabs.len() && to < self.tabs.len() && from != to {
            let tab = self.tabs.remove(from);
            self.tabs.insert(to, tab);
        }
    }

    /// Close other tabs (keep the specified one)
    pub fn close_other_tabs(&mut self, keep_id: TabId) -> Vec<Tab> {
        let mut closed = Vec::new();
        self.tabs.retain(|t| {
            if t.id != keep_id {
                closed.push(t.clone());
                false
            } else {
                true
            }
        });
        self.active_tab = Some(keep_id);
        closed
    }

    /// Close tabs to the right of the specified tab
    pub fn close_tabs_to_right(&mut self, id: TabId) -> Vec<Tab> {
        let mut closed = Vec::new();
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            while self.tabs.len() > index + 1 {
                closed.push(self.tabs.remove(index + 1));
            }
            // Update active tab if it was closed
            if let Some(active) = self.active_tab {
                if !self.tabs.iter().any(|t| t.id == active) {
                    self.active_tab = Some(id);
                }
            }
        }
        closed
    }

    /// Close all tabs
    pub fn close_all_tabs(&mut self) -> Vec<Tab> {
        let closed = std::mem::take(&mut self.tabs);
        self.active_tab = None;
        closed
    }
}
