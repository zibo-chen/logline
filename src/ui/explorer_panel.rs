//! Explorer Panel - File and stream browser
//!
//! Displays local files and remote streams in a tree-like structure.

use crate::android_logcat::{AndroidDevice, ConnectionType};
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
    /// Android devices
    pub android_devices: Vec<AndroidDevice>,
    /// TCP connect dialog state
    tcp_connect_address: String,
    /// Show TCP connect dialog
    show_tcp_connect_dialog: bool,
    /// Error message for TCP connection
    tcp_connect_error: Option<String>,
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
            android_devices: Vec::new(),
            tcp_connect_address: String::new(),
            show_tcp_connect_dialog: false,
            tcp_connect_error: None,
        }
    }

    /// Update remote streams
    pub fn update_remote_streams(&mut self, streams: Vec<RemoteStream>) {
        self.remote_streams = streams;
    }

    /// Update Android devices
    pub fn update_android_devices(&mut self, devices: Vec<AndroidDevice>) {
        self.android_devices = devices;
    }

    /// Set TCP connection error message
    pub fn set_tcp_connect_error(&mut self, error: Option<String>) {
        self.tcp_connect_error = error;
    }

    /// Clear TCP connect dialog
    pub fn clear_tcp_connect_dialog(&mut self) {
        self.show_tcp_connect_dialog = false;
        self.tcp_connect_address.clear();
        self.tcp_connect_error = None;
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

                                    let response =
                                        response.on_hover_text(path.display().to_string());

                                    // Context menu for file
                                    response.context_menu(|ui| {
                                        ui.set_min_width(200.0);

                                        if ui.button(t::open_file_context()).clicked() {
                                            action = ExplorerAction::OpenLocalFile(path.clone());
                                            ui.close();
                                        }

                                        if ui.button(t::open_in_split()).clicked() {
                                            action = ExplorerAction::OpenInSplit(path.clone());
                                            ui.close();
                                        }

                                        ui.separator();

                                        if ui.button(t::copy_absolute_path()).clicked() {
                                            action = ExplorerAction::CopyAbsolutePath(path.clone());
                                            ui.close();
                                        }

                                        if ui.button(t::copy_relative_path()).clicked() {
                                            action = ExplorerAction::CopyRelativePath(path.clone());
                                            ui.close();
                                        }

                                        if ui.button(t::copy_filename()).clicked() {
                                            action = ExplorerAction::CopyFilename(path.clone());
                                            ui.close();
                                        }

                                        ui.separator();

                                        #[cfg(target_os = "macos")]
                                        if ui.button(t::reveal_in_finder()).clicked() {
                                            action = ExplorerAction::RevealInFinder(path.clone());
                                            ui.close();
                                        }

                                        #[cfg(target_os = "windows")]
                                        if ui.button("Reveal in Explorer").clicked() {
                                            action = ExplorerAction::RevealInFinder(path.clone());
                                            ui.close();
                                        }

                                        #[cfg(target_os = "linux")]
                                        if ui.button("Reveal in File Manager").clicked() {
                                            action = ExplorerAction::RevealInFinder(path.clone());
                                            ui.close();
                                        }

                                        ui.separator();

                                        if ui.button(t::remove_from_recent()).clicked() {
                                            action = ExplorerAction::RemoveFromRecent(path.clone());
                                            ui.close();
                                        }
                                    });
                                });
                            }

                            // Add clear all recent files option at the bottom of the list
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            if ui.button(t::clear_recent_files()).clicked() {
                                action = ExplorerAction::ClearRecentFiles;
                            }
                        }

                        ui.add_space(8.0);

                        if ui.button(format!("📁 {}", t::open_file())).clicked() {
                            action = ExplorerAction::OpenFileDialog;
                        }
                    });

                ui.add_space(12.0);

                // ANDROID DEVICES section
                CollapsingHeader::new(RichText::new("📱 Android Devices").strong())
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.android_devices.is_empty() {
                            ui.label(RichText::new("No devices connected").weak().italics());
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Connect via USB or WiFi (TCP/IP)")
                                    .weak()
                                    .small(),
                            );
                        } else {
                            for device in &self.android_devices.clone() {
                                let (status_icon, status_color) = if device.is_online {
                                    ("●", Color32::from_rgb(50, 205, 50))
                                } else {
                                    ("○", Color32::GRAY)
                                };

                                // Connection type icon
                                let conn_icon = match device.connection_type {
                                    ConnectionType::Usb => "🔌",
                                    ConnectionType::Tcp => "📶",
                                    ConnectionType::Unknown => "❓",
                                };

                                ui.horizontal(|ui| {
                                    ui.label(conn_icon);
                                    ui.label(RichText::new(status_icon).color(status_color).size(10.0));

                                    let device_label = if device.model != "Unknown" {
                                        device.model.clone()
                                    } else {
                                        device.serial.clone()
                                    };
                                    
                                    let response = ui.selectable_label(
                                        false,
                                        RichText::new(&device_label).size(12.0),
                                    );

                                    if response.clicked() && device.is_online {
                                        action = ExplorerAction::OpenAndroidLogcat(device.clone());
                                    }

                                    // Tooltip with device info
                                    let tooltip_text = format!(
                                        "📱 {}\nSerial: {}\nProduct: {}\nState: {}\nConnection: {}{}",
                                        device.model,
                                        device.serial,
                                        device.product,
                                        device.state,
                                        device.connection_type,
                                        if device.is_online { "\n\nClick to view logcat" } else { "\n\n⚠️ Device offline" }
                                    );

                                    // Context menu for device
                                    response.on_hover_text(&tooltip_text).context_menu(|ui| {
                                        ui.set_min_width(180.0);
                                        
                                        if device.is_online {
                                            if ui.button("📋 View Logcat").clicked() {
                                                action = ExplorerAction::OpenAndroidLogcat(device.clone());
                                                ui.close();
                                            }
                                            ui.separator();
                                        }

                                        if ui.button("📋 Copy Serial").clicked() {
                                            ui.ctx().copy_text(device.serial.clone());
                                            ui.close();
                                        }

                                        // Disconnect option for TCP devices
                                        if device.connection_type == ConnectionType::Tcp {
                                            ui.separator();
                                            if ui.button("🔌 Disconnect").clicked() {
                                                action = ExplorerAction::DisconnectAndroidDevice(device.serial.clone());
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                            }
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Action buttons
                        ui.horizontal(|ui| {
                            if ui.button("🔄 Refresh").on_hover_text("Refresh device list").clicked() {
                                action = ExplorerAction::RefreshAndroidDevices;
                            }
                            
                            if ui.button("📶 Connect TCP").on_hover_text("Connect to device over WiFi").clicked() {
                                self.show_tcp_connect_dialog = true;
                                self.tcp_connect_error = None;
                            }
                        });

                        // TCP Connect Dialog
                        if self.show_tcp_connect_dialog {
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.label(RichText::new("Connect via TCP/IP").strong());
                                ui.add_space(4.0);
                                
                                ui.horizontal(|ui| {
                                    ui.label("Address:");
                                    let response = ui.text_edit_singleline(&mut self.tcp_connect_address);
                                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        if !self.tcp_connect_address.is_empty() {
                                            action = ExplorerAction::ConnectAndroidTcp(self.tcp_connect_address.clone());
                                        }
                                    }
                                });
                                
                                ui.label(RichText::new("e.g. 192.168.1.100 or 192.168.1.100:5555").weak().small());
                                
                                if let Some(ref error) = self.tcp_connect_error {
                                    ui.label(RichText::new(error).color(Color32::RED).small());
                                }
                                
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Connect").clicked() {
                                        if !self.tcp_connect_address.is_empty() {
                                            action = ExplorerAction::ConnectAndroidTcp(self.tcp_connect_address.clone());
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.show_tcp_connect_dialog = false;
                                        self.tcp_connect_address.clear();
                                        self.tcp_connect_error = None;
                                    }
                                });
                            });
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
    OpenInSplit(PathBuf),
    CopyAbsolutePath(PathBuf),
    CopyRelativePath(PathBuf),
    CopyFilename(PathBuf),
    RevealInFinder(PathBuf),
    RemoveFromRecent(PathBuf),
    ClearRecentFiles,
    OpenAndroidLogcat(AndroidDevice),
    RefreshAndroidDevices,
    ConnectAndroidTcp(String),
    DisconnectAndroidDevice(String),
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
