//! Close confirmation dialog
//!
//! Shows a dialog asking the user whether to exit or minimize to tray when closing the application.

use crate::i18n::Translations as t;
use egui::{Align, Layout, RichText};

/// Result of the close dialog interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDialogResult {
    /// User chose to exit the application
    Exit,
    /// User chose to minimize to tray
    MinimizeToTray,
    /// User cancelled the close operation
    Cancel,
}

/// Close confirmation dialog
pub struct CloseDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Whether "remember my choice" is checked
    pub remember_choice: bool,
}

impl Default for CloseDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl CloseDialog {
    /// Create a new close dialog
    pub fn new() -> Self {
        Self {
            open: false,
            remember_choice: false,
        }
    }

    /// Show the dialog (call this to open it)
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.remember_choice = false;
    }

    /// Render the close dialog
    /// Returns Some(result) if user made a choice, None if dialog is still open
    pub fn show(&mut self, ctx: &egui::Context) -> Option<CloseDialogResult> {
        if !self.open {
            return None;
        }

        let mut result = None;

        egui::Window::new(t::close_dialog_title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size(egui::vec2(380.0, 200.0)) // 固定窗口大小
            .show(ctx, |ui| {
                // 使用更紧凑的间距设置
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);

                    // Message
                    ui.label(RichText::new(t::close_dialog_message()).size(14.0));

                    ui.add_space(8.0);

                    // 使用网格布局来更好地控制按钮
                    egui::Grid::new("close_dialog_buttons")
                        .num_columns(1)
                        .spacing([0.0, 4.0])
                        .show(ui, |ui| {
                            // Exit button
                            let exit_button = egui::Button::new(
                                RichText::new(format!("🚪 {}", t::close_dialog_exit())).size(14.0),
                            )
                            .min_size(egui::vec2(350.0, 28.0));

                            if ui.add(exit_button).clicked() {
                                result = Some(CloseDialogResult::Exit);
                                self.open = false;
                            }
                            ui.end_row();

                            // Minimize to tray button
                            let minimize_button = egui::Button::new(
                                RichText::new(format!("📥 {}", t::close_dialog_minimize()))
                                    .size(14.0),
                            )
                            .min_size(egui::vec2(350.0, 28.0));

                            if ui.add(minimize_button).clicked() {
                                result = Some(CloseDialogResult::MinimizeToTray);
                                self.open = false;
                            }
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    // 水平布局放置复选框和取消按钮
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.remember_choice, t::close_dialog_remember());

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(t::close_dialog_cancel()).clicked() {
                                result = Some(CloseDialogResult::Cancel);
                                self.open = false;
                            }
                        });
                    });
                });
            });

        result
    }

    /// Check if "remember choice" was checked
    pub fn should_remember(&self) -> bool {
        self.remember_choice
    }
}
