//! MCP (Model Context Protocol) Server Module
//!
//! This module provides MCP tools for AI assistants to analyze application logs.
//! It integrates with the Logline core functionality to expose log analysis
//! capabilities via the SSE transport.

mod server;
mod tools;
mod types;

pub use server::McpServer;
pub use types::*;
