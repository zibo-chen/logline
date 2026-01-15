//! Main log view component with virtual scrolling

use crate::config::DisplayConfig;
use crate::highlighter::Highlighter;
use crate::log_buffer::LogBuffer;
use crate::log_entry::LogEntry;
use crate::search::SearchEngine;
use crate::virtual_scroll::VirtualScroll;
use egui::{self, Color32, Rect, Response, Sense, Ui, UiKind, Vec2};

/// Context menu actions
#[derive(Clone, Debug, PartialEq)]
pub enum ContextMenuAction {
    /// Copy selected lines
    Copy,
    /// Copy all visible lines
    CopyAll,
    /// Toggle bookmark on selected line
    ToggleBookmark,
    /// Clear selection
    ClearSelection,
    /// Select all lines
    SelectAll,
    /// Scroll to top
    ScrollToTop,
    /// Scroll to bottom
    ScrollToBottom,
}

/// Selection range for multi-line selection
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectionRange {
    /// Start row index (in display order)
    pub start_row: usize,
    /// End row index (in display order)
    pub end_row: usize,
    /// Whether selection is active (dragging)
    pub is_dragging: bool,
}

impl SelectionRange {
    /// Get the minimum row index
    pub fn min_row(&self) -> usize {
        self.start_row.min(self.end_row)
    }

    /// Get the maximum row index
    pub fn max_row(&self) -> usize {
        self.start_row.max(self.end_row)
    }

    /// Check if a row is within the selection
    pub fn contains(&self, row: usize) -> bool {
        row >= self.min_row() && row <= self.max_row()
    }

    /// Get the number of selected rows
    pub fn count(&self) -> usize {
        self.max_row() - self.min_row() + 1
    }
}

/// Main view for displaying log entries
pub struct MainView {
    /// Unique ID for this view instance (to avoid ID collisions in split view)
    pub view_id: egui::Id,
    /// Virtual scroll handler
    pub virtual_scroll: VirtualScroll,
    /// Syntax highlighter
    pub highlighter: Highlighter,
    /// Selected line index (in buffer)
    pub selected_line: Option<usize>,
    /// Multi-line selection range
    pub selection_range: Option<SelectionRange>,
    /// Whether the view has focus
    #[allow(dead_code)]
    pub has_focus: bool,
    /// Target scroll row (used for scroll_to_line, scroll_to_top, scroll_to_bottom)
    scroll_to_row: Option<usize>,
    /// Current row height (cached for scroll calculations)
    current_row_height: f32,
    /// Current total rows (cached for scroll calculations)
    current_total_rows: usize,
    /// Pending scroll to bottom request
    pending_scroll_to_bottom: bool,
}

impl MainView {
    /// Create a new main view
    pub fn new() -> Self {
        Self::with_id(egui::Id::new("main_log_view_default"))
    }

    /// Create a new main view with a specific ID
    pub fn with_id(view_id: egui::Id) -> Self {
        Self {
            view_id,
            virtual_scroll: VirtualScroll::new(),
            highlighter: Highlighter::new(),
            selected_line: None,
            selection_range: None,
            has_focus: false,
            scroll_to_row: None,
            current_row_height: 18.0,
            current_total_rows: 0,
            pending_scroll_to_bottom: false,
        }
    }

    /// Render the main view
    pub fn show(
        &mut self,
        ui: &mut Ui,
        buffer: &LogBuffer,
        filtered_indices: Option<&[usize]>,
        search: &SearchEngine,
        display_config: &DisplayConfig,
    ) -> (Response, Option<ContextMenuAction>) {
        let total_rows = filtered_indices.map(|f| f.len()).unwrap_or(buffer.len());

        // Calculate layout
        let available_size = ui.available_size();
        let row_height = display_config.font_size * display_config.line_height;

        // Update virtual scroll config
        self.virtual_scroll.config.row_height = row_height;
        self.current_row_height = row_height;
        self.current_total_rows = total_rows;

        // Calculate target scroll offset if we need to scroll to a specific row
        let mut scroll_to_y = self.scroll_to_row.take().map(|buffer_idx| {
            // Convert buffer_idx to display_row
            let logical_row = if let Some(indices) = filtered_indices {
                // Find the position of buffer_idx in filtered_indices
                indices
                    .iter()
                    .position(|&idx| idx == buffer_idx)
                    .unwrap_or(0)
            } else {
                buffer_idx
            };

            // In reverse order mode, flip the row index
            let display_row = if self.virtual_scroll.state.reverse_order {
                total_rows.saturating_sub(1).saturating_sub(logical_row)
            } else {
                logical_row
            };

            display_row as f32 * row_height
        });

        // Track if this is a scroll to bottom operation
        let is_scroll_to_bottom = self.pending_scroll_to_bottom;

        // Handle pending scroll to bottom (uses actual total_rows)
        if self.pending_scroll_to_bottom {
            self.pending_scroll_to_bottom = false;
            if total_rows > 0 {
                // "Bottom" means the newest log (last buffer_idx), not the bottom of screen
                // Find the last buffer_idx
                let last_buffer_idx = if let Some(indices) = filtered_indices {
                    indices.last().copied().unwrap_or(0)
                } else {
                    total_rows.saturating_sub(1)
                };

                // Convert to display row
                let logical_row = if let Some(indices) = filtered_indices {
                    indices
                        .iter()
                        .position(|&idx| idx == last_buffer_idx)
                        .unwrap_or(0)
                } else {
                    last_buffer_idx
                };

                let display_row = if self.virtual_scroll.state.reverse_order {
                    total_rows.saturating_sub(1).saturating_sub(logical_row)
                } else {
                    logical_row
                };

                scroll_to_y = Some(display_row as f32 * row_height);
            }
        }

        // When user scrolls to bottom, enable auto-scroll to follow new logs
        // When user scrolls to other positions, disable auto-scroll to view specific content
        // Only enable auto-scroll when jumping to the location where new logs appear:
        // - Normal mode: bottom is where new logs appear
        // - Reverse mode: top (buffer_idx max) is where new logs appear, not bottom
        if is_scroll_to_bottom && !self.virtual_scroll.state.reverse_order {
            self.virtual_scroll.state.auto_scroll = true;
        } else if scroll_to_y.is_some() {
            self.virtual_scroll.state.auto_scroll = false;
        }

        // Create the scroll area with unique ID for state persistence
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt(self.view_id)
            .auto_shrink([false, false]);

        // Handle auto-scroll based on order mode
        if self.virtual_scroll.state.auto_scroll {
            if self.virtual_scroll.state.reverse_order {
                // In reverse mode, auto-scroll to top (row 0)
                // Only set if no explicit scroll target
                if scroll_to_y.is_none() {
                    scroll_to_y = Some(0.0);
                }
            } else {
                // In normal mode, use stick_to_bottom
                scroll_area = scroll_area.stick_to_bottom(true);
            }
        }

        // Apply scroll offset when explicitly requested
        if let Some(y) = scroll_to_y {
            // For scroll to bottom with auto-scroll, we need to explicitly set the offset
            // to ensure immediate scrolling (stick_to_bottom alone won't scroll if already at bottom)
            // For other positions, only scroll if auto-scroll is disabled
            if is_scroll_to_bottom || !self.virtual_scroll.state.auto_scroll {
                scroll_area = scroll_area.vertical_scroll_offset(y);
            }
        }

        let response = scroll_area.show(ui, |ui| {
            // Reserve space for all content
            let content_height = total_rows as f32 * row_height;

            // Use a large width to allow horizontal scrolling
            let content_width = available_size.x.max(3000.0);

            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(content_width, content_height.max(available_size.y)),
                Sense::click_and_drag(),
            );

            // Get clip rect (the visible viewport)
            let clip_rect = ui.clip_rect();

            // Calculate which rows are actually visible based on clip_rect
            // The clip_rect tells us what part of the content is visible
            let first_visible_row = if rect.min.y < clip_rect.min.y {
                // Content is scrolled up, some rows are above viewport
                ((clip_rect.min.y - rect.min.y) / row_height).floor() as usize
            } else {
                0
            };

            let last_visible_row = if rect.min.y < clip_rect.max.y {
                ((clip_rect.max.y - rect.min.y) / row_height).ceil() as usize
            } else {
                0
            };

            // Add overscan for smoother scrolling (larger buffer to reduce flicker)
            let overscan = 5;
            let start_row = first_visible_row.saturating_sub(overscan);
            let end_row = (last_visible_row + overscan).min(total_rows);

            // Get line number width
            let max_line_num = buffer.last_line_number().max(1);
            let line_num_width = format!("{}", max_line_num)
                .len()
                .max(display_config.line_number_width);
            let line_num_pixel_width = if display_config.show_line_numbers {
                (line_num_width as f32 + 2.0) * display_config.font_size * 0.6
            } else {
                0.0
            };

            // Render visible rows
            let painter = ui.painter();
            let reverse_order = self.virtual_scroll.state.reverse_order;

            for row_idx in start_row..end_row {
                // Get the actual buffer index
                // In reverse order mode, we need to flip the row index
                let display_row = if reverse_order {
                    total_rows.saturating_sub(1).saturating_sub(row_idx)
                } else {
                    row_idx
                };

                let buffer_idx = if let Some(indices) = filtered_indices {
                    indices.get(display_row).copied()
                } else {
                    Some(display_row)
                };

                let Some(buffer_idx) = buffer_idx else {
                    continue;
                };
                let Some(entry) = buffer.get(buffer_idx) else {
                    continue;
                };

                // Calculate row position
                let row_y = rect.min.y + row_idx as f32 * row_height;
                let row_rect = Rect::from_min_size(
                    egui::pos2(rect.min.x, row_y),
                    Vec2::new(rect.width(), row_height),
                );

                // Draw selection background (multi-line selection)
                // selection_range stores logical rows, so we need to check display_row
                let is_in_selection = self
                    .selection_range
                    .map(|sel| sel.contains(display_row))
                    .unwrap_or(false);

                if is_in_selection {
                    painter.rect_filled(row_rect, 0.0, self.highlighter.theme.selection);
                } else if Some(buffer_idx) == self.selected_line && self.selection_range.is_none() {
                    // Single line selection (only when no multi-line selection)
                    painter.rect_filled(row_rect, 0.0, self.highlighter.theme.selection);
                }

                // Draw current search match highlight
                if search.is_current_match(buffer_idx) {
                    painter.rect_filled(row_rect, 0.0, self.highlighter.theme.current_match);
                }

                // Draw bookmark indicator
                if entry.bookmarked {
                    let bookmark_rect =
                        Rect::from_min_size(row_rect.min, Vec2::new(4.0, row_height));
                    painter.rect_filled(bookmark_rect, 0.0, self.highlighter.theme.bookmark);
                }

                // Draw line number
                let mut text_x = rect.min.x + 8.0;
                if display_config.show_line_numbers {
                    let line_num_text =
                        format!("{:>width$}", entry.line_number, width = line_num_width);
                    painter.text(
                        egui::pos2(text_x, row_y + row_height * 0.5),
                        egui::Align2::LEFT_CENTER,
                        &line_num_text,
                        egui::FontId::monospace(display_config.font_size),
                        self.highlighter.theme.line_number,
                    );
                    text_x += line_num_pixel_width;
                }

                // Draw separator line
                if display_config.show_line_numbers {
                    painter.line_segment(
                        [
                            egui::pos2(text_x - 4.0, row_y),
                            egui::pos2(text_x - 4.0, row_y + row_height),
                        ],
                        egui::Stroke::new(1.0, Color32::from_gray(60)),
                    );
                }

                // Draw log content with highlighting
                let search_query = if search.is_active() {
                    Some(search.config.query.as_str())
                } else {
                    None
                };

                // Use infinite width for text (no wrapping, allows horizontal scrolling)
                let text_available_width = f32::INFINITY;

                let layout_job = self.highlighter.highlight_line_with_wrap(
                    &entry.content,
                    entry.level,
                    search_query,
                    search.config.case_sensitive,
                    text_available_width,
                );

                // Layout the text with the context fonts
                let galley = ui.painter().layout_job(layout_job);
                painter.galley(
                    egui::pos2(text_x + 4.0, row_y + (row_height - galley.size().y) * 0.5),
                    galley,
                    Color32::WHITE,
                );

                // Draw row separator line at the bottom of each row
                if display_config.show_row_separator {
                    painter.line_segment(
                        [
                            egui::pos2(rect.min.x, row_y + row_height - 0.5),
                            egui::pos2(rect.max.x, row_y + row_height - 0.5),
                        ],
                        egui::Stroke::new(0.5, Color32::from_gray(45)),
                    );
                }
            }

            // Handle drag for multi-line selection
            if response.drag_started() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let clicked_row = ((pos.y - rect.min.y) / row_height).floor() as usize;
                    if clicked_row < total_rows {
                        // In reverse order mode, flip the row index to get the correct logical row
                        let logical_row = if reverse_order {
                            total_rows.saturating_sub(1).saturating_sub(clicked_row)
                        } else {
                            clicked_row
                        };

                        self.selection_range = Some(SelectionRange {
                            start_row: logical_row,
                            end_row: logical_row,
                            is_dragging: true,
                        });
                        self.selected_line = None;
                    }
                }
            }

            // Update selection during drag
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let current_row = ((pos.y - rect.min.y) / row_height).floor() as isize;
                    let current_row = current_row.max(0) as usize;
                    let current_row = current_row.min(total_rows.saturating_sub(1));

                    // In reverse order mode, flip the row index to get the correct logical row
                    let logical_row = if reverse_order {
                        total_rows.saturating_sub(1).saturating_sub(current_row)
                    } else {
                        current_row
                    };

                    if let Some(ref mut sel) = self.selection_range {
                        if sel.is_dragging {
                            sel.end_row = logical_row;
                        }
                    }
                }
            }

            // End drag
            if response.drag_stopped() {
                if let Some(ref mut sel) = self.selection_range {
                    sel.is_dragging = false;
                }
            }

            // Handle click for single selection (only if not dragging)
            if response.clicked() && self.selection_range.map(|s| !s.is_dragging).unwrap_or(true) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let clicked_row = ((pos.y - rect.min.y) / row_height).floor() as usize;
                    if clicked_row < total_rows {
                        // In reverse order mode, flip the row index to get the correct logical row
                        let logical_row = if reverse_order {
                            total_rows.saturating_sub(1).saturating_sub(clicked_row)
                        } else {
                            clicked_row
                        };

                        // Clear multi-line selection on single click
                        self.selection_range = None;
                        self.selected_line = if let Some(indices) = filtered_indices {
                            indices.get(logical_row).copied()
                        } else {
                            Some(logical_row)
                        };
                    }
                }
            }

            (response, total_rows)
        });

        // In reverse mode, detect if user scrolled away from top to disable auto-scroll
        // In normal mode, stick_to_bottom handles this automatically
        if self.virtual_scroll.state.reverse_order && self.virtual_scroll.state.auto_scroll {
            // Get the current scroll offset from the ScrollArea state
            let current_offset = response.state.offset.y;
            // If user scrolled away from top (offset > small threshold), disable auto-scroll
            if current_offset > row_height * 0.5 {
                self.virtual_scroll.state.auto_scroll = false;
            }
        }

        // Context menu
        let mut context_action = None;
        let has_selection = self.has_selection();
        let total_rows = response.inner.1;

        response.inner.0.context_menu(|ui| {
            ui.set_min_width(150.0);

            if ui
                .add_enabled(
                    has_selection,
                    egui::Button::new("📋 复制").shortcut_text("⌘C"),
                )
                .clicked()
            {
                context_action = Some(ContextMenuAction::Copy);
                ui.close_kind(UiKind::Menu);
            }

            if ui.button("📄 复制全部").clicked() {
                context_action = Some(ContextMenuAction::CopyAll);
                ui.close_kind(UiKind::Menu)
            }

            ui.separator();

            if ui
                .add_enabled(
                    has_selection,
                    egui::Button::new("🔖 切换书签              ⌘B"),
                )
                .clicked()
            {
                context_action = Some(ContextMenuAction::ToggleBookmark);
                ui.close_kind(UiKind::Menu)
            }

            ui.separator();

            if ui.button("✓ 全选                        ⌘A").clicked() {
                context_action = Some(ContextMenuAction::SelectAll);
                ui.close_kind(UiKind::Menu)
            }

            if ui
                .add_enabled(has_selection, egui::Button::new("✗ 清除选择"))
                .clicked()
            {
                context_action = Some(ContextMenuAction::ClearSelection);
                ui.close_kind(UiKind::Menu)
            }

            ui.separator();

            if ui.button("⬆ 滚动到顶部            Home").clicked() {
                context_action = Some(ContextMenuAction::ScrollToTop);
                ui.close_kind(UiKind::Menu)
            }

            if ui.button("⬇ 滚动到底部             End").clicked() {
                context_action = Some(ContextMenuAction::ScrollToBottom);
                ui.close_kind(UiKind::Menu)
            }
        });

        // Handle SelectAll here since we have total_rows
        if context_action == Some(ContextMenuAction::SelectAll) && total_rows > 0 {
            self.selection_range = Some(SelectionRange {
                start_row: 0,
                end_row: total_rows.saturating_sub(1),
                is_dragging: false,
            });
            self.selected_line = None;
        }

        (response.inner.0, context_action)
    }

    /// Scroll to a specific line (by buffer index)
    pub fn scroll_to_line(&mut self, line_index: usize) {
        self.scroll_to_row = Some(line_index);
        self.selected_line = Some(line_index);
    }

    /// Scroll to top (first log entry, line number 1)
    pub fn scroll_to_top(&mut self) {
        // Jump to buffer_idx 0 (line number 1, oldest log)
        self.scroll_to_row = Some(0);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        // Use pending flag so we can use actual total_rows in show()
        self.pending_scroll_to_bottom = true;
    }

    /// Check if auto-scroll is enabled
    pub fn is_auto_scroll(&self) -> bool {
        self.virtual_scroll.state.auto_scroll
    }

    /// Toggle reverse order
    pub fn toggle_reverse_order(&mut self) {
        self.virtual_scroll.state.toggle_reverse_order();
    }

    /// Check if reverse order is enabled
    pub fn is_reverse_order(&self) -> bool {
        self.virtual_scroll.state.reverse_order
    }

    /// Get selected entry
    #[allow(dead_code)]
    pub fn get_selected<'a>(&self, buffer: &'a LogBuffer) -> Option<&'a LogEntry> {
        self.selected_line.and_then(|i| buffer.get(i))
    }

    /// Get selected text (supports multi-line selection)
    pub fn get_selected_text(
        &self,
        buffer: &LogBuffer,
        filtered_indices: Option<&[usize]>,
    ) -> Option<String> {
        if let Some(sel) = self.selection_range {
            // Multi-line selection
            let mut lines = Vec::new();
            for row_idx in sel.min_row()..=sel.max_row() {
                let buffer_idx = if let Some(indices) = filtered_indices {
                    indices.get(row_idx).copied()
                } else {
                    Some(row_idx)
                };

                if let Some(idx) = buffer_idx {
                    if let Some(entry) = buffer.get(idx) {
                        lines.push(entry.content.clone());
                    }
                }
            }
            if !lines.is_empty() {
                return Some(lines.join("\n"));
            }
        } else if let Some(selected) = self.selected_line {
            // Single line selection
            if let Some(entry) = buffer.get(selected) {
                return Some(entry.content.clone());
            }
        }
        None
    }

    /// Check if there is any selection
    pub fn has_selection(&self) -> bool {
        self.selection_range.is_some() || self.selected_line.is_some()
    }

    /// Get the number of selected lines
    pub fn selected_lines_count(&self) -> usize {
        if let Some(sel) = self.selection_range {
            sel.count()
        } else if self.selected_line.is_some() {
            1
        } else {
            0
        }
    }

    /// Get all selected buffer indices
    pub fn get_selected_indices(&self, filtered_indices: Option<&[usize]>) -> Vec<usize> {
        if let Some(sel) = self.selection_range {
            // Multi-line selection
            let mut indices = Vec::new();
            for row_idx in sel.min_row()..=sel.max_row() {
                let buffer_idx = if let Some(filter_indices) = filtered_indices {
                    filter_indices.get(row_idx).copied()
                } else {
                    Some(row_idx)
                };

                if let Some(idx) = buffer_idx {
                    indices.push(idx);
                }
            }
            indices
        } else if let Some(selected) = self.selected_line {
            // Single line selection
            vec![selected]
        } else {
            Vec::new()
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_line = None;
        self.selection_range = None;
    }

    /// Set selection to a single line or range
    pub fn set_selection(&mut self, start_row: usize, end_row: usize) {
        if start_row == end_row {
            self.selected_line = Some(start_row);
            self.selection_range = None;
        } else {
            self.selection_range = Some(SelectionRange {
                start_row,
                end_row,
                is_dragging: false,
            });
            self.selected_line = None;
        }
    }

    /// Select all rows
    pub fn select_all(&mut self, total_rows: usize) {
        if total_rows > 0 {
            self.selection_range = Some(SelectionRange {
                start_row: 0,
                end_row: total_rows.saturating_sub(1),
                is_dragging: false,
            });
            self.selected_line = None;
        }
    }

    /// Set theme
    pub fn set_dark_theme(&mut self, dark: bool) {
        self.highlighter.theme = if dark {
            crate::highlighter::HighlightTheme::dark()
        } else {
            crate::highlighter::HighlightTheme::light()
        };
    }
}

impl Default for MainView {
    fn default() -> Self {
        Self::new()
    }
}
