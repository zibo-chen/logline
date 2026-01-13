#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Logline - High-performance real-time log viewer
//!
//! A cross-platform log viewer application built with Rust and egui,
//! designed for efficient real-time log monitoring and analysis.

mod app;
mod config;
mod file_watcher;
mod highlighter;
mod i18n;
mod log_buffer;
mod log_entry;
mod log_reader;
mod protocol;
mod remote_server;
mod search;
mod tray;
mod ui;
mod virtual_scroll;

mod mcp;

use app::LoglineApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Logline");

    // Load icon for window
    let icon_bytes = include_bytes!("../res/icon.png");
    let icon = if let Ok(icon_image) = image::load_from_memory(icon_bytes) {
        let icon_rgba = icon_image.to_rgba8();
        let (width, height) = icon_rgba.dimensions();
        egui::IconData {
            rgba: icon_rgba.into_raw(),
            width,
            height,
        }
    } else {
        egui::IconData::default()
    };

    // Configure native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Logline - Log Viewer")
            .with_taskbar(false) // Don't show in Dock/taskbar when window is hidden
            .with_icon(icon),
        // Disable eframe's built-in persistence, we use our own config file
        persistence_path: None,
        ..Default::default()
    };

    // Run the application
    // Note: TrayManager will be created inside the app after the event loop starts
    eframe::run_native(
        "Logline",
        native_options,
        Box::new(move |cc| Ok(Box::new(LoglineApp::new(cc)))),
    )
}
