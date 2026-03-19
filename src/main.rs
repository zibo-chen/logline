#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Logline - High-performance real-time log viewer
//!
//! A cross-platform log viewer application built with Rust and egui,
//! designed for efficient real-time log monitoring and analysis.

mod android_logcat;
mod app;
mod bookmarks;
mod config;
mod file_watcher;
mod grok_parser;
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
mod open_file_handler;

use app::LoglineApp;
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Logline");

    // Collect files passed as command-line arguments (e.g. `logline file.log`)
    let initial_files: Vec<PathBuf> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    if !initial_files.is_empty() {
        tracing::info!("Opening files from command line: {:?}", initial_files);
    }

    // Prepare the channel for file-open events (phase 1 – before eframe starts).
    // The NSAppleEventManager registration (phase 2) happens inside LoglineApp::new()
    // after NSApplication has been created by winit.
    let file_receiver = open_file_handler::create_file_receiver();

    // Load icon for window
    let icon_bytes = include_bytes!("../res/icon.png");
    let icon = match image::load_from_memory(icon_bytes) {
        Ok(icon_image) => {
            let icon_rgba = icon_image.to_rgba8();
            let (width, height) = icon_rgba.dimensions();
            tracing::info!("Successfully loaded window icon: {}x{}", width, height);
            egui::IconData {
                rgba: icon_rgba.into_raw(),
                width,
                height,
            }
        }
        Err(e) => {
            tracing::error!("Failed to load window icon: {}", e);
            egui::IconData::default()
        }
    };

    // Configure native options with platform-specific titlebar settings
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Logline - Log Viewer")
        .with_icon(icon.clone());

    // Platform-specific titlebar configuration
    #[cfg(target_os = "macos")]
    {
        // macOS: Use native titlebar with overlay style
        // Content extends into titlebar area but native buttons remain visible
        viewport_builder = viewport_builder
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_taskbar(false); // Don't show in Dock when window is hidden
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // Windows/Linux: Disable decorations to use custom titlebar
        // Keep taskbar icon visible on Windows
        viewport_builder = viewport_builder.with_decorations(false).with_taskbar(true);
        // Ensure taskbar icon is shown on Windows
    }

    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        // Disable eframe's built-in persistence, we use our own config file
        persistence_path: None,
        ..Default::default()
    };

    // Run the application
    // Note: TrayManager will be created inside the app after the event loop starts
    eframe::run_native(
        "Logline",
        native_options,
        Box::new(move |cc| {
            // Install image loaders for egui (required for egui-desktop SVG assets)
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(LoglineApp::new(cc, initial_files, file_receiver)))
        }),
    )
}
