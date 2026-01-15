//! Virtual scrolling implementation for efficient rendering of large log files

use crate::log_buffer::{FilteredLogView, LogBuffer};
use crate::log_entry::LogEntry;

/// Configuration for virtual scrolling
#[derive(Debug, Clone)]
pub struct VirtualScrollConfig {
    /// Height of each row in pixels (fixed height mode)
    pub row_height: f32,
    /// Number of extra rows to render above/below viewport (overscan)
    pub overscan: usize,
    /// Minimum scroll threshold before updating
    #[allow(dead_code)]
    pub scroll_threshold: f32,
}

impl Default for VirtualScrollConfig {
    fn default() -> Self {
        Self {
            row_height: 18.0,
            overscan: 5,
            scroll_threshold: 1.0,
        }
    }
}

/// State for virtual scrolling
#[derive(Debug, Clone)]
pub struct VirtualScrollState {
    /// Current scroll offset in pixels
    pub scroll_offset: f32,
    /// Total content height
    pub content_height: f32,
    /// Viewport height
    pub viewport_height: f32,
    /// First visible row index
    pub first_visible_row: usize,
    /// Number of visible rows
    pub visible_row_count: usize,
    /// Whether auto-scroll is enabled
    pub auto_scroll: bool,
    /// Whether user is currently dragging scrollbar
    #[allow(dead_code)]
    pub is_dragging: bool,
    /// Whether to display in reverse order (newest first)
    pub reverse_order: bool,
}

impl Default for VirtualScrollState {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            content_height: 0.0,
            viewport_height: 0.0,
            first_visible_row: 0,
            visible_row_count: 0,
            auto_scroll: true,
            is_dragging: false,
            reverse_order: false,
        }
    }
}

impl VirtualScrollState {
    /// Create new virtual scroll state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update state based on total rows and viewport
    #[allow(dead_code)]
    pub fn update(
        &mut self,
        total_rows: usize,
        viewport_height: f32,
        config: &VirtualScrollConfig,
    ) {
        self.viewport_height = viewport_height;
        self.content_height = total_rows as f32 * config.row_height;

        // Calculate visible rows
        self.visible_row_count =
            (viewport_height / config.row_height).ceil() as usize + config.overscan * 2;
        self.visible_row_count = self.visible_row_count.min(total_rows);

        // Auto-scroll behavior differs in reverse mode
        if self.auto_scroll && total_rows > 0 {
            if self.reverse_order {
                // In reverse mode, auto-scroll to top (newest entries)
                self.scroll_offset = 0.0;
            } else {
                // In normal mode, auto-scroll to bottom (newest entries)
                let max_scroll = (self.content_height - viewport_height).max(0.0);
                self.scroll_offset = max_scroll;
            }
        }

        // Calculate first visible row
        self.first_visible_row = (self.scroll_offset / config.row_height).floor() as usize;
        self.first_visible_row = self.first_visible_row.saturating_sub(config.overscan);

        // Clamp to valid range
        if total_rows > 0 {
            let max_first_row = total_rows.saturating_sub(self.visible_row_count);
            self.first_visible_row = self.first_visible_row.min(max_first_row);
        }
    }

    /// Handle scroll input
    #[allow(dead_code)]
    pub fn scroll(&mut self, delta: f32, config: &VirtualScrollConfig) {
        if delta.abs() < config.scroll_threshold {
            return;
        }

        let max_scroll = (self.content_height - self.viewport_height).max(0.0);
        self.scroll_offset = (self.scroll_offset - delta).clamp(0.0, max_scroll);

        // Smart auto-scroll behavior based on scroll position
        if self.reverse_order {
            // In reverse mode, auto-scroll if near top
            self.auto_scroll = self.scroll_offset <= config.row_height;
        } else {
            // In normal mode, auto-scroll if near bottom
            self.auto_scroll = self.scroll_offset >= max_scroll - config.row_height;
        }

        // Update first visible row
        self.first_visible_row = (self.scroll_offset / config.row_height).floor() as usize;
        self.first_visible_row = self.first_visible_row.saturating_sub(config.overscan);
    }

    /// Scroll to a specific row
    pub fn scroll_to_row(&mut self, row: usize, config: &VirtualScrollConfig) {
        self.auto_scroll = false;
        self.scroll_offset = row as f32 * config.row_height;

        let max_scroll = (self.content_height - self.viewport_height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);

        self.first_visible_row = row.saturating_sub(config.overscan);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = 0.0;
        self.first_visible_row = 0;
    }

    /// Toggle reverse order
    pub fn toggle_reverse_order(&mut self) {
        self.reverse_order = !self.reverse_order;
        // When switching modes, maintain position relative to content
        // In reverse mode, auto-scroll goes to top instead of bottom
        if self.reverse_order && self.auto_scroll {
            self.scroll_to_top();
            self.auto_scroll = true; // Keep auto-scroll enabled in reverse mode
        }
    }

    /// Check if scrolled to bottom
    #[allow(dead_code)]
    pub fn is_at_bottom(&self) -> bool {
        let max_scroll = (self.content_height - self.viewport_height).max(0.0);
        self.scroll_offset >= max_scroll - 1.0
    }

    /// Get the range of rows to render
    pub fn visible_range(&self, total_rows: usize) -> std::ops::Range<usize> {
        let start = self.first_visible_row;
        let end = (start + self.visible_row_count).min(total_rows);
        start..end
    }

    /// Get scroll percentage (0.0 - 1.0)
    #[allow(dead_code)]
    pub fn scroll_percentage(&self) -> f32 {
        let max_scroll = (self.content_height - self.viewport_height).max(0.0);
        if max_scroll > 0.0 {
            self.scroll_offset / max_scroll
        } else {
            0.0
        }
    }

    /// Set scroll by percentage
    #[allow(dead_code)]
    pub fn set_scroll_percentage(&mut self, percentage: f32, config: &VirtualScrollConfig) {
        let max_scroll = (self.content_height - self.viewport_height).max(0.0);
        self.scroll_offset = percentage.clamp(0.0, 1.0) * max_scroll;
        self.first_visible_row = (self.scroll_offset / config.row_height).floor() as usize;
        self.first_visible_row = self.first_visible_row.saturating_sub(config.overscan);
        self.auto_scroll = percentage >= 0.99;
    }
}

/// Virtual scrolling renderer for log entries
pub struct VirtualScroll {
    /// Configuration
    pub config: VirtualScrollConfig,
    /// Current state
    pub state: VirtualScrollState,
}

impl VirtualScroll {
    /// Create a new virtual scroll instance
    pub fn new() -> Self {
        Self {
            config: VirtualScrollConfig::default(),
            state: VirtualScrollState::new(),
        }
    }

    /// Create with custom configuration
    #[allow(dead_code)]
    pub fn with_config(config: VirtualScrollConfig) -> Self {
        Self {
            config,
            state: VirtualScrollState::new(),
        }
    }

    /// Update the scroll state
    #[allow(dead_code)]
    pub fn update(&mut self, total_rows: usize, viewport_height: f32) {
        self.state.update(total_rows, viewport_height, &self.config);
    }

    /// Handle scroll input
    #[allow(dead_code)]
    pub fn scroll(&mut self, delta: f32) {
        self.state.scroll(delta, &self.config);
    }

    /// Scroll to a specific row
    pub fn scroll_to_row(&mut self, row: usize) {
        self.state.scroll_to_row(row, &self.config);
    }

    /// Get visible entries from buffer
    #[allow(dead_code)]
    pub fn get_visible_entries<'a>(&self, buffer: &'a LogBuffer) -> Vec<(usize, &'a LogEntry)> {
        let range = self.state.visible_range(buffer.len());
        buffer
            .get_range(range.clone())
            .enumerate()
            .map(|(i, entry)| (range.start + i, entry))
            .collect()
    }

    /// Get visible entries from filtered view
    #[allow(dead_code)]
    pub fn get_visible_filtered<'a>(
        &self,
        filter: &FilteredLogView,
        buffer: &'a LogBuffer,
    ) -> Vec<(usize, &'a LogEntry)> {
        let range = self.state.visible_range(filter.len());
        filter
            .indices
            .iter()
            .skip(range.start)
            .take(range.len())
            .enumerate()
            .filter_map(|(i, &idx)| buffer.get(idx).map(|entry| (range.start + i, entry)))
            .collect()
    }

    /// Calculate Y position for a row
    #[allow(dead_code)]
    pub fn row_y_position(&self, row_index: usize) -> f32 {
        row_index as f32 * self.config.row_height - self.state.scroll_offset
    }

    /// Get the row at a Y position
    #[allow(dead_code)]
    pub fn row_at_y(&self, y: f32) -> usize {
        ((y + self.state.scroll_offset) / self.config.row_height).floor() as usize
    }

    /// Page up
    #[allow(dead_code)]
    pub fn page_up(&mut self) {
        let page_rows = (self.state.viewport_height / self.config.row_height).floor() as usize;
        let new_row = self.state.first_visible_row.saturating_sub(page_rows);
        self.scroll_to_row(new_row);
    }

    /// Page down
    #[allow(dead_code)]
    pub fn page_down(&mut self, total_rows: usize) {
        let page_rows = (self.state.viewport_height / self.config.row_height).floor() as usize;
        let new_row = (self.state.first_visible_row + page_rows).min(total_rows.saturating_sub(1));
        self.scroll_to_row(new_row);
    }
}

impl Default for VirtualScroll {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_range() {
        let mut scroll = VirtualScroll::new();
        scroll.update(1000, 360.0); // 20 rows visible at 18px height

        let range = scroll.state.visible_range(1000);
        assert!(range.start < range.end);
        assert!(range.end <= 1000);
    }

    #[test]
    fn test_scroll_to_row() {
        let mut scroll = VirtualScroll::new();
        scroll.update(1000, 360.0);
        scroll.scroll_to_row(500);

        let range = scroll.state.visible_range(1000);
        assert!(range.contains(&500) || range.start <= 500);
    }

    #[test]
    fn test_auto_scroll() {
        let mut scroll = VirtualScroll::new();
        scroll.state.auto_scroll = true;
        scroll.update(100, 360.0);

        assert!(scroll.state.is_at_bottom());

        // Scroll up should disable auto-scroll
        scroll.scroll(-100.0);
        assert!(!scroll.state.auto_scroll);
    }
}
