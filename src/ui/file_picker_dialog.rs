//! File picker dialog with VSCode-style path input and suggestions

use crate::i18n::Translations as I18n;
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};

use encoding_rs::Encoding;

/// Actions that can be triggered by the file picker
#[derive(Debug, Clone)]
pub enum FilePickerAction {
    /// User selected a file to open (path, optional encoding)
    OpenFile(PathBuf, Option<&'static Encoding>),
    /// User cancelled the dialog
    Cancel,
    /// No action
    None,
}

/// File suggestion item
#[derive(Debug, Clone)]
struct FileSuggestion {
    /// Full path
    path: PathBuf,
    /// Display name (relative or file name)
    display_name: String,
    /// Whether this is a directory
    is_dir: bool,
}

/// File picker dialog with VSCode-style interface
pub struct FilePickerDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Current input path
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
}

impl Default for FilePickerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FilePickerDialog {
    /// Create a new file picker dialog
    pub fn new() -> Self {
        Self {
            open: false,
            input_path: String::new(),
            suggestions: Vec::new(),
            selected_index: 0,
            recent_files: Vec::new(),
            focus_input: true,
            search_query: String::new(),
            selected_encoding: None,
            encoding_index: 0,
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

    /// Open the dialog
    pub fn show_dialog(&mut self) {
        self.open = true;
        self.input_path.clear();
        self.search_query.clear();
        self.selected_index = 0;
        self.focus_input = true;
        self.selected_encoding = None;
        self.encoding_index = 0;
        self.update_suggestions();
    }

    /// Show the file picker dialog
    pub fn show(&mut self, ctx: &egui::Context) -> FilePickerAction {
        if !self.open {
            return FilePickerAction::None;
        }

        let mut action = FilePickerAction::None;
        let mut needs_update = false;

        let mut window_open = self.open;
        egui::Window::new(I18n::open_file_dialog_title())
            .collapsible(false)
            .resizable(false)
            .fixed_size([600.0, 400.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut window_open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(580.0);

                    // Path input with icon
                    ui.horizontal(|ui| {
                        ui.label("📁");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.input_path)
                                .hint_text(I18n::file_path_input_hint())
                                .desired_width(ui.available_width() - 80.0),
                        );

                        // Focus input on first frame
                        if self.focus_input {
                            response.request_focus();
                            self.focus_input = false;
                        }

                        // Handle input changes
                        if response.changed() {
                            self.search_query = self.input_path.clone();
                            self.selected_index = 0;
                            self.update_suggestions();
                        }

                        // Handle Ctrl/Cmd+A to select all text
                        if response.has_focus()
                            && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
                        {
                            // Select all text in the input field
                            let text_edit_state = egui::TextEdit::load_state(ctx, response.id);
                            if let Some(mut state) = text_edit_state {
                                let ccursor_range = egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(self.input_path.len()),
                                );
                                state.cursor.set_char_range(Some(ccursor_range));
                                egui::TextEdit::store_state(ctx, response.id, state);
                            }
                        }

                        // Handle keyboard navigation
                        if response.has_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                // Enter: select current suggestion or try to open input path
                                let current_path = PathBuf::from(&self.input_path);
                                let has_parent = if current_path.exists() && current_path.is_dir() {
                                    current_path.parent().is_some()
                                } else if let Some(parent) = current_path.parent() {
                                    parent.parent().is_some()
                                } else {
                                    false
                                };

                                // Check if "../" is selected
                                if has_parent && self.selected_index == 0 {
                                    let parent_path =
                                        if current_path.exists() && current_path.is_dir() {
                                            current_path.parent().unwrap()
                                        } else {
                                            current_path.parent().unwrap().parent().unwrap()
                                        };
                                    self.input_path = format!("{}/", parent_path.display());
                                    self.search_query = self.input_path.clone();
                                    self.selected_index = 0;
                                    self.update_suggestions();
                                } else if !self.suggestions.is_empty() {
                                    let suggestion_idx = if has_parent {
                                        self.selected_index.saturating_sub(1)
                                    } else {
                                        self.selected_index
                                    };

                                    if suggestion_idx < self.suggestions.len() {
                                        let selected = &self.suggestions[suggestion_idx];
                                        if selected.is_dir {
                                            // Navigate into directory
                                            self.input_path =
                                                format!("{}/", selected.path.display());
                                            self.search_query = self.input_path.clone();
                                            self.selected_index = 0;
                                            self.update_suggestions();
                                        } else {
                                            // Open file
                                            action = FilePickerAction::OpenFile(
                                                selected.path.clone(),
                                                self.selected_encoding,
                                            );
                                            self.open = false;
                                        }
                                    }
                                } else if !self.input_path.is_empty() {
                                    // Try to open the input path directly
                                    let path = PathBuf::from(&self.input_path);
                                    if path.exists() && path.is_file() {
                                        action = FilePickerAction::OpenFile(
                                            path,
                                            self.selected_encoding,
                                        );
                                        self.open = false;
                                    }
                                }
                            } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                action = FilePickerAction::Cancel;
                                self.open = false;
                            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                if !self.suggestions.is_empty() {
                                    // Check if we have a parent directory option
                                    let current_path = PathBuf::from(&self.input_path);
                                    let has_parent =
                                        if current_path.exists() && current_path.is_dir() {
                                            current_path.parent().is_some()
                                        } else if let Some(parent) = current_path.parent() {
                                            parent.parent().is_some()
                                        } else {
                                            false
                                        };
                                    let max_index = if has_parent {
                                        self.suggestions.len()
                                    } else {
                                        self.suggestions.len() - 1
                                    };
                                    self.selected_index = (self.selected_index + 1).min(max_index);
                                }
                            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                }
                            } else if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                                // Tab: autocomplete with current selection
                                let current_path = PathBuf::from(&self.input_path);
                                let has_parent = if current_path.exists() && current_path.is_dir() {
                                    current_path.parent().is_some()
                                } else if let Some(parent) = current_path.parent() {
                                    parent.parent().is_some()
                                } else {
                                    false
                                };

                                // Check if "../" is selected
                                if has_parent && self.selected_index == 0 {
                                    let parent_path =
                                        if current_path.exists() && current_path.is_dir() {
                                            current_path.parent().unwrap()
                                        } else {
                                            current_path.parent().unwrap().parent().unwrap()
                                        };
                                    self.input_path = format!("{}/", parent_path.display());
                                    self.search_query = self.input_path.clone();
                                    self.selected_index = 0;
                                    self.update_suggestions();
                                } else if !self.suggestions.is_empty() {
                                    let suggestion_idx = if has_parent {
                                        self.selected_index.saturating_sub(1)
                                    } else {
                                        self.selected_index
                                    };

                                    if suggestion_idx < self.suggestions.len() {
                                        let selected = &self.suggestions[suggestion_idx];
                                        if selected.is_dir {
                                            self.input_path =
                                                format!("{}/", selected.path.display());
                                        } else {
                                            self.input_path = selected.path.display().to_string();
                                        }
                                        self.search_query = self.input_path.clone();
                                        self.selected_index = 0;
                                        self.update_suggestions();
                                    }
                                }
                            }
                        }

                        if ui.button(I18n::browse_button()).clicked() {
                            // Fall back to traditional file dialog
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Log files", &["log", "txt", "json"])
                                .add_filter("All files", &["*"])
                                .pick_file()
                            {
                                action = FilePickerAction::OpenFile(path, self.selected_encoding);
                                self.open = false;
                            }
                        }
                    });

                    // Encoding selection
                    ui.horizontal(|ui| {
                        ui.label(I18n::file_encoding());
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

                        ui.add(
                            egui::Label::new(I18n::file_encoding_hint())
                                .wrap_mode(egui::TextWrapMode::Wrap),
                        );
                    });

                    ui.separator();

                    // Suggestions list
                    let selected_index = self.selected_index;
                    let search_query = self.search_query.clone();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .max_height(300.0)
                        .show(ui, |ui| {
                            if self.suggestions.is_empty() && self.recent_files.is_empty() {
                                ui.label(I18n::start_typing_hint());
                            } else if self.suggestions.is_empty() && !self.recent_files.is_empty() {
                                // Show recent files when no suggestions
                                ui.label(I18n::recent_files_label());
                                ui.separator();
                                for (idx, path) in self.recent_files.iter().enumerate() {
                                    let is_selected =
                                        idx == selected_index && search_query.is_empty();
                                    Self::render_file_item_static(
                                        ui,
                                        path,
                                        false,
                                        is_selected,
                                        &mut action,
                                        &mut self.open,
                                        self.selected_encoding,
                                    );
                                }
                            } else {
                                // Show "../" option to go up one directory if we're in a directory
                                let current_path = PathBuf::from(&self.input_path);
                                let show_parent = if current_path.exists() && current_path.is_dir()
                                {
                                    current_path.parent().is_some()
                                } else if let Some(parent) = current_path.parent() {
                                    parent.parent().is_some()
                                } else {
                                    false
                                };

                                if show_parent {
                                    let is_selected = 0 == selected_index;
                                    let parent_path =
                                        if current_path.exists() && current_path.is_dir() {
                                            current_path.parent().unwrap()
                                        } else {
                                            current_path.parent().unwrap().parent().unwrap()
                                        };

                                    let parent_suggestion = FileSuggestion {
                                        path: parent_path.to_path_buf(),
                                        display_name: "../".to_string(),
                                        is_dir: true,
                                    };

                                    Self::render_suggestion_item_static(
                                        ui,
                                        &parent_suggestion,
                                        is_selected,
                                        &mut action,
                                        &mut self.open,
                                        &mut self.input_path,
                                        &mut self.search_query,
                                        &mut self.selected_index,
                                        &mut needs_update,
                                        self.selected_encoding,
                                    );
                                }

                                // Show suggestions
                                let offset = if show_parent { 1 } else { 0 };
                                for (idx, suggestion) in self.suggestions.iter().enumerate() {
                                    let is_selected = idx + offset == selected_index;
                                    Self::render_suggestion_item_static(
                                        ui,
                                        suggestion,
                                        is_selected,
                                        &mut action,
                                        &mut self.open,
                                        &mut self.input_path,
                                        &mut self.search_query,
                                        &mut self.selected_index,
                                        &mut needs_update,
                                        self.selected_encoding,
                                    );
                                }
                            }
                        });
                });
            });

        // Check if window was closed
        if !window_open && self.open {
            self.open = false;
            if matches!(action, FilePickerAction::None) {
                action = FilePickerAction::Cancel;
            }
        }

        // Update suggestions if directory was navigated
        if needs_update {
            self.update_suggestions();
        }

        action
    }

    /// Render a suggestion item
    fn render_suggestion_item_static(
        ui: &mut egui::Ui,
        suggestion: &FileSuggestion,
        is_selected: bool,
        action: &mut FilePickerAction,
        open: &mut bool,
        input_path: &mut String,
        search_query: &mut String,
        selected_index: &mut usize,
        needs_update: &mut bool,
        selected_encoding: Option<&'static Encoding>,
    ) {
        let icon = if suggestion.is_dir { "📁" } else { "📄" };

        let mut frame = egui::Frame::default().inner_margin(egui::Margin::symmetric(8, 4));

        if is_selected {
            frame = frame.fill(ui.style().visuals.selection.bg_fill);
        }

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon);

                let response = ui.selectable_label(false, &suggestion.display_name);

                if response.clicked() {
                    if suggestion.is_dir {
                        // Navigate into directory
                        *input_path = format!("{}/", suggestion.path.display());
                        *search_query = input_path.clone();
                        *selected_index = 0;
                        *needs_update = true;
                    } else {
                        // Open file
                        *action =
                            FilePickerAction::OpenFile(suggestion.path.clone(), selected_encoding);
                        *open = false;
                    }
                }

                // Show full path on hover
                if response.hovered() {
                    response.on_hover_text(suggestion.path.display().to_string());
                }
            });
        });
    }

    /// Render a file item (for recent files)
    fn render_file_item_static(
        ui: &mut egui::Ui,
        path: &Path,
        is_dir: bool,
        is_selected: bool,
        action: &mut FilePickerAction,
        open: &mut bool,
        selected_encoding: Option<&'static Encoding>,
    ) {
        let icon = if is_dir { "📁" } else { "📄" };
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let mut frame = egui::Frame::default().inner_margin(egui::Margin::symmetric(8, 4));

        if is_selected {
            frame = frame.fill(ui.style().visuals.selection.bg_fill);
        }

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(icon);

                let response = ui.selectable_label(false, &display_name);

                if response.clicked() {
                    *action = FilePickerAction::OpenFile(path.to_path_buf(), selected_encoding);
                    *open = false;
                }

                // Show full path on hover
                if response.hovered() {
                    response.on_hover_text(path.display().to_string());
                }
            });
        });
    }

    /// Update suggestions based on current input
    fn update_suggestions(&mut self) {
        self.suggestions.clear();

        if self.input_path.is_empty() {
            return;
        }

        let path = PathBuf::from(&self.input_path);

        // Determine base directory and search pattern
        let (base_dir, pattern) = if path.exists() && path.is_dir() {
            // If it's a directory, list its contents
            (path.clone(), String::new())
        } else if let Some(parent) = path.parent() {
            // If parent exists, search in parent directory
            if parent.exists() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                (parent.to_path_buf(), file_name)
            } else {
                // Try home directory as fallback
                if let Some(home) = dirs::home_dir() {
                    (home, self.input_path.to_lowercase())
                } else {
                    return;
                }
            }
        } else {
            // Use home directory as fallback
            if let Some(home) = dirs::home_dir() {
                (home, self.input_path.to_lowercase())
            } else {
                return;
            }
        };

        // List directory contents
        if let Ok(entries) = fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let entry_path = entry.path();
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy().to_lowercase();

                    // Filter by pattern
                    if pattern.is_empty() || file_name_str.contains(&pattern) {
                        // Only show log files and directories
                        let is_dir = metadata.is_dir();
                        let is_log_file = !is_dir
                            && (entry_path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|ext| matches!(ext, "log" | "txt" | "json"))
                                .unwrap_or(false));

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

        // Limit number of suggestions
        self.suggestions.truncate(50);
    }
}
