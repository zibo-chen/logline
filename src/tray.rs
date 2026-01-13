//! System tray support for Logline
//!
//! This module provides system tray functionality, allowing the application
//! to minimize to the system tray and run in the background.

use std::sync::mpsc::{channel, Receiver};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

/// Events from the system tray
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// User clicked "Show Window"
    ShowWindow,
    /// User clicked "Quit"
    Quit,
}

/// System tray manager
pub struct TrayManager {
    /// The tray icon handle
    _tray_icon: TrayIcon,
    /// Menu item IDs
    show_item_id: tray_icon::menu::MenuId,
    quit_item_id: tray_icon::menu::MenuId,

    /// Event receiver
    event_rx: Receiver<TrayEvent>,
}

impl TrayManager {
    /// Create a new tray manager with the specified icon
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load icon from embedded bytes
        let icon_bytes = include_bytes!("../res/tray.png");
        let icon_image = image::load_from_memory(icon_bytes)?;
        let icon_rgba = icon_image.to_rgba8();
        let (width, height) = icon_rgba.dimensions();
        let icon = Icon::from_rgba(icon_rgba.into_raw(), width, height)?;

        // Create menu
        let menu = Menu::new();
        let show_item = MenuItem::new("Show", true, None);
        let quit_item = MenuItem::new("Exit", true, None);

        let show_item_id = show_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        menu.append(&show_item)?;
        menu.append(&quit_item)?;

        // Create tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false) // Left click will not show menu
            .with_tooltip("Logline - 日志查看器")
            .with_icon(icon)
            .build()?;

        // Create event channel
        let (_event_tx, event_rx) = channel();

        Ok(Self {
            _tray_icon: tray_icon,
            show_item_id,
            quit_item_id,
            event_rx,
        })
    }

    /// Process pending menu events and return any tray events
    pub fn poll_events(&self) -> Option<TrayEvent> {
        // Process all tray icon events in the queue to avoid being blocked by Move events
        let receiver = TrayIconEvent::receiver();
        while let Ok(event) = receiver.try_recv() {
            tracing::debug!("Received tray icon event: {:?}", event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                tracing::info!("Left click detected, showing window");
                return Some(TrayEvent::ShowWindow);
            }
        }

        // Check for menu events (right click menu)
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            tracing::debug!("Received menu event: {:?}", event);
            if event.id == self.show_item_id {
                return Some(TrayEvent::ShowWindow);
            } else if event.id == self.quit_item_id {
                return Some(TrayEvent::Quit);
            }
        }

        // Check for events from the channel
        if let Ok(event) = self.event_rx.try_recv() {
            return Some(event);
        }

        None
    }
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new().expect("Failed to create tray manager")
    }
}
