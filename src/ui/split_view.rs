//! Split View - VSCode-style split panes for viewing multiple logs simultaneously
//!
//! Provides a split view container that can show one or two log views side by side.
//! Supports dragging tabs between panes and adjustable splitter position.

use crate::ui::tab_bar::TabId;
use egui::{self, Color32, Rect, Sense, Stroke, Ui, Vec2};

/// Identifies which pane in a split view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SplitPane {
    /// The left/primary pane (or single pane when not split)
    #[default]
    Left,
    /// The right/secondary pane (only available when split)
    Right,
}

/// Configuration for the split view
#[derive(Debug, Clone)]
pub struct SplitViewConfig {
    /// Whether split view is active
    pub is_split: bool,
    /// Split ratio (0.0-1.0, where 0.5 is equal split)
    pub split_ratio: f32,
    /// Minimum pane width in pixels
    pub min_pane_width: f32,
    /// Active pane (which pane has focus)
    pub active_pane: SplitPane,
    /// Tab ID for left pane
    pub left_tab: Option<TabId>,
    /// Tab ID for right pane
    pub right_tab: Option<TabId>,
}

impl Default for SplitViewConfig {
    fn default() -> Self {
        Self {
            is_split: false,
            split_ratio: 0.5,
            min_pane_width: 200.0,
            active_pane: SplitPane::Left,
            left_tab: None,
            right_tab: None,
        }
    }
}

/// Actions from split view interactions
#[derive(Debug, Clone, PartialEq)]
pub enum SplitAction {
    /// No action
    None,
    /// Split ratio changed
    SplitRatioChanged(f32),
}

/// Split view widget
pub struct SplitView {
    /// Configuration
    pub config: SplitViewConfig,
    /// Is dragging splitter
    dragging_splitter: bool,
    /// Dark theme
    dark_theme: bool,
}

impl Default for SplitView {
    fn default() -> Self {
        Self::new()
    }
}

impl SplitView {
    /// Create a new split view
    pub fn new() -> Self {
        Self {
            config: SplitViewConfig::default(),
            dragging_splitter: false,
            dark_theme: true,
        }
    }

    /// Set theme
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.dark_theme = dark;
    }

    /// Check if split view is active
    pub fn is_split(&self) -> bool {
        self.config.is_split
    }

    /// Get the active pane
    pub fn active_pane(&self) -> SplitPane {
        self.config.active_pane
    }

    /// Get the tab ID for a pane
    pub fn get_pane_tab(&self, pane: SplitPane) -> Option<TabId> {
        match pane {
            SplitPane::Left => self.config.left_tab,
            SplitPane::Right => self.config.right_tab,
        }
    }

    /// Set the tab for a pane
    pub fn set_pane_tab(&mut self, pane: SplitPane, tab_id: Option<TabId>) {
        match pane {
            SplitPane::Left => self.config.left_tab = tab_id,
            SplitPane::Right => self.config.right_tab = tab_id,
        }
    }

    /// Enable split view
    pub fn enable_split(&mut self, right_tab: TabId) {
        self.config.is_split = true;
        self.config.right_tab = Some(right_tab);
    }

    /// Disable split view
    pub fn disable_split(&mut self) {
        self.config.is_split = false;
        self.config.right_tab = None;
        self.config.active_pane = SplitPane::Left;
    }

    /// Set active pane
    pub fn set_active_pane(&mut self, pane: SplitPane) {
        self.config.active_pane = pane;
    }

    /// Set the active tab ID (for the current active pane)
    pub fn set_active_tab(&mut self, tab_id: TabId) {
        if self.config.is_split {
            match self.config.active_pane {
                SplitPane::Left => self.config.left_tab = Some(tab_id),
                SplitPane::Right => self.config.right_tab = Some(tab_id),
            }
        } else {
            self.config.left_tab = Some(tab_id);
        }
    }

    /// Handle tab close - if the closed tab was in a pane, update accordingly
    pub fn handle_tab_close(&mut self, tab_id: TabId) {
        if self.config.left_tab == Some(tab_id) {
            self.config.left_tab = None;
        }
        if self.config.right_tab == Some(tab_id) {
            // If right pane tab is closed, disable split
            self.disable_split();
        }
    }

    /// Render the split view and return rects for each pane
    /// Returns (left_rect, right_rect_option, action)
    pub fn show(&mut self, ui: &mut Ui) -> (Rect, Option<Rect>, SplitAction) {
        let mut action = SplitAction::None;
        let available = ui.available_rect_before_wrap();

        if !self.config.is_split {
            // Single pane mode
            return (available, None, action);
        }

        // Split mode
        let total_width = available.width();
        let splitter_width = 6.0;
        let left_width = (total_width * self.config.split_ratio - splitter_width / 2.0)
            .max(self.config.min_pane_width);
        let right_width =
            (total_width - left_width - splitter_width).max(self.config.min_pane_width);

        // Calculate rects
        let left_rect =
            Rect::from_min_size(available.min, Vec2::new(left_width, available.height()));

        let splitter_rect = Rect::from_min_size(
            egui::pos2(left_rect.max.x, available.min.y),
            Vec2::new(splitter_width, available.height()),
        );

        let right_rect = Rect::from_min_size(
            egui::pos2(splitter_rect.max.x, available.min.y),
            Vec2::new(right_width, available.height()),
        );

        // Draw splitter
        let splitter_color = if self.dark_theme {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(200, 200, 200)
        };

        let splitter_hover_color = if self.dark_theme {
            Color32::from_rgb(0, 122, 204)
        } else {
            Color32::from_rgb(0, 122, 204)
        };

        let splitter_response = ui.interact(
            splitter_rect,
            ui.id().with("split_splitter"),
            Sense::click_and_drag(),
        );

        let splitter_visual_color = if splitter_response.hovered() || splitter_response.dragged() {
            splitter_hover_color
        } else {
            splitter_color
        };

        ui.painter()
            .rect_filled(splitter_rect, 0.0, splitter_visual_color);

        // Draw splitter handle (three dots)
        let handle_y = splitter_rect.center().y;
        let dot_spacing = 6.0;
        for i in -1..=1 {
            let dot_center =
                egui::pos2(splitter_rect.center().x, handle_y + i as f32 * dot_spacing);
            ui.painter().circle_filled(
                dot_center,
                2.0,
                if self.dark_theme {
                    Color32::from_rgb(120, 120, 120)
                } else {
                    Color32::from_rgb(160, 160, 160)
                },
            );
        }

        // Handle splitter drag
        if splitter_response.drag_started() {
            self.dragging_splitter = true;
        }

        if self.dragging_splitter {
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                let new_ratio = (pointer_pos.x - available.min.x) / total_width;
                let clamped_ratio = new_ratio.clamp(
                    self.config.min_pane_width / total_width,
                    1.0 - self.config.min_pane_width / total_width,
                );
                if (clamped_ratio - self.config.split_ratio).abs() > 0.001 {
                    self.config.split_ratio = clamped_ratio;
                    action = SplitAction::SplitRatioChanged(clamped_ratio);
                }
            }

            if splitter_response.drag_stopped() {
                self.dragging_splitter = false;
            }

            // Change cursor
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // Draw active pane indicator (subtle border)
        let active_rect = match self.config.active_pane {
            SplitPane::Left => left_rect,
            SplitPane::Right => right_rect,
        };
        ui.painter().rect_stroke(
            active_rect.shrink(1.0),
            0.0,
            Stroke::new(2.0, Color32::from_rgb(0, 122, 204)),
            egui::StrokeKind::Inside,
        );

        // Note: We don't handle clicks here to switch active pane because
        // the inner MainView scroll areas consume the mouse events.
        // Instead, the active pane is switched when the user interacts with
        // a tab or uses keyboard shortcuts.

        (left_rect, Some(right_rect), action)
    }
}
