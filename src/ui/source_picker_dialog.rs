//! Source Picker Dialog - Unified data source selection
//!
//! A modern dialog that combines local files, Android devices, and remote streams
//! into a single, tabbed interface with a clean design.

use crate::android_logcat::{AndroidDevice, ConnectionType};
use crate::i18n::Translations as I18n;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use encoding_rs::Encoding;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// Unified Style Constants
// ============================================================================

/// Dialog styling constants for consistent UI
mod style {
    use super::*;

    // Colors for dark theme
    pub const DARK_BG: Color32 = Color32::from_rgb(30, 30, 30);
    pub const DARK_CARD_BG: Color32 = Color32::from_rgb(45, 45, 45);
    pub const DARK_INPUT_BG: Color32 = Color32::from_rgb(38, 38, 38);
    pub const DARK_HOVER_BG: Color32 = Color32::from_rgb(55, 55, 55);
    pub const DARK_SELECTED_BG: Color32 = Color32::from_rgb(50, 70, 110);
    pub const DARK_BORDER: Color32 = Color32::from_rgb(70, 70, 70);
    pub const DARK_TEXT: Color32 = Color32::from_rgb(230, 230, 230);
    pub const DARK_TEXT_DIM: Color32 = Color32::from_rgb(140, 140, 140);

    // Colors for light theme
    pub const LIGHT_BG: Color32 = Color32::from_rgb(248, 248, 248);
    pub const LIGHT_CARD_BG: Color32 = Color32::from_rgb(255, 255, 255);
    pub const LIGHT_INPUT_BG: Color32 = Color32::from_rgb(250, 250, 250);
    pub const LIGHT_HOVER_BG: Color32 = Color32::from_rgb(235, 235, 235);
    pub const LIGHT_SELECTED_BG: Color32 = Color32::from_rgb(210, 230, 255);
    pub const LIGHT_BORDER: Color32 = Color32::from_rgb(210, 210, 210);
    pub const LIGHT_TEXT: Color32 = Color32::from_rgb(30, 30, 30);
    pub const LIGHT_TEXT_DIM: Color32 = Color32::from_rgb(110, 110, 110);

    // Accent colors
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(66, 133, 244);
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 199, 89);
    pub const ACCENT_RED: Color32 = Color32::from_rgb(255, 69, 58);

    // Sizing constants (as integers for egui API)
    pub const CARD_ROUNDING: u8 = 10;
    pub const INPUT_ROUNDING: u8 = 8;
    pub const BUTTON_ROUNDING: u8 = 6;
    pub const CARD_PADDING: i8 = 14;
    pub const INPUT_PADDING_H: f32 = 12.0;
    pub const ITEM_SPACING: f32 = 8.0;
    pub const SECTION_SPACING: f32 = 16.0;

    /// Get theme colors based on current visuals
    pub struct ThemeColors {
        pub bg: Color32,
        pub card_bg: Color32,
        pub input_bg: Color32,
        pub hover_bg: Color32,
        pub selected_bg: Color32,
        pub border: Color32,
        pub text: Color32,
        pub text_dim: Color32,
    }

    impl ThemeColors {
        pub fn from_visuals(visuals: &egui::Visuals) -> Self {
            if visuals.dark_mode {
                Self {
                    bg: DARK_BG,
                    card_bg: DARK_CARD_BG,
                    input_bg: DARK_INPUT_BG,
                    hover_bg: DARK_HOVER_BG,
                    selected_bg: DARK_SELECTED_BG,
                    border: DARK_BORDER,
                    text: DARK_TEXT,
                    text_dim: DARK_TEXT_DIM,
                }
            } else {
                Self {
                    bg: LIGHT_BG,
                    card_bg: LIGHT_CARD_BG,
                    input_bg: LIGHT_INPUT_BG,
                    hover_bg: LIGHT_HOVER_BG,
                    selected_bg: LIGHT_SELECTED_BG,
                    border: LIGHT_BORDER,
                    text: LIGHT_TEXT,
                    text_dim: LIGHT_TEXT_DIM,
                }
            }
        }
    }
}

use style::ThemeColors;

// ============================================================================
// Public Types
// ============================================================================

/// Actions that can be triggered by the source picker
#[derive(Debug, Clone)]
pub enum SourcePickerAction {
    /// User selected a file to open
    OpenFile(PathBuf, Option<&'static Encoding>),
    /// User selected an Android device
    OpenAndroidDevice(AndroidDevice),
    /// User wants to refresh Android devices
    RefreshAndroidDevices,
    /// User wants to connect to a TCP device
    ConnectAndroidTcp(String),
    /// User wants to disconnect an Android device
    DisconnectAndroidDevice(String),
    /// User cancelled the dialog
    Cancel,
    /// No action
    None,
}

/// Tab types for the source picker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceTab {
    #[default]
    LocalFiles,
    AndroidDevices,
}

/// File suggestion item
#[derive(Debug, Clone)]
struct FileSuggestion {
    path: PathBuf,
    display_name: String,
    is_dir: bool,
}

// ============================================================================
// Source Picker Dialog
// ============================================================================

/// Source picker dialog with tabbed interface
pub struct SourcePickerDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Current active tab
    pub active_tab: SourceTab,
    /// Current input path for file selection
    input_path: String,
    /// List of suggestions based on input
    suggestions: Vec<FileSuggestion>,
    /// Selected suggestion index
    selected_index: usize,
    /// Recent files list
    recent_files: Vec<PathBuf>,
    /// Whether to focus input next frame
    focus_input: bool,
    /// Search query for filtering suggestions
    search_query: String,
    /// Selected encoding (None for auto-detect)
    selected_encoding: Option<&'static Encoding>,
    /// Index of selected encoding in dropdown
    encoding_index: usize,
    /// Android devices list
    android_devices: Vec<AndroidDevice>,
    /// TCP connect address
    tcp_connect_address: String,
    /// TCP connection error
    tcp_connect_error: Option<String>,
    /// Whether to show TCP connect input
    show_tcp_connect: bool,
}

impl Default for SourcePickerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SourcePickerDialog {
    /// Create a new source picker dialog
    pub fn new() -> Self {
        Self {
            open: false,
            active_tab: SourceTab::LocalFiles,
            input_path: String::new(),
            suggestions: Vec::new(),
            selected_index: 0,
            recent_files: Vec::new(),
            focus_input: true,
            search_query: String::new(),
            selected_encoding: None,
            encoding_index: 0,
            android_devices: Vec::new(),
            tcp_connect_address: String::new(),
            tcp_connect_error: None,
            show_tcp_connect: false,
        }
    }

    /// Get available encodings
    fn get_encodings() -> Vec<(&'static str, Option<&'static Encoding>)> {
        vec![
            ("Auto Detect", None),
            ("UTF-8", Some(encoding_rs::UTF_8)),
            ("GBK", Some(encoding_rs::GBK)),
            ("GB18030", Some(encoding_rs::GB18030)),
            ("Big5", Some(encoding_rs::BIG5)),
            ("Shift-JIS", Some(encoding_rs::SHIFT_JIS)),
            ("EUC-KR", Some(encoding_rs::EUC_KR)),
            ("ISO-8859-1", Some(encoding_rs::WINDOWS_1252)),
            ("Windows-1252", Some(encoding_rs::WINDOWS_1252)),
        ]
    }

    /// Set recent files list
    pub fn set_recent_files(&mut self, files: Vec<PathBuf>) {
        self.recent_files = files;
    }

    /// Update Android devices
    pub fn update_android_devices(&mut self, devices: Vec<AndroidDevice>) {
        self.android_devices = devices;
    }

    /// Set TCP connection error
    pub fn set_tcp_connect_error(&mut self, error: Option<String>) {
        self.tcp_connect_error = error;
    }

    /// Clear TCP connect state
    pub fn clear_tcp_connect(&mut self) {
        self.tcp_connect_address.clear();
        self.tcp_connect_error = None;
        self.show_tcp_connect = false;
    }

    /// Open the dialog with specific tab
    pub fn show_dialog(&mut self) {
        self.show_dialog_tab(SourceTab::LocalFiles);
    }

    /// Open the dialog with specific tab
    pub fn show_dialog_tab(&mut self, tab: SourceTab) {
        self.open = true;
        self.active_tab = tab;
        self.input_path.clear();
        self.search_query.clear();
        self.selected_index = 0;
        self.focus_input = true;
        self.selected_encoding = None;
        self.encoding_index = 0;
        self.tcp_connect_address.clear();
        self.tcp_connect_error = None;
        self.show_tcp_connect = false;
        self.update_suggestions();
    }

    /// Show the source picker dialog
    pub fn show(&mut self, ctx: &egui::Context) -> SourcePickerAction {
        if !self.open {
            return SourcePickerAction::None;
        }

        let colors = ThemeColors::from_visuals(&ctx.style().visuals);
        let mut action = SourcePickerAction::None;
        let mut needs_update = false;

        let mut window_open = self.open;

        // Window frame with custom styling
        let window_frame = egui::Frame::new()
            .fill(colors.bg)
            .stroke(Stroke::new(1.0, colors.border))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::same(0));

        egui::Window::new(I18n::open_source_dialog_title())
            .collapsible(false)
            .resizable(false)
            .fixed_size([680.0, 520.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(window_frame)
            .open(&mut window_open)
            .show(ctx, |ui| {
                // Apply spacing for this window
                ui.spacing_mut().item_spacing = Vec2::splat(style::ITEM_SPACING);

                ui.vertical(|ui| {
                    // Tab bar
                    ui.add_space(style::SECTION_SPACING);
                    self.render_tab_bar(ui, &colors);
                    ui.add_space(style::SECTION_SPACING);

                    // Content area with card styling
                    egui::Frame::new()
                        .fill(colors.card_bg)
                        .corner_radius(egui::CornerRadius::same(style::CARD_ROUNDING))
                        .stroke(Stroke::new(1.0, colors.border))
                        .inner_margin(egui::Margin::same(style::CARD_PADDING))
                        .outer_margin(egui::Margin::symmetric(style::SECTION_SPACING as i8, 0))
                        .show(ui, |ui| match self.active_tab {
                            SourceTab::LocalFiles => {
                                action =
                                    self.render_local_files_tab(ui, &colors, &mut needs_update);
                            }
                            SourceTab::AndroidDevices => {
                                action = self.render_android_devices_tab(ui, &colors);
                                if matches!(action, SourcePickerAction::OpenAndroidDevice(_)) {
                                    self.open = false;
                                }
                            }
                        });

                    ui.add_space(style::SECTION_SPACING);
                });
            });

        // Check if window was closed
        if !window_open && self.open {
            self.open = false;
            if matches!(action, SourcePickerAction::None) {
                action = SourcePickerAction::Cancel;
            }
        }

        // Update suggestions if directory was navigated
        if needs_update {
            self.update_suggestions();
        }

        action
    }

    // ========================================================================
    // Tab Bar
    // ========================================================================

    /// Render the modern tab bar
    fn render_tab_bar(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        ui.horizontal(|ui| {
            ui.add_space(style::SECTION_SPACING);

            // Local Files Tab
            let files_selected = self.active_tab == SourceTab::LocalFiles;
            if self.render_tab_button(
                ui,
                "📁",
                I18n::local_files_tab(),
                files_selected,
                None,
                colors,
            ) {
                self.active_tab = SourceTab::LocalFiles;
                self.focus_input = true;
            }

            ui.add_space(style::ITEM_SPACING);

            // Android Devices Tab
            let android_selected = self.active_tab == SourceTab::AndroidDevices;
            let online_count = self.android_devices.iter().filter(|d| d.is_online).count();
            let badge = if online_count > 0 {
                Some(online_count)
            } else {
                None
            };
            if self.render_tab_button(
                ui,
                "📱",
                I18n::android_devices_tab(),
                android_selected,
                badge,
                colors,
            ) {
                self.active_tab = SourceTab::AndroidDevices;
            }

            ui.add_space(style::SECTION_SPACING);
        });
    }

    /// Render a single tab button with icon, text, and optional badge
    fn render_tab_button(
        &self,
        ui: &mut egui::Ui,
        icon: &str,
        label: &str,
        selected: bool,
        badge: Option<usize>,
        colors: &ThemeColors,
    ) -> bool {
        let padding = Vec2::new(16.0, 10.0);
        let rounding = egui::CornerRadius::same(style::BUTTON_ROUNDING);

        // Calculate size
        let icon_text = format!("{}  {}", icon, label);
        let badge_text = badge.map(|n| format!(" {}", n));
        let full_text = if let Some(ref b) = badge_text {
            format!("{}{}", icon_text, b)
        } else {
            icon_text.clone()
        };

        let galley = ui.painter().layout_no_wrap(
            full_text.clone(),
            egui::FontId::proportional(14.0),
            colors.text,
        );
        let desired_size = galley.size() + padding * 2.0;

        // Allocate space with click sense
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        // Determine colors based on state
        let (bg_color, text_color) = if selected {
            (style::ACCENT_BLUE, Color32::WHITE)
        } else if response.hovered() {
            (colors.hover_bg, colors.text)
        } else {
            (Color32::TRANSPARENT, colors.text_dim)
        };

        // Draw background
        ui.painter().rect_filled(rect, rounding, bg_color);

        // Draw text
        let text_pos = rect.min + padding;
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            &icon_text,
            egui::FontId::proportional(14.0),
            text_color,
        );

        // Draw badge
        if let Some(count) = badge {
            let badge_str = format!("{}", count);
            let badge_galley = ui.painter().layout_no_wrap(
                badge_str.clone(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
            let badge_size = badge_galley.size() + Vec2::new(8.0, 4.0);
            let badge_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.max.x - badge_size.x - padding.x,
                    rect.center().y - badge_size.y / 2.0,
                ),
                badge_size,
            );
            ui.painter().rect_filled(
                badge_rect,
                egui::CornerRadius::same((badge_size.y / 2.0) as u8),
                style::ACCENT_GREEN,
            );
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                &badge_str,
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );
        }

        // Cursor change on hover
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response.clicked()
    }

    // ========================================================================
    // Local Files Tab
    // ========================================================================

    fn render_local_files_tab(
        &mut self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        needs_update: &mut bool,
    ) -> SourcePickerAction {
        // Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(I18n::local_files_tab())
                    .strong()
                    .size(15.0)
                    .color(colors.text),
            );
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(I18n::start_typing_hint())
                .size(12.0)
                .color(colors.text_dim),
        );
        ui.add_space(style::SECTION_SPACING);

        // Path input section
        let mut action = self.render_path_input(ui, colors, needs_update);

        ui.add_space(style::ITEM_SPACING);

        // Encoding selection
        self.render_encoding_selector(ui, colors);

        ui.add_space(style::SECTION_SPACING);

        // Separator line
        let sep_rect = ui.available_rect_before_wrap();
        ui.painter().hline(
            sep_rect.x_range(),
            sep_rect.top(),
            Stroke::new(1.0, colors.border),
        );
        ui.add_space(style::SECTION_SPACING);

        // File list
        action = self.render_file_list(ui, colors, needs_update, action);

        action
    }

    /// Render the path input field with browse button
    fn render_path_input(
        &mut self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        needs_update: &mut bool,
    ) -> SourcePickerAction {
        let mut action = SourcePickerAction::None;

        ui.horizontal(|ui| {
            // Folder icon
            ui.label(RichText::new("📂").size(20.0));
            ui.add_space(8.0);

            // Input field with Frame styling - use allocate_space to reserve space for browse button
            let total_width = ui.available_width();
            let browse_button_width = 110.0; // Button width + spacing
            let input_width = total_width - browse_button_width;

            let frame_response = egui::Frame::new()
                .fill(colors.input_bg)
                .stroke(Stroke::new(1.5, colors.border))
                .corner_radius(egui::CornerRadius::same(style::INPUT_ROUNDING))
                .inner_margin(egui::Margin::symmetric(style::INPUT_PADDING_H as i8, 8))
                .show(ui, |ui| {
                    // Set a fixed width for the input area
                    ui.set_width(input_width - 24.0); // Subtract frame margins
                    ui.add(
                        egui::TextEdit::singleline(&mut self.input_path)
                            .hint_text(
                                RichText::new(I18n::file_path_input_hint()).color(colors.text_dim),
                            )
                            .font(egui::FontId::proportional(14.0))
                            .text_color(colors.text)
                            .frame(false)
                            .desired_width(ui.available_width()),
                    )
                });

            let response = frame_response.inner;

            // Focus handling
            if self.focus_input {
                response.request_focus();
                self.focus_input = false;
            }

            // Handle input changes
            if response.changed() {
                self.search_query = self.input_path.clone();
                self.selected_index = 0;
                *needs_update = true;
            }

            // Handle Ctrl/Cmd+A to select all
            if response.has_focus()
                && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
            {
                let text_edit_state = egui::TextEdit::load_state(ui.ctx(), response.id);
                if let Some(mut state) = text_edit_state {
                    let ccursor_range = egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.input_path.len()),
                    );
                    state.cursor.set_char_range(Some(ccursor_range));
                    egui::TextEdit::store_state(ui.ctx(), response.id, state);
                }
            }

            // Keyboard navigation
            if response.has_focus() {
                action = self.handle_keyboard_navigation(ui, needs_update);
            }

            ui.add_space(8.0);

            // Browse button with primary styling
            let browse_btn =
                egui::Button::new(RichText::new(I18n::browse_button()).color(Color32::WHITE))
                    .fill(style::ACCENT_BLUE)
                    .stroke(Stroke::NONE)
                    .corner_radius(egui::CornerRadius::same(style::BUTTON_ROUNDING))
                    .min_size(Vec2::new(95.0, 36.0));

            if ui.add(browse_btn).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Log files", &["log", "txt", "json"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    action = SourcePickerAction::OpenFile(path, self.selected_encoding);
                    self.open = false;
                }
            }
        });

        action
    }

    /// Render the encoding selector
    fn render_encoding_selector(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(I18n::file_encoding())
                    .color(colors.text)
                    .size(13.0),
            );
            ui.add_space(4.0);

            let encodings = Self::get_encodings();
            let current_label = encodings[self.encoding_index].0;

            egui::ComboBox::from_id_salt("encoding_selector")
                .selected_text(current_label)
                .width(150.0)
                .show_ui(ui, |ui| {
                    for (idx, (label, encoding)) in encodings.iter().enumerate() {
                        if ui
                            .selectable_value(&mut self.encoding_index, idx, *label)
                            .clicked()
                        {
                            self.selected_encoding = *encoding;
                        }
                    }
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new(I18n::file_encoding_hint())
                    .color(colors.text_dim)
                    .size(11.0),
            );
        });
    }

    /// Render the file list (suggestions or recent files)
    fn render_file_list(
        &mut self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        needs_update: &mut bool,
        mut action: SourcePickerAction,
    ) -> SourcePickerAction {
        // Clone data to avoid borrow conflicts
        let suggestions_clone = self.suggestions.clone();
        let recent_files_clone = self.recent_files.clone();
        let selected_encoding = self.selected_encoding;
        let selected_index = self.selected_index;
        let search_query = self.search_query.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(260.0)
            .show(ui, |ui| {
                if suggestions_clone.is_empty() && recent_files_clone.is_empty() {
                    // Empty state
                    self.render_empty_state(ui, colors, "📂", I18n::start_typing_hint());
                } else if suggestions_clone.is_empty() && !recent_files_clone.is_empty() {
                    // Show recent files
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⏱").size(16.0));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(I18n::recent_files_label())
                                .strong()
                                .size(13.0)
                                .color(colors.text),
                        );
                    });
                    ui.add_space(style::ITEM_SPACING);

                    for (idx, path) in recent_files_clone.iter().enumerate() {
                        let is_selected = idx == selected_index && search_query.is_empty();
                        if let Some(new_action) =
                            self.render_file_card(ui, path, is_selected, colors, selected_encoding)
                        {
                            action = new_action;
                            self.open = false;
                        }
                    }
                } else {
                    // Show suggestions with parent directory option
                    let current_path = PathBuf::from(&self.input_path);
                    let show_parent = self.should_show_parent_option(&current_path);

                    if show_parent {
                        let parent_path = self.get_parent_path(&current_path);
                        if let Some(parent) = parent_path {
                            let is_selected = selected_index == 0;
                            if self.render_directory_item(ui, &parent, "../", is_selected, colors) {
                                self.input_path = format!("{}/", parent.display());
                                self.search_query = self.input_path.clone();
                                self.selected_index = 0;
                                *needs_update = true;
                            }
                        }
                    }

                    let offset = if show_parent { 1 } else { 0 };
                    for (idx, suggestion) in suggestions_clone.iter().enumerate() {
                        let is_selected = idx + offset == selected_index;

                        if suggestion.is_dir {
                            if self.render_directory_item(
                                ui,
                                &suggestion.path,
                                &suggestion.display_name,
                                is_selected,
                                colors,
                            ) {
                                self.input_path = format!("{}/", suggestion.path.display());
                                self.search_query = self.input_path.clone();
                                self.selected_index = 0;
                                *needs_update = true;
                            }
                        } else if let Some(new_action) = self.render_file_card(
                            ui,
                            &suggestion.path,
                            is_selected,
                            colors,
                            selected_encoding,
                        ) {
                            action = new_action;
                            self.open = false;
                        }
                    }
                }
            });

        action
    }

    /// Check if we should show the parent directory option
    fn should_show_parent_option(&self, path: &Path) -> bool {
        if path.exists() && path.is_dir() {
            path.parent().is_some()
        } else if let Some(parent) = path.parent() {
            parent.parent().is_some()
        } else {
            false
        }
    }

    /// Get the parent path for navigation
    fn get_parent_path(&self, path: &Path) -> Option<PathBuf> {
        if path.exists() && path.is_dir() {
            path.parent().map(|p| p.to_path_buf())
        } else {
            path.parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
        }
    }

    /// Render empty state with icon and message
    fn render_empty_state(
        &self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        icon: &str,
        message: &str,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new(icon).size(48.0).color(colors.text_dim));
            ui.add_space(8.0);
            ui.label(RichText::new(message).color(colors.text_dim).size(13.0));
        });
    }

    /// Render a file card (for files)
    fn render_file_card(
        &self,
        ui: &mut egui::Ui,
        path: &Path,
        is_selected: bool,
        colors: &ThemeColors,
        selected_encoding: Option<&'static Encoding>,
    ) -> Option<SourcePickerAction> {
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let bg_color = if is_selected {
            colors.selected_bg
        } else {
            colors.input_bg
        };
        let stroke_color = if is_selected {
            style::ACCENT_BLUE
        } else {
            colors.border
        };

        let frame_response = egui::Frame::new()
            .fill(bg_color)
            .stroke(Stroke::new(1.0, stroke_color))
            .corner_radius(egui::CornerRadius::same(style::INPUT_ROUNDING))
            .inner_margin(egui::Margin::same(12))
            .outer_margin(egui::Margin::symmetric(0, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📄").size(22.0));
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&display_name)
                                .strong()
                                .size(13.0)
                                .color(colors.text),
                        );
                        let path_str = path.display().to_string();
                        ui.label(RichText::new(path_str).size(11.0).color(colors.text_dim));
                    });
                });
            })
            .response;

        let interact = frame_response.interact(egui::Sense::click());
        if interact.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if interact.clicked() {
            Some(SourcePickerAction::OpenFile(
                path.to_path_buf(),
                selected_encoding,
            ))
        } else {
            None
        }
    }

    /// Render a directory item
    fn render_directory_item(
        &self,
        ui: &mut egui::Ui,
        path: &Path,
        display_name: &str,
        is_selected: bool,
        colors: &ThemeColors,
    ) -> bool {
        let bg_color = if is_selected {
            colors.selected_bg
        } else {
            colors.input_bg
        };
        let stroke_color = if is_selected {
            style::ACCENT_BLUE
        } else {
            colors.border
        };

        let frame_response = egui::Frame::new()
            .fill(bg_color)
            .stroke(Stroke::new(1.0, stroke_color))
            .corner_radius(egui::CornerRadius::same(style::INPUT_ROUNDING))
            .inner_margin(egui::Margin::same(10))
            .outer_margin(egui::Margin::symmetric(0, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📁").size(18.0));
                    ui.add_space(8.0);
                    ui.label(RichText::new(display_name).size(13.0).color(colors.text));
                });
            })
            .response;

        let interact = frame_response.interact(egui::Sense::click());
        if interact.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let clicked = interact.clicked();
        interact.on_hover_text(path.display().to_string());
        clicked
    }

    // ========================================================================
    // Android Devices Tab
    // ========================================================================

    fn render_android_devices_tab(
        &mut self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
    ) -> SourcePickerAction {
        let mut action = SourcePickerAction::None;

        // Header with actions
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(I18n::connected_devices())
                    .strong()
                    .size(15.0)
                    .color(colors.text),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Refresh button
                if self.render_icon_button(ui, "🔄", I18n::refresh(), colors) {
                    action = SourcePickerAction::RefreshAndroidDevices;
                }

                ui.add_space(8.0);

                // TCP connect toggle
                let tcp_label = if self.show_tcp_connect { "✕" } else { "📶" };
                let tcp_tooltip = if self.show_tcp_connect {
                    I18n::cancel()
                } else {
                    I18n::connect_tcp()
                };
                if self.render_icon_button(ui, tcp_label, tcp_tooltip, colors) {
                    self.show_tcp_connect = !self.show_tcp_connect;
                    if !self.show_tcp_connect {
                        self.tcp_connect_address.clear();
                        self.tcp_connect_error = None;
                    }
                }
            });
        });

        ui.add_space(style::SECTION_SPACING);

        // TCP Connect panel
        if self.show_tcp_connect {
            action = self.render_tcp_connect_panel(ui, colors, action);
            ui.add_space(style::SECTION_SPACING);
        }

        // Separator line
        let sep_rect = ui.available_rect_before_wrap();
        ui.painter().hline(
            sep_rect.x_range(),
            sep_rect.top(),
            Stroke::new(1.0, colors.border),
        );
        ui.add_space(style::SECTION_SPACING);

        // Device list
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(260.0)
            .show(ui, |ui| {
                if self.android_devices.is_empty() {
                    self.render_empty_state(ui, colors, "📱", I18n::no_devices_connected());
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(I18n::connect_device_hint())
                                .color(colors.text_dim)
                                .size(12.0),
                        );
                    });
                } else {
                    for device in self.android_devices.clone() {
                        if let Some(new_action) = self.render_device_card(ui, &device, colors) {
                            action = new_action;
                        }
                    }
                }
            });

        action
    }

    /// Render the TCP connect panel
    fn render_tcp_connect_panel(
        &mut self,
        ui: &mut egui::Ui,
        colors: &ThemeColors,
        mut action: SourcePickerAction,
    ) -> SourcePickerAction {
        egui::Frame::new()
            .fill(colors.input_bg)
            .stroke(Stroke::new(1.0, colors.border))
            .corner_radius(egui::CornerRadius::same(style::INPUT_ROUNDING))
            .inner_margin(egui::Margin::same(style::CARD_PADDING))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📶").size(18.0));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(I18n::connect_via_tcp())
                            .strong()
                            .color(colors.text),
                    );
                });

                ui.add_space(style::ITEM_SPACING);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(I18n::ip_address())
                            .color(colors.text)
                            .size(13.0),
                    );
                    ui.add_space(4.0);

                    // TCP address input using Frame styling
                    let tcp_frame = egui::Frame::new()
                        .fill(colors.card_bg)
                        .stroke(Stroke::new(1.0, colors.border))
                        .corner_radius(egui::CornerRadius::same(style::INPUT_ROUNDING))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.tcp_connect_address)
                                    .hint_text(
                                        RichText::new("192.168.1.100:5555").color(colors.text_dim),
                                    )
                                    .font(egui::FontId::proportional(13.0))
                                    .text_color(colors.text)
                                    .frame(false)
                                    .desired_width(160.0),
                            )
                        });

                    let input_response = tcp_frame.inner;

                    if input_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        if !self.tcp_connect_address.is_empty() {
                            action = SourcePickerAction::ConnectAndroidTcp(
                                self.tcp_connect_address.clone(),
                            );
                        }
                    }

                    ui.add_space(8.0);

                    // Connect button
                    let connect_btn =
                        egui::Button::new(RichText::new(I18n::connect()).color(Color32::WHITE))
                            .fill(style::ACCENT_BLUE)
                            .stroke(Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(style::BUTTON_ROUNDING))
                            .min_size(Vec2::new(80.0, 32.0));

                    if ui.add(connect_btn).clicked() && !self.tcp_connect_address.is_empty() {
                        action =
                            SourcePickerAction::ConnectAndroidTcp(self.tcp_connect_address.clone());
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    RichText::new(I18n::tcp_address_hint())
                        .color(colors.text_dim)
                        .size(11.0),
                );

                if let Some(ref error) = self.tcp_connect_error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(error).color(style::ACCENT_RED).size(12.0));
                }
            });

        action
    }

    /// Render an icon button with tooltip
    fn render_icon_button(
        &self,
        ui: &mut egui::Ui,
        icon: &str,
        tooltip: &str,
        colors: &ThemeColors,
    ) -> bool {
        let button = egui::Button::new(RichText::new(icon).size(14.0))
            .fill(colors.input_bg)
            .stroke(Stroke::new(1.0, colors.border))
            .corner_radius(egui::CornerRadius::same(style::BUTTON_ROUNDING))
            .min_size(Vec2::new(32.0, 32.0));

        let response = ui.add(button);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        response.on_hover_text(tooltip).clicked()
    }

    /// Render a device card
    fn render_device_card(
        &self,
        ui: &mut egui::Ui,
        device: &AndroidDevice,
        colors: &ThemeColors,
    ) -> Option<SourcePickerAction> {
        let mut action = None;

        let (status_color, status_text) = if device.is_online {
            (style::ACCENT_GREEN, I18n::online())
        } else {
            (colors.text_dim, I18n::device_offline())
        };

        let conn_icon = match device.connection_type {
            ConnectionType::Usb => "🔌",
            ConnectionType::Tcp => "📶",
            ConnectionType::Unknown => "❓",
        };

        let card_bg = if device.is_online {
            colors.card_bg
        } else {
            colors.input_bg
        };
        let card_stroke = if device.is_online {
            style::ACCENT_GREEN
        } else {
            colors.border
        };

        egui::Frame::new()
            .fill(card_bg)
            .stroke(Stroke::new(1.5, card_stroke))
            .corner_radius(egui::CornerRadius::same(style::CARD_ROUNDING))
            .inner_margin(egui::Margin::same(style::CARD_PADDING))
            .outer_margin(egui::Margin::symmetric(0, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Device icon
                    ui.vertical(|ui| {
                        ui.label(RichText::new("📱").size(32.0));
                        ui.label(RichText::new(conn_icon).size(12.0).color(colors.text_dim));
                    });

                    ui.add_space(14.0);

                    // Device info
                    ui.vertical(|ui| {
                        let device_name = if device.model != "Unknown" {
                            &device.model
                        } else {
                            &device.serial
                        };

                        ui.label(
                            RichText::new(device_name)
                                .strong()
                                .size(14.0)
                                .color(colors.text),
                        );

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("●").color(status_color).size(10.0));
                            ui.add_space(2.0);
                            ui.label(RichText::new(status_text).size(12.0).color(colors.text_dim));
                        });

                        if device.product != "Unknown" {
                            ui.label(
                                RichText::new(&device.product)
                                    .size(11.0)
                                    .color(colors.text_dim),
                            );
                        }
                        ui.label(
                            RichText::new(&device.serial)
                                .size(11.0)
                                .color(colors.text_dim),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if device.is_online {
                            // View Logcat button
                            let view_btn = egui::Button::new(
                                RichText::new(I18n::view_logcat()).color(Color32::WHITE),
                            )
                            .fill(style::ACCENT_BLUE)
                            .stroke(Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(style::BUTTON_ROUNDING))
                            .min_size(Vec2::new(100.0, 32.0));

                            if ui.add(view_btn).clicked() {
                                action =
                                    Some(SourcePickerAction::OpenAndroidDevice(device.clone()));
                            }
                        }

                        if device.connection_type == ConnectionType::Tcp {
                            ui.add_space(8.0);
                            // Disconnect button
                            let disconnect_btn = egui::Button::new(
                                RichText::new(I18n::disconnect()).color(colors.text),
                            )
                            .fill(colors.input_bg)
                            .stroke(Stroke::new(1.0, colors.border))
                            .corner_radius(egui::CornerRadius::same(style::BUTTON_ROUNDING))
                            .min_size(Vec2::new(80.0, 32.0));

                            if ui.add(disconnect_btn).clicked() {
                                action = Some(SourcePickerAction::DisconnectAndroidDevice(
                                    device.serial.clone(),
                                ));
                            }
                        }
                    });
                });
            });

        action
    }

    // ========================================================================
    // Keyboard Navigation
    // ========================================================================

    fn handle_keyboard_navigation(
        &mut self,
        ui: &egui::Ui,
        needs_update: &mut bool,
    ) -> SourcePickerAction {
        let mut action = SourcePickerAction::None;

        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let current_path = PathBuf::from(&self.input_path);
            let has_parent = self.should_show_parent_option(&current_path);

            if has_parent && self.selected_index == 0 {
                if let Some(parent) = self.get_parent_path(&current_path) {
                    self.input_path = format!("{}/", parent.display());
                    self.search_query = self.input_path.clone();
                    self.selected_index = 0;
                    *needs_update = true;
                }
            } else if !self.suggestions.is_empty() {
                let suggestion_idx = if has_parent {
                    self.selected_index.saturating_sub(1)
                } else {
                    self.selected_index
                };

                if suggestion_idx < self.suggestions.len() {
                    let selected = &self.suggestions[suggestion_idx];
                    if selected.is_dir {
                        self.input_path = format!("{}/", selected.path.display());
                        self.search_query = self.input_path.clone();
                        self.selected_index = 0;
                        *needs_update = true;
                    } else {
                        action = SourcePickerAction::OpenFile(
                            selected.path.clone(),
                            self.selected_encoding,
                        );
                        self.open = false;
                    }
                }
            } else if !self.input_path.is_empty() {
                let path = PathBuf::from(&self.input_path);
                if path.exists() && path.is_file() {
                    action = SourcePickerAction::OpenFile(path, self.selected_encoding);
                    self.open = false;
                }
            }
        } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            action = SourcePickerAction::Cancel;
            self.open = false;
        } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            if !self.suggestions.is_empty() {
                let current_path = PathBuf::from(&self.input_path);
                let has_parent = self.should_show_parent_option(&current_path);
                let max_index = if has_parent {
                    self.suggestions.len()
                } else {
                    self.suggestions.len().saturating_sub(1)
                };
                self.selected_index = (self.selected_index + 1).min(max_index);
            }
        } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            }
        } else if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
            let current_path = PathBuf::from(&self.input_path);
            let has_parent = self.should_show_parent_option(&current_path);

            if has_parent && self.selected_index == 0 {
                if let Some(parent) = self.get_parent_path(&current_path) {
                    self.input_path = format!("{}/", parent.display());
                    self.search_query = self.input_path.clone();
                    self.selected_index = 0;
                    *needs_update = true;
                }
            } else if !self.suggestions.is_empty() {
                let suggestion_idx = if has_parent {
                    self.selected_index.saturating_sub(1)
                } else {
                    self.selected_index
                };

                if suggestion_idx < self.suggestions.len() {
                    let selected = &self.suggestions[suggestion_idx];
                    if selected.is_dir {
                        self.input_path = format!("{}/", selected.path.display());
                    } else {
                        self.input_path = selected.path.display().to_string();
                    }
                    self.search_query = self.input_path.clone();
                    self.selected_index = 0;
                    *needs_update = true;
                }
            }
        }

        action
    }

    // ========================================================================
    // Suggestions Management
    // ========================================================================

    fn update_suggestions(&mut self) {
        self.suggestions.clear();

        if self.input_path.is_empty() {
            return;
        }

        let path = PathBuf::from(&self.input_path);

        let (base_dir, pattern) = if path.exists() && path.is_dir() {
            (path.clone(), String::new())
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                (parent.to_path_buf(), file_name)
            } else if let Some(home) = dirs::home_dir() {
                (home, self.input_path.to_lowercase())
            } else {
                return;
            }
        } else if let Some(home) = dirs::home_dir() {
            (home, self.input_path.to_lowercase())
        } else {
            return;
        };

        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let entry_path = entry.path();
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy().to_lowercase();

                    if pattern.is_empty() || file_name_str.contains(&pattern) {
                        let is_dir = metadata.is_dir();
                        let is_log_file = !is_dir
                            && entry_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| matches!(ext, "log" | "txt" | "json"))
                                .unwrap_or(false);

                        if is_dir || is_log_file {
                            let display_name = if is_dir {
                                format!("{}/", file_name.to_string_lossy())
                            } else {
                                file_name.to_string_lossy().to_string()
                            };

                            self.suggestions.push(FileSuggestion {
                                path: entry_path,
                                display_name,
                                is_dir,
                            });
                        }
                    }
                }
            }
        }

        // Sort: directories first, then by name
        self.suggestions.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.display_name.cmp(&b.display_name),
        });

        // Limit suggestions
        self.suggestions.truncate(50);
    }
}
