//! System tray support for Logline
//!
//! This module provides system tray functionality, allowing the application
//! to minimize to the system tray and run in the background.

use crate::i18n::Translations;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

/// Events from the system tray
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// User clicked "Show Window"
    ShowWindow,
    /// User clicked "Hide Window"
    HideWindow,
    /// User clicked "Open File"
    OpenFile,
    /// User clicked "Settings"
    Settings,
    /// User clicked "About"
    About,
    /// User clicked "Quit"
    Quit,
}

/// System tray manager
pub struct TrayManager {
    /// The tray icon handle
    _tray_icon: TrayIcon,
    /// Event receiver (receives events from the background thread)
    event_rx: Receiver<TrayEvent>,
    /// Stop signal sender
    _stop_tx: Sender<()>,
    /// Quit requested flag (shared with tray thread)
    quit_requested: Arc<AtomicBool>,
}

impl TrayManager {
    /// Create a new tray manager with the specified icon
    pub fn new(ctx: egui::Context) -> Result<Self, Box<dyn std::error::Error>> {
        // Load icon from embedded bytes
        let icon_bytes = include_bytes!("../res/tray.png");
        let icon_image = image::load_from_memory(icon_bytes)?;
        let icon_rgba = icon_image.to_rgba8();
        let (width, height) = icon_rgba.dimensions();
        let icon = Icon::from_rgba(icon_rgba.into_raw(), width, height)?;

        // Create menu with i18n support
        let menu = Menu::new();

        // Window control items
        let show_item = MenuItem::new(Translations::tray_show_window(), true, None);
        let hide_item = MenuItem::new(Translations::tray_hide_window(), true, None);

        // Separator
        let separator1 = PredefinedMenuItem::separator();

        // File operations
        let open_file_item = MenuItem::new(Translations::tray_open_file(), true, None);

        // Separator
        let separator2 = PredefinedMenuItem::separator();

        // Settings and About
        let settings_item = MenuItem::new(Translations::tray_settings(), true, None);
        let about_item = MenuItem::new(Translations::tray_about(), true, None);

        // Separator
        let separator3 = PredefinedMenuItem::separator();

        // Quit
        let quit_item = MenuItem::new(Translations::tray_quit(), true, None);

        // Store IDs for event handling
        let show_item_id = show_item.id().clone();
        let hide_item_id = hide_item.id().clone();
        let open_file_item_id = open_file_item.id().clone();
        let settings_item_id = settings_item.id().clone();
        let about_item_id = about_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        // Build menu
        menu.append(&show_item)?;
        menu.append(&hide_item)?;
        menu.append(&separator1)?;
        menu.append(&open_file_item)?;
        menu.append(&separator2)?;
        menu.append(&settings_item)?;
        menu.append(&about_item)?;
        menu.append(&separator3)?;
        menu.append(&quit_item)?;

        // Create tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false) // Left click will not show menu
            .with_tooltip(Translations::tray_tooltip())
            .with_icon(icon)
            .build()?;

        // Create channels for communication
        let (event_tx, event_rx) = channel();
        let (stop_tx, stop_rx) = channel();

        let quit_requested = Arc::new(AtomicBool::new(false));
        let quit_requested_thread = Arc::clone(&quit_requested);

        // Clone IDs for the background thread
        let show_id = show_item_id.clone();
        let hide_id = hide_item_id.clone();
        let open_file_id = open_file_item_id.clone();
        let settings_id = settings_item_id.clone();
        let about_id = about_item_id.clone();
        let quit_id = quit_item_id.clone();

        // Spawn a dedicated thread to monitor tray events
        thread::spawn(move || {
            tracing::info!("Tray event monitoring thread started");
            loop {
                // Check if we should stop
                if stop_rx.try_recv().is_ok() {
                    tracing::info!("Tray event monitoring thread stopping");
                    break;
                }

                // Process tray icon events (clicks)
                let tray_receiver = TrayIconEvent::receiver();
                while let Ok(event) = tray_receiver.try_recv() {
                    tracing::debug!("Tray icon event received: {:?}", event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        tracing::info!("Left click detected on tray icon");
                        let _ = event_tx.send(TrayEvent::ShowWindow);
                        // Request repaint so the UI loop processes the event promptly
                        ctx.request_repaint();
                    }
                }

                // Process menu events (right-click menu)
                let menu_receiver = MenuEvent::receiver();
                if let Ok(event) = menu_receiver.try_recv() {
                    tracing::debug!("Menu event received: {:?}", event);
                    if event.id == show_id {
                        tracing::info!("Show window menu item clicked");
                        let _ = event_tx.send(TrayEvent::ShowWindow);
                        ctx.request_repaint();
                    } else if event.id == hide_id {
                        tracing::info!("Hide window menu item clicked");
                        let _ = event_tx.send(TrayEvent::HideWindow);
                        ctx.request_repaint();
                    } else if event.id == open_file_id {
                        tracing::info!("Open file menu item clicked");
                        let _ = event_tx.send(TrayEvent::OpenFile);
                        ctx.request_repaint();
                    } else if event.id == settings_id {
                        tracing::info!("Settings menu item clicked");
                        let _ = event_tx.send(TrayEvent::Settings);
                        ctx.request_repaint();
                    } else if event.id == about_id {
                        tracing::info!("About menu item clicked");
                        let _ = event_tx.send(TrayEvent::About);
                        ctx.request_repaint();
                    } else if event.id == quit_id {
                        tracing::info!("Quit menu item clicked");
                        quit_requested_thread.store(true, Ordering::SeqCst);
                        let _ = event_tx.send(TrayEvent::Quit);
                        ctx.request_repaint();
                    }
                }

                // Small sleep to avoid busy-waiting
                thread::sleep(Duration::from_millis(50));
            }
            tracing::info!("Tray event monitoring thread stopped");
        });

        Ok(Self {
            _tray_icon: tray_icon,
            event_rx,
            _stop_tx: stop_tx,
            quit_requested,
        })
    }

    /// Process pending menu events and return any tray events
    /// This is now a simple non-blocking check of the channel
    pub fn poll_events(&self) -> Option<TrayEvent> {
        // Simply try to receive from the channel
        // The background thread is doing all the actual event monitoring
        self.event_rx.try_recv().ok()
    }

    /// Whether quit has been requested from the tray
    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested.load(Ordering::SeqCst)
    }
}
