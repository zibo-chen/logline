//! MCP Server implementation with SSE transport
//!
//! Uses rmcp's StreamableHttpService with axum for SSE-based MCP communication.

use crate::mcp::tools::{LoglineToolState, LoglineTools};
use crate::mcp::types::McpConfig;
use crate::remote_server::RemoteStream;

use anyhow::Result;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// MCP Server for Logline
///
/// Provides log analysis tools to AI assistants via SSE transport.
pub struct McpServer {
    config: McpConfig,
    state: Arc<LoglineToolState>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    cancellation_token: CancellationToken,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: McpConfig, cache_dir: PathBuf) -> Self {
        Self {
            config,
            state: Arc::new(LoglineToolState::new(cache_dir)),
            shutdown_tx: None,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Create with default configuration
    #[allow(dead_code)]
    pub fn with_defaults(cache_dir: PathBuf) -> Self {
        Self::new(McpConfig::default(), cache_dir)
    }

    /// Get a clone of the tool state for updating
    #[allow(dead_code)]
    pub fn state(&self) -> Arc<LoglineToolState> {
        self.state.clone()
    }

    /// Update remote streams
    pub fn update_remote_streams(&self, streams: Vec<RemoteStream>) {
        self.state.update_remote_streams(streams);
    }

    /// Add a local file to track
    pub fn add_local_file(&self, path: PathBuf) {
        self.state.add_local_file(path);
    }

    /// Get the server address
    #[allow(dead_code)]
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }

    /// Start the MCP server
    ///
    /// This runs the server in a background task.
    /// Returns immediately after spawning the server.
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let state = self.state.clone();
        let ct = self.cancellation_token.clone();

        // Create the streamable HTTP service
        let service = StreamableHttpService::new(
            move || Ok(LoglineTools::new(state.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig {
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );

        // Create axum router
        let router = axum::Router::new().nest_service("/mcp", service);

        // Bind TCP listener
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

        tracing::info!("MCP server started at http://{}/mcp", bind_addr);

        // Spawn server task
        let ct_for_shutdown = ct.clone();
        tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                tokio::select! {
                    _ = shutdown_rx => {
                        tracing::info!("MCP server shutdown requested");
                    }
                    _ = ct_for_shutdown.cancelled() => {
                        tracing::info!("MCP server cancelled");
                    }
                }
            });

            if let Err(e) = server.await {
                tracing::error!("MCP server error: {}", e);
            }

            tracing::info!("MCP server stopped");
        });

        Ok(())
    }

    /// Stop the MCP server
    pub fn stop(&mut self) {
        tracing::info!("Stopping MCP server");

        // Cancel all ongoing operations
        self.cancellation_token.cancel();

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Give server a moment to clean up
        std::thread::sleep(std::time::Duration::from_millis(50));

        tracing::info!("MCP server stopped");
    }

    /// Check if server is running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        !self.cancellation_token.is_cancelled()
    }

    /// Get the MCP endpoint URL
    pub fn endpoint_url(&self) -> String {
        format!(
            "http://{}:{}/mcp",
            self.config.bind_address, self.config.port
        )
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_server_creation() {
        let temp_dir = tempdir().unwrap();
        let server = McpServer::with_defaults(temp_dir.path().to_path_buf());

        assert_eq!(server.address(), "127.0.0.1:12600");
        assert!(server.endpoint_url().contains("/mcp"));
    }
}
