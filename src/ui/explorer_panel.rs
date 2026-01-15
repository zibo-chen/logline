//! Explorer Panel - File and stream browser
//!
//! Displays local files and remote streams in a tree-like structure.

use crate::i18n::Translations as t;
use crate::remote_server::{ConnectionStatus, RemoteStream};
use egui::{CollapsingHeader, Color32, RichText, Ui};
use std::collections::HashMap;
use std::path::PathBuf;

/// Explorer panel state
pub struct ExplorerPanel {
    /// Recent local files
    pub local_files: Vec<PathBuf>,
    /// Remote streams
    pub remote_streams: Vec<RemoteStream>,
}

impl Default for ExplorerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerPanel {
    pub fn new() -> Self {
        Self {
            local_files: Vec::new(),
            remote_streams: Vec::new(),
        }
    }

    /// Update remote streams
    pub fn update_remote_streams(&mut self, streams: Vec<RemoteStream>) {
        self.remote_streams = streams;
    }

    /// Render the explorer panel
    pub fn show(&mut self, ui: &mut Ui) -> ExplorerAction {
        let mut action = ExplorerAction::None;

        // Set minimum width to prevent panel from shrinking
        ui.set_min_width(200.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // LOCAL FILES section (moved to top)
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

                ui.add_space(12.0);

                // REMOTE STREAMS section (grouped by IP)
                CollapsingHeader::new(
                    RichText::new(format!("🌐 {}", t::remote_streams())).strong(),
                )
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
            });

        action
    }
}

/// Actions from the explorer panel
#[derive(Debug, Clone)]
pub enum ExplorerAction {
    None,
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
