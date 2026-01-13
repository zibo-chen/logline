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
mod ui;
mod virtual_scroll;

mod mcp;

use app::LoglineApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Logline");

    // Configure native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Logline - Log Viewer"),
        // Disable eframe's built-in persistence, we use our own config file
        persistence_path: None,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "Logline",
        native_options,
        Box::new(|cc| Ok(Box::new(LoglineApp::new(cc)))),
    )
}
