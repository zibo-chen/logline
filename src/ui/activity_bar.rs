//! Activity Bar - Left-side icon navigation
//!
//! A narrow vertical bar with icon buttons for switching views.

use crate::i18n::Translations as t;
use egui::{Color32, RichText, Ui, Vec2};

/// Activity bar view types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityView {
    Explorer,
    Search,
    Filters,
    Grok,
    Bookmarks,
    Settings,
}

/// Activity bar state
pub struct ActivityBar {
    /// Current active view
    pub active_view: ActivityView,
    /// Whether sidebar is visible
    pub sidebar_visible: bool,
    /// Server listening status
    pub server_running: bool,
    /// Server port
    pub server_port: u16,
    /// Number of connected agents
    pub connected_agents: usize,
    /// MCP server running status
    pub mcp_running: bool,
    /// MCP server port
    pub mcp_port: u16,
}

impl Default for ActivityBar {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityBar {
    pub fn new() -> Self {
        Self {
            active_view: ActivityView::Explorer,
            sidebar_visible: true,
            server_running: false,
            server_port: 12500,
            connected_agents: 0,
            mcp_running: false,
            mcp_port: 12600,
        }
    }

    /// Render the activity bar
    pub fn show(&mut self, ui: &mut Ui) -> ActivityBarAction {
        let mut action = ActivityBarAction::None;

        ui.vertical_centered(|ui| {
            ui.add_space(8.0);

            // Explorer button - only show as active when sidebar is visible and this view is selected
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Explorer;
            if self.icon_button(ui, "📁", t::explorer(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Explorer {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Explorer;
                    action = ActivityBarAction::SwitchView(ActivityView::Explorer);
                }
            }

            ui.add_space(4.0);

            // Search button
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Search;
            if self.icon_button(ui, "🔍", t::search(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Search {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Search;
                    action = ActivityBarAction::SwitchView(ActivityView::Search);
                }
            }

            ui.add_space(4.0);

            // Filters button
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Filters;
            if self.icon_button(ui, "⚡", t::filters(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Filters {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Filters;
                    action = ActivityBarAction::SwitchView(ActivityView::Filters);
                }
            }

            ui.add_space(4.0);

            // Grok parser button
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Grok;
            if self.icon_button(ui, "🔧", t::grok_parser(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Grok {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Grok;
                    action = ActivityBarAction::SwitchView(ActivityView::Grok);
                }
            }

            ui.add_space(4.0);

            // Bookmarks button
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Bookmarks;
            if self.icon_button(ui, "★", t::bookmarks(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Bookmarks {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Bookmarks;
                    action = ActivityBarAction::SwitchView(ActivityView::Bookmarks);
                }
            }

            ui.add_space(4.0);

            // Settings button
            let is_active = self.sidebar_visible && self.active_view == ActivityView::Settings;
            if self.icon_button(ui, "⚙", t::settings(), is_active) {
                if self.sidebar_visible && self.active_view == ActivityView::Settings {
                    action = ActivityBarAction::TogglePanel;
                } else {
                    self.active_view = ActivityView::Settings;
                    action = ActivityBarAction::SwitchView(ActivityView::Settings);
                }
            }

            // Spacer to push status indicators to bottom
            ui.add_space(ui.available_height() - 120.0);

            // Server status indicator
            let (status_icon, status_color, tooltip) = if self.server_running {
                if self.connected_agents > 0 {
                    (
                        "📡",
                        Color32::from_rgb(50, 205, 50),
                        t::server_running()
                            .replace("{}", &self.server_port.to_string())
                            .replacen("{}", &self.connected_agents.to_string(), 1),
                    )
                } else {
                    (
                        "📡",
                        Color32::from_rgb(255, 193, 7),
                        t::server_waiting().replace("{}", &self.server_port.to_string()),
                    )
                }
            } else {
                ("📡", Color32::GRAY, t::server_stopped().to_string())
            };

            let response = ui.add(
                egui::Button::new(RichText::new(status_icon).size(20.0).color(status_color))
                    .frame(false)
                    .min_size(Vec2::new(32.0, 32.0)),
            );

            if response.clicked() {
                action = ActivityBarAction::ToggleServer;
            }

            response.on_hover_text(tooltip);

            ui.add_space(4.0);

            // MCP Server status indicator
            let (mcp_icon, mcp_color, mcp_tooltip) = if self.mcp_running {
                (
                    "✨",
                    Color32::from_rgb(50, 205, 50),
                    t::mcp_running().replace("{}", &self.mcp_port.to_string()),
                )
            } else {
                ("✨", Color32::GRAY, t::mcp_stopped().to_string())
            };

            let mcp_response = ui.add(
                egui::Button::new(RichText::new(mcp_icon).size(20.0).color(mcp_color))
                    .frame(false)
                    .min_size(Vec2::new(32.0, 32.0)),
            );

            if mcp_response.clicked() {
                action = ActivityBarAction::ToggleMcp;
            }

            mcp_response.on_hover_text(mcp_tooltip);

            ui.add_space(4.0);
        });

        action
    }

    /// Render an icon button
    fn icon_button(&self, ui: &mut Ui, icon: &str, tooltip: &str, active: bool) -> bool {
        let text_color = if active {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().text_color()
        };

        let bg_color = if active {
            ui.visuals().selection.bg_fill.linear_multiply(0.3)
        } else {
            Color32::TRANSPARENT
        };

        let response = ui.add(
            egui::Button::new(RichText::new(icon).size(20.0).color(text_color))
                .fill(bg_color)
                .min_size(Vec2::new(40.0, 40.0)),
        );

        let clicked = response.clicked();
        response.on_hover_text(tooltip);
        clicked
    }
}

/// Actions from the activity bar
#[derive(Debug, Clone)]
pub enum ActivityBarAction {
    None,
    SwitchView(ActivityView),
    TogglePanel,
    ToggleServer,
    ToggleMcp,
}
