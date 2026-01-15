//! Bookmarks Panel
//!
//! Shows all bookmarked lines grouped into continuous segments.

use crate::i18n::Translations as t;
use crate::log_buffer::LogBuffer;
use egui::{self, Color32, RichText, Ui};

/// A continuous segment of bookmarked lines
#[derive(Debug, Clone)]
pub struct BookmarkSegment {
    /// Start line number (inclusive)
    pub start_line: usize,
    /// End line number (inclusive)
    pub end_line: usize,
    /// Indices in the buffer
    pub indices: Vec<usize>,
}

/// Actions from the bookmarks panel
#[derive(Debug, Clone)]
pub enum BookmarkAction {
    None,
    /// Jump to a specific line
    JumpToLine(usize),
    /// Remove bookmark from a line
    #[allow(dead_code)]
    RemoveBookmark(usize),
    /// Remove all bookmarks from a segment
    RemoveSegment(Vec<usize>),
    /// Clear all bookmarks
    ClearAll,
}

/// Bookmarks Panel component
pub struct BookmarksPanel {
    /// Currently selected segment index
    selected_segment: Option<usize>,
}

impl Default for BookmarksPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl BookmarksPanel {
    /// Create a new bookmarks panel
    pub fn new() -> Self {
        Self {
            selected_segment: None,
        }
    }

    /// Group bookmarked entries into continuous segments
    fn group_bookmarks(buffer: &LogBuffer) -> Vec<BookmarkSegment> {
        let mut segments = Vec::new();
        let mut current_segment: Option<BookmarkSegment> = None;

        for (idx, entry) in buffer.iter().enumerate() {
            if entry.bookmarked {
                match &mut current_segment {
                    None => {
                        // Start a new segment
                        current_segment = Some(BookmarkSegment {
                            start_line: entry.line_number,
                            end_line: entry.line_number,
                            indices: vec![idx],
                        });
                    }
                    Some(segment) => {
                        // Check if this is continuous (within 1 line)
                        if entry.line_number == segment.end_line + 1 {
                            // Extend current segment
                            segment.end_line = entry.line_number;
                            segment.indices.push(idx);
                        } else {
                            // Start a new segment
                            segments.push(segment.clone());
                            current_segment = Some(BookmarkSegment {
                                start_line: entry.line_number,
                                end_line: entry.line_number,
                                indices: vec![idx],
                            });
                        }
                    }
                }
            }
        }

        // Push the last segment if any
        if let Some(segment) = current_segment {
            segments.push(segment);
        }

        segments
    }

    /// Render the bookmarks panel
    pub fn show(&mut self, ui: &mut Ui, buffer: &LogBuffer) -> BookmarkAction {
        let mut action = BookmarkAction::None;

        // Set minimum width to prevent panel from shrinking
        ui.set_min_width(200.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading(RichText::new(t::bookmarks()).strong());
                ui.add_space(12.0);

                let segments = Self::group_bookmarks(buffer);

                if segments.is_empty() {
                    // Empty state with better visual
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("★").size(32.0).weak());
                        ui.add_space(8.0);
                        ui.label(RichText::new(t::no_bookmarks()).weak().italics());
                        ui.add_space(8.0);
                        ui.label(RichText::new(t::bookmark_hint()).weak().small());
                    });
                } else {
                    // Statistics
                    let total_lines: usize = segments.iter().map(|s| s.indices.len()).sum();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}: {}", t::total_segments(), segments.len()))
                                .small(),
                        );
                        ui.label(RichText::new("  •  ").weak().small());
                        ui.label(
                            RichText::new(format!("{}: {}", t::total_bookmarks(), total_lines))
                                .small(),
                        );
                    });

                    ui.add_space(12.0);

                    // Segments list
                    for (seg_idx, segment) in segments.iter().enumerate() {
                        let _is_selected = self.selected_segment == Some(seg_idx);

                        // Simple horizontal layout, no background
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⭐").size(14.0));

                            let segment_text = if segment.start_line == segment.end_line {
                                format!("{} {}", t::line(), segment.start_line)
                            } else {
                                format!(
                                    "{} {}-{}",
                                    t::lines(),
                                    segment.start_line,
                                    segment.end_line
                                )
                            };

                            // Clickable line range
                            if ui
                                .link(RichText::new(&segment_text).monospace())
                                .on_hover_text(t::go_to_line())
                                .clicked()
                            {
                                self.selected_segment = Some(seg_idx);
                                action = BookmarkAction::JumpToLine(segment.start_line);
                            }

                            // Line count badge
                            let count = segment.indices.len();
                            ui.label(RichText::new(format!("({}L)", count)).small().weak());

                            // Remove segment button
                            if ui
                                .small_button("✕")
                                .on_hover_text(t::remove_segment())
                                .clicked()
                            {
                                action = BookmarkAction::RemoveSegment(segment.indices.clone());
                            }
                        });

                        ui.add_space(4.0);
                    }

                    ui.add_space(12.0);

                    // Clear all bookmarks button
                    if ui
                        .button(
                            RichText::new(t::clear_all_bookmarks())
                                .color(Color32::from_rgb(244, 67, 54)),
                        )
                        .clicked()
                    {
                        action = BookmarkAction::ClearAll;
                    }
                }

                ui.add_space(8.0);
            });

        action
    }

    /// Truncate content for preview
    #[allow(dead_code)]
    fn truncate_content(content: &str, max_len: usize) -> String {
        let trimmed = content.trim();
        if trimmed.len() <= max_len {
            trimmed.to_string()
        } else {
            format!("{}...", &trimmed[..max_len])
        }
    }
}
