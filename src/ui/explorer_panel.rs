//! Explorer Panel - Clean file and stream browser
//!
//! Displays local files and remote streams in a clean, simplified tree structure.
//! Actions like opening files and connecting devices are handled by the Source Picker Dialog.

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

    /// Render the explorer panel
    pub fn show(&mut self, ui: &mut Ui) -> ExplorerAction {
        let mut action = ExplorerAction::None;

        // Set minimum width to prevent panel from shrinking
        ui.set_min_width(200.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // Header with open button
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t::explorer_header()).strong().size(13.0));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Plus button to open source picker
                        let btn = ui.add(
                            egui::Button::new(RichText::new("➕").size(14.0))
                                .frame(false)
                        );
                        if btn.clicked() {
                            action = ExplorerAction::OpenSourcePicker;
                        }
                        btn.on_hover_text(t::open_source());
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // LOCAL FILES section
                CollapsingHeader::new(RichText::new(format!("📂 {}", t::local_files())).size(12.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.local_files.is_empty() {
                            ui.label(RichText::new(t::no_recent_files()).weak().italics().small());
                        } else {
                            for path in &self.local_files.clone() {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path.display().to_string());

                                let response = ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new("📄").size(11.0));
                                    ui.selectable_label(false, RichText::new(&name).size(11.0))
                                }).inner;

                                if response.clicked() {
                                    action = ExplorerAction::OpenLocalFile(path.clone());
                                }

                                let response = response.on_hover_text(path.display().to_string());

                                // Context menu for file
                                response.context_menu(|ui| {
                                    ui.set_min_width(180.0);

                                    if ui.button(format!("📂  {}", t::open_file_context())).clicked() {
                                        action = ExplorerAction::OpenLocalFile(path.clone());
                                        ui.close();
                                    }

                                    if ui.button(format!("⊞  {}", t::open_in_split())).clicked() {
                                        action = ExplorerAction::OpenInSplit(path.clone());
                                        ui.close();
                                    }

                                    ui.separator();

                                    if ui.button(format!("📋  {}", t::copy_absolute_path())).clicked() {
                                        action = ExplorerAction::CopyAbsolutePath(path.clone());
                                        ui.close();
                                    }

                                    if ui.button(format!("📋  {}", t::copy_filename())).clicked() {
                                        action = ExplorerAction::CopyFilename(path.clone());
                                        ui.close();
                                    }

                                    ui.separator();

                                    #[cfg(target_os = "macos")]
                                    if ui.button(format!("🔍  {}", t::reveal_in_finder())).clicked() {
                                        action = ExplorerAction::RevealInFinder(path.clone());
                                        ui.close();
                                    }

                                    #[cfg(target_os = "windows")]
                                    if ui.button("🔍  Reveal in Explorer").clicked() {
                                        action = ExplorerAction::RevealInFinder(path.clone());
                                        ui.close();
                                    }

                                    #[cfg(target_os = "linux")]
                                    if ui.button("🔍  Reveal in File Manager").clicked() {
                                        action = ExplorerAction::RevealInFinder(path.clone());
                                        ui.close();
                                    }

                                    ui.separator();

                                    if ui.button(format!("✕  {}", t::remove_from_recent())).clicked() {
                                        action = ExplorerAction::RemoveFromRecent(path.clone());
                                        ui.close();
                                    }
                                });
                            }

                            // Clear recent files button
                            ui.add_space(4.0);
                            if ui.small_button(t::clear_recent_files()).clicked() {
                                action = ExplorerAction::ClearRecentFiles;
                            }
                        }
                    });

                ui.add_space(8.0);

                // ANDROID DEVICES section
                let online_devices = self.android_devices.iter().filter(|d| d.is_online).count();
                let devices_header = if online_devices > 0 {
                    format!("📱 {} ({})", t::android_devices(), online_devices)
                } else {
                    format!("📱 {}", t::android_devices())
                };
                
                CollapsingHeader::new(RichText::new(&devices_header).size(12.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.android_devices.is_empty() {
                            ui.label(RichText::new(t::no_devices_connected()).weak().italics().small());
                        } else {
                            for device in &self.android_devices.clone() {
                                let (status_icon, status_color) = if device.is_online {
                                    ("●", Color32::from_rgb(50, 205, 50))
                                } else {
                                    ("○", Color32::GRAY)
                                };

                                let conn_icon = match device.connection_type {
                                    ConnectionType::Usb => "🔌",
                                    ConnectionType::Tcp => "📶",
                                    ConnectionType::Unknown => "❓",
                                };

                                let device_label = if device.model != "Unknown" {
                                    device.model.clone()
                                } else {
                                    device.serial.clone()
                                };

                                let response = ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(conn_icon).size(10.0));
                                    ui.label(RichText::new(status_icon).color(status_color).size(8.0));
                                    ui.selectable_label(false, RichText::new(&device_label).size(11.0))
                                }).inner;

                                if response.clicked() && device.is_online {
                                    action = ExplorerAction::OpenAndroidLogcat(device.clone());
                                }

                                let tooltip_text = format!(
                                    "📱 {}\n{}: {}\n{}: {}\n{}{}",
                                    device.model,
                                    t::serial(),
                                    device.serial,
                                    t::connection(),
                                    device.connection_type,
                                    t::state_label(),
                                    if device.is_online { 
                                        format!("\n\n{}", t::click_to_view_logcat()) 
                                    } else { 
                                        format!("\n\n⚠️ {}", t::device_offline()) 
                                    }
                                );

                                response.on_hover_text(&tooltip_text).context_menu(|ui| {
                                    ui.set_min_width(160.0);
                                    
                                    if device.is_online {
                                        if ui.button(format!("📋  {}", t::view_logcat())).clicked() {
                                            action = ExplorerAction::OpenAndroidLogcat(device.clone());
                                            ui.close();
                                        }
                                        ui.separator();
                                    }

                                    if ui.button(format!("📋  {}", t::copy_serial())).clicked() {
                                        ui.ctx().copy_text(device.serial.clone());
                                        ui.close();
                                    }

                                    if device.connection_type == ConnectionType::Tcp {
                                        ui.separator();
                                        if ui.button(format!("🔌  {}", t::disconnect())).clicked() {
                                            action = ExplorerAction::DisconnectAndroidDevice(device.serial.clone());
                                            ui.close();
                                        }
                                    }
                                });
                            }
                        }

                        // Manage devices button
                        ui.add_space(4.0);
                        if ui.small_button(t::manage_devices()).clicked() {
                            action = ExplorerAction::OpenSourcePickerAndroid;
                        }
                    });

                ui.add_space(8.0);

                // REMOTE STREAMS section
                let online_streams = self.remote_streams.iter()
                    .filter(|s| s.status == ConnectionStatus::Online)
                    .count();
                let streams_header = if online_streams > 0 {
                    format!("🌐 {} ({})", t::remote_streams(), online_streams)
                } else {
                    format!("🌐 {}", t::remote_streams())
                };
                
                CollapsingHeader::new(RichText::new(&streams_header).size(12.0))
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.remote_streams.is_empty() {
                            ui.label(RichText::new(t::waiting_for_connections()).weak().italics().small());
                        } else {
                            // Group streams by IP address
                            let mut streams_by_ip: HashMap<String, Vec<&RemoteStream>> = HashMap::new();
                            for stream in &self.remote_streams {
                                let ip = stream.ip_address();
                                streams_by_ip.entry(ip).or_default().push(stream);
                            }

                            let mut ips: Vec<_> = streams_by_ip.keys().collect();
                            ips.sort();

                            for ip in ips {
                                let streams = &streams_by_ip[ip];
                                let ip_online_count = streams
                                    .iter()
                                    .filter(|s| s.status == ConnectionStatus::Online)
                                    .count();

                                let ip_header = format!("🖥 {} ({})", ip, ip_online_count);
                                let header_color = if ip_online_count > 0 {
                                    Color32::from_rgb(50, 205, 50)
                                } else {
                                    Color32::GRAY
                                };

                                CollapsingHeader::new(
                                    RichText::new(&ip_header).color(header_color).size(11.0),
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

                                        let response = ui.horizontal(|ui| {
                                            ui.add_space(8.0);
                                            ui.label(RichText::new(status_icon).color(status_color).size(8.0));
                                            ui.selectable_label(
                                                false,
                                                RichText::new(&stream.project_name).size(11.0),
                                            )
                                        }).inner;

                                        if response.clicked() {
                                            action = ExplorerAction::OpenRemoteStream((*stream).clone());
                                        }

                                        response.on_hover_ui(|ui| {
                                            ui.label(format!("{}: {}", t::project(), stream.project_name));
                                            ui.label(format!("{}: {}", t::address(), stream.remote_addr));
                                            ui.label(format!("{}: {:?}", t::status(), stream.status));
                                            ui.label(format!("{}: {}", t::received(), format_bytes(stream.bytes_received)));
                                        });

                                        // Show bytes received
                                        ui.horizontal(|ui| {
                                            ui.add_space(20.0);
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
    OpenSourcePicker,
    OpenSourcePickerAndroid,
    OpenInSplit(PathBuf),
    CopyAbsolutePath(PathBuf),
    CopyFilename(PathBuf),
    RevealInFinder(PathBuf),
    RemoveFromRecent(PathBuf),
    ClearRecentFiles,
    OpenAndroidLogcat(AndroidDevice),
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
