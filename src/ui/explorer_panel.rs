//! Explorer Panel - File and stream browser
//!
//! Displays local files and remote streams in a tree-like structure.

use crate::i18n::Translations as t;
use crate::remote_server::{ConnectionStatus, RemoteStream};
use egui::{CollapsingHeader, Color32, RichText, Ui};
use std::collections::HashMap;
use std::path::PathBuf;

/// Open editor/tab entry
#[derive(Debug, Clone)]
pub struct OpenEditor {
    /// Display name
    pub name: String,
    /// File path (local or cache)
    pub path: PathBuf,
    /// Whether this is a remote stream
    pub is_remote: bool,
    /// Whether it has unsaved changes
    pub is_dirty: bool,
}

/// Explorer panel state
pub struct ExplorerPanel {
    /// Currently open editors/tabs
    pub open_editors: Vec<OpenEditor>,
    /// Selected editor index
    pub selected_editor: Option<usize>,
    /// Recent local files
    pub local_files: Vec<PathBuf>,
    /// Remote streams
    pub remote_streams: Vec<RemoteStream>,
    /// Whether panel is visible
    #[allow(dead_code)]
    pub visible: bool,
}

impl Default for ExplorerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerPanel {
    pub fn new() -> Self {
        Self {
            open_editors: Vec::new(),
            selected_editor: None,
            local_files: Vec::new(),
            remote_streams: Vec::new(),
            visible: true,
        }
    }

    /// Add an open editor
    pub fn add_editor(&mut self, editor: OpenEditor) {
        // Check if already open
        if let Some(idx) = self.open_editors.iter().position(|e| e.path == editor.path) {
            self.selected_editor = Some(idx);
            return;
        }

        self.open_editors.push(editor);
        self.selected_editor = Some(self.open_editors.len() - 1);
    }

    /// Close an editor and return the closed editor
    pub fn close_editor(&mut self, index: usize) -> Option<OpenEditor> {
        if index < self.open_editors.len() {
            let editor = self.open_editors.remove(index);

            // Update selection
            if let Some(selected) = self.selected_editor {
                if selected >= self.open_editors.len() {
                    self.selected_editor = self.open_editors.len().checked_sub(1);
                } else if selected > index {
                    self.selected_editor = Some(selected - 1);
                }
            }

            Some(editor)
        } else {
            None
        }
    }

    /// Update remote streams
    pub fn update_remote_streams(&mut self, streams: Vec<RemoteStream>) {
        self.remote_streams = streams;
    }

    /// Add to recent files
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if exists
        self.local_files.retain(|p| p != &path);

        // Add to front
        self.local_files.insert(0, path);

        // Keep only last 10
        self.local_files.truncate(10);
    }

    /// Render the explorer panel
    pub fn show(&mut self, ui: &mut Ui) -> ExplorerAction {
        let mut action = ExplorerAction::None;

        ui.vertical(|ui| {
            ui.add_space(4.0);

            // OPEN EDITORS section
            CollapsingHeader::new(RichText::new(format!("📑 {}", t::open_editors())).strong())
                .default_open(true)
                .show(ui, |ui| {
                    if self.open_editors.is_empty() {
                        ui.label(RichText::new(t::no_open_files()).weak().italics());
                    } else {
                        let mut close_idx = None;
                        let mut select_idx = None;

                        for (idx, editor) in self.open_editors.iter().enumerate() {
                            let is_selected = self.selected_editor == Some(idx);

                            ui.horizontal(|ui| {
                                // Icon
                                let icon = if editor.is_remote { "📡" } else { "📄" };
                                ui.label(icon);

                                // Name with selection highlight
                                let name_text = if editor.is_dirty {
                                    format!("● {}", editor.name)
                                } else {
                                    editor.name.clone()
                                };

                                let response = ui.selectable_label(
                                    is_selected,
                                    RichText::new(&name_text).size(12.0),
                                );

                                if response.clicked() {
                                    select_idx = Some(idx);
                                }

                                // Add spacing to push button to the right
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Close button - always render, will be visible when row is hovered
                                        if ui.small_button("✕").clicked() {
                                            close_idx = Some(idx);
                                        }
                                    },
                                );
                            });
                        }

                        if let Some(idx) = select_idx {
                            self.selected_editor = Some(idx);
                            action = ExplorerAction::SelectEditor(idx);
                        }

                        if let Some(idx) = close_idx {
                            // Get editor info before closing
                            if let Some(editor) = self.open_editors.get(idx).cloned() {
                                action = ExplorerAction::CloseEditor(idx, editor);
                            }
                        }
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // REMOTE STREAMS section (grouped by IP)
            CollapsingHeader::new(RichText::new(format!("🌐 {}", t::remote_streams())).strong())
                .default_open(true)
                .show(ui, |ui| {
                    if self.remote_streams.is_empty() {
                        ui.label(RichText::new(t::waiting_for_connections()).weak().italics());
                        ui.add_space(4.0);
                        ui.label(RichText::new(t::agents_will_appear()).weak().small());
                    } else {
                        // Group streams by IP address
                        let mut streams_by_ip: HashMap<String, Vec<&RemoteStream>> = HashMap::new();
                        for stream in &self.remote_streams {
                            let ip = stream.ip_address();
                            streams_by_ip.entry(ip).or_default().push(stream);
                        }

                        // Sort IPs for consistent display
                        let mut ips: Vec<_> = streams_by_ip.keys().collect();
                        ips.sort();

                        for ip in ips {
                            let streams = &streams_by_ip[ip];

                            // Count online streams for this IP
                            let online_count = streams
                                .iter()
                                .filter(|s| s.status == ConnectionStatus::Online)
                                .count();
                            let total_count = streams.len();

                            // IP group header with status indicator
                            let ip_header = if online_count > 0 {
                                format!("🖥 {} ({}/{})", ip, online_count, total_count)
                            } else {
                                format!("🖥 {} ({})", ip, t::offline())
                            };

                            let header_color = if online_count > 0 {
                                Color32::from_rgb(50, 205, 50)
                            } else {
                                Color32::GRAY
                            };

                            CollapsingHeader::new(
                                RichText::new(&ip_header).color(header_color).size(12.0),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                for stream in streams {
                                    let (status_icon, status_color) = match stream.status {
                                        ConnectionStatus::Online => {
                                            ("●", Color32::from_rgb(50, 205, 50))
                                        }
                                        ConnectionStatus::Offline => ("○", Color32::GRAY),
                                    };

                                    ui.horizontal(|ui| {
                                        ui.add_space(8.0);
                                        ui.label(RichText::new(status_icon).color(status_color));

                                        let response = ui.selectable_label(
                                            false,
                                            RichText::new(&stream.project_name).size(12.0),
                                        );

                                        if response.clicked() {
                                            action =
                                                ExplorerAction::OpenRemoteStream((*stream).clone());
                                        }

                                        // Show tooltip with full details
                                        response.on_hover_ui(|ui| {
                                            ui.label(format!(
                                                "{}: {}",
                                                t::project(),
                                                stream.project_name
                                            ));
                                            ui.label(format!(
                                                "{}: {}",
                                                t::address(),
                                                stream.remote_addr
                                            ));
                                            ui.label(format!(
                                                "{}: {:?}",
                                                t::status(),
                                                stream.status
                                            ));
                                            ui.label(format!(
                                                "{}: {}",
                                                t::received(),
                                                format_bytes(stream.bytes_received)
                                            ));
                                        });
                                    });

                                    // Show bytes received
                                    ui.horizontal(|ui| {
                                        ui.add_space(24.0);
                                        let bytes = format_bytes(stream.bytes_received);
                                        ui.label(RichText::new(bytes).weak().small());
                                    });
                                }
                            });
                        }
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // LOCAL FILES section
            CollapsingHeader::new(RichText::new(format!("📂 {}", t::local_files())).strong())
                .default_open(true)
                .show(ui, |ui| {
                    if self.local_files.is_empty() {
                        ui.label(RichText::new(t::no_recent_files()).weak().italics());
                    } else {
                        for path in &self.local_files.clone() {
                            let name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string());

                            ui.horizontal(|ui| {
                                ui.label("📄");
                                let response =
                                    ui.selectable_label(false, RichText::new(&name).size(12.0));

                                if response.clicked() {
                                    action = ExplorerAction::OpenLocalFile(path.clone());
                                }

                                response.on_hover_text(path.display().to_string());
                            });
                        }
                    }

                    ui.add_space(8.0);

                    if ui.button(format!("📁 {}", t::open_file())).clicked() {
                        action = ExplorerAction::OpenFileDialog;
                    }
                });
        });

        action
    }
}

/// Actions from the explorer panel
#[derive(Debug, Clone)]
pub enum ExplorerAction {
    None,
    SelectEditor(usize),
    /// Close editor with (index, editor info)
    CloseEditor(usize, OpenEditor),
    OpenLocalFile(PathBuf),
    OpenRemoteStream(RemoteStream),
    OpenFileDialog,
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
