//! Remote log server for receiving logs from agents
//!
//! This module implements a TCP server that listens for connections from
//! logline-agent instances and writes received logs to local cache files.

use crate::protocol::{Frame, MessageType, ProtocolError, DEFAULT_PORT};
use anyhow::{Context, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// Remote stream connection status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Agent is connected and streaming
    Online,
    /// Agent disconnected
    Offline,
}

/// Information about a connected remote stream
#[derive(Debug, Clone)]
pub struct RemoteStream {
    /// Unique stream identifier (project_name@ip:port)
    pub stream_id: String,
    /// Project name from handshake
    pub project_name: String,
    /// Connection status
    pub status: ConnectionStatus,
    /// Local cache file path
    pub cache_path: PathBuf,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Total bytes received
    pub bytes_received: u64,
}

impl RemoteStream {
    /// Get the IP address as a string
    pub fn ip_address(&self) -> String {
        self.remote_addr.ip().to_string()
    }

    /// Get a display name that includes full address (IP:port)
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        format!("{}@{}", self.project_name, self.remote_addr)
    }
}

/// Events from the remote server
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// New agent connected
    AgentConnected {
        project_name: String,
        stream_id: String,
        remote_addr: SocketAddr,
        #[allow(dead_code)]
        cache_path: PathBuf,
    },
    /// Agent disconnected
    AgentDisconnected {
        project_name: String,
        stream_id: String,
    },
    /// Log data received (for UI refresh notification)
    #[allow(dead_code)]
    LogDataReceived { project_name: String, bytes: usize },
    /// Server error
    Error(String),
    /// Server started
    Started { port: u16 },
    /// Server stopped
    Stopped,
}

/// Remote log server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,
    /// Cache directory for storing remote logs
    pub cache_dir: PathBuf,
    /// Read timeout for client connections
    pub read_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let cache_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("logline")
            .join("cache");

        Self {
            port: DEFAULT_PORT,
            cache_dir,
            read_timeout: Duration::from_secs(60),
        }
    }
}

/// Remote log server
pub struct RemoteServer {
    config: ServerConfig,
    running: Arc<AtomicBool>,
    streams: Arc<RwLock<HashMap<String, RemoteStream>>>,
    event_tx: Sender<ServerEvent>,
    event_rx: Receiver<ServerEvent>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl RemoteServer {
    /// Create a new remote server
    pub fn new(config: ServerConfig) -> Self {
        let (event_tx, event_rx) = bounded(1000);

        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            streams: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_rx,
            shutdown_tx: None,
        }
    }

    /// Create with default configuration
    #[allow(dead_code)]
    pub fn with_defaults() -> Self {
        Self::new(ServerConfig::default())
    }

    /// Update the server port (must be called before start)
    pub fn set_port(&mut self, port: u16) {
        self.config.port = port;
    }

    /// Get the current configured port
    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Start the server
    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Ensure cache directory exists
        fs::create_dir_all(&self.config.cache_dir).context("Failed to create cache directory")?;

        self.running.store(true, Ordering::Relaxed);

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let running = self.running.clone();
        let streams = self.streams.clone();
        let event_tx = self.event_tx.clone();
        let config = self.config.clone();

        // Spawn the async server in a separate thread with its own runtime
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                if let Err(e) =
                    Self::run_server(config, running, streams, event_tx, shutdown_rx).await
                {
                    tracing::error!("Server error: {}", e);
                }
            });
        });

        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.blocking_send(());
        }

        let _ = self.event_tx.send(ServerEvent::Stopped);
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get the event receiver
    pub fn event_receiver(&self) -> Receiver<ServerEvent> {
        self.event_rx.clone()
    }

    /// Get current remote streams
    pub fn streams(&self) -> Vec<RemoteStream> {
        self.streams.read().unwrap().values().cloned().collect()
    }

    /// Get a specific stream by project name
    #[allow(dead_code)]
    pub fn get_stream(&self, project_name: &str) -> Option<RemoteStream> {
        self.streams.read().unwrap().get(project_name).cloned()
    }

    /// Async server main loop
    async fn run_server(
        config: ServerConfig,
        running: Arc<AtomicBool>,
        streams: Arc<RwLock<HashMap<String, RemoteStream>>>,
        event_tx: Sender<ServerEvent>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port))
            .await
            .context("Failed to bind to port")?;

        tracing::info!("Remote server listening on port {}", config.port);
        let _ = event_tx.send(ServerEvent::Started { port: config.port });

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    tracing::info!("Received shutdown signal");
                    break;
                }

                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            tracing::info!("New connection from {}", addr);

                            let running = running.clone();
                            let streams = streams.clone();
                            let event_tx = event_tx.clone();
                            let config = config.clone();

                            // Spawn a task for each connection
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection_async(
                                    stream, addr, running, streams, event_tx, config
                                ).await {
                                    tracing::error!("Connection error from {}: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                            let _ = event_tx.send(ServerEvent::Error(e.to_string()));
                        }
                    }
                }
            }

            // Check if we should stop
            if !running.load(Ordering::Relaxed) {
                break;
            }
        }

        tracing::info!("Remote server stopped");
        Ok(())
    }

    /// Handle a single client connection asynchronously
    async fn handle_connection_async(
        stream: TcpStream,
        addr: SocketAddr,
        running: Arc<AtomicBool>,
        streams: Arc<RwLock<HashMap<String, RemoteStream>>>,
        event_tx: Sender<ServerEvent>,
        config: ServerConfig,
    ) -> Result<()> {
        // Set TCP nodelay for lower latency
        stream.set_nodelay(true)?;

        let mut reader = BufReader::new(stream);

        // Wait for handshake with timeout
        let project_name = match tokio::time::timeout(
            Duration::from_secs(30),
            Self::wait_for_handshake_async(&mut reader),
        )
        .await
        {
            Ok(Ok(name)) => name,
            Ok(Err(e)) => {
                tracing::warn!("Handshake failed from {}: {}", addr, e);
                return Ok(());
            }
            Err(_) => {
                tracing::warn!("Handshake timeout from {}", addr);
                return Ok(());
            }
        };

        tracing::info!("Agent '{}' connected from {}", project_name, addr);

        // Generate unique stream ID
        let stream_id = format!("{}@{}:{}", project_name, addr.ip(), addr.port());

        // Setup cache file
        let cache_path = config.cache_dir.join(format!(
            "{}_{}_{}.log",
            sanitize_filename(&project_name),
            addr.ip().to_string().replace(['.', ':'], "_"),
            addr.port()
        ));

        let cache_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&cache_path)
            .context("Failed to open cache file")?;

        let mut writer = std::io::BufWriter::new(cache_file);

        // Register stream
        {
            let mut streams_guard = streams.write().unwrap();
            streams_guard.insert(
                stream_id.clone(),
                RemoteStream {
                    stream_id: stream_id.clone(),
                    project_name: project_name.clone(),
                    status: ConnectionStatus::Online,
                    cache_path: cache_path.clone(),
                    last_activity: Instant::now(),
                    remote_addr: addr,
                    bytes_received: 0,
                },
            );
        }

        let _ = event_tx.send(ServerEvent::AgentConnected {
            project_name: project_name.clone(),
            stream_id: stream_id.clone(),
            remote_addr: addr,
            cache_path: cache_path.clone(),
        });

        // Main receive loop
        let mut total_bytes = 0u64;
        let mut last_flush = Instant::now();
        let mut last_stream_update = Instant::now();
        let flush_interval = Duration::from_millis(100);
        let stream_update_interval = Duration::from_millis(500);

        tracing::info!("Starting receive loop for '{}'", project_name);

        while running.load(Ordering::Relaxed) {
            // Read with timeout to allow checking running flag periodically
            let frame_result =
                tokio::time::timeout(config.read_timeout, Self::read_frame_async(&mut reader))
                    .await;

            match frame_result {
                Ok(Ok(frame)) => {
                    match frame.message_type {
                        MessageType::LogData => {
                            let data_len = frame.payload.len();
                            tracing::debug!("Received {} bytes from '{}'", data_len, project_name);

                            // Write to cache file
                            if let Err(e) = writer.write_all(&frame.payload) {
                                tracing::error!("Failed to write to cache: {}", e);
                                break;
                            }

                            // Flush periodically
                            let now = Instant::now();
                            if now.duration_since(last_flush) >= flush_interval {
                                if let Err(e) = writer.flush() {
                                    tracing::error!("Failed to flush cache: {}", e);
                                    break;
                                }
                                last_flush = now;
                            }

                            total_bytes += data_len as u64;

                            // Update stream info periodically
                            if now.duration_since(last_stream_update) >= stream_update_interval {
                                let mut streams_guard = streams.write().unwrap();
                                if let Some(stream) = streams_guard.get_mut(&stream_id) {
                                    stream.last_activity = now;
                                    stream.bytes_received = total_bytes;
                                }
                                last_stream_update = now;
                            }
                        }
                        MessageType::Keepalive => {
                            // Just continue the loop
                        }
                        MessageType::Handshake => {
                            // Ignore duplicate handshakes
                        }
                    }
                }
                Ok(Err(ProtocolError::Io(ref e)))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    tracing::info!("Agent '{}' disconnected", project_name);
                    break;
                }
                Ok(Err(e)) => {
                    tracing::error!("Protocol error from '{}': {}", project_name, e);
                    break;
                }
                Err(_) => {
                    // Timeout - flush any pending data and continue
                    let _ = writer.flush();
                    continue;
                }
            }
        }

        // Final flush before closing
        let _ = writer.flush();

        // Mark as offline
        {
            let mut streams_guard = streams.write().unwrap();
            if let Some(stream) = streams_guard.get_mut(&stream_id) {
                stream.status = ConnectionStatus::Offline;
            }
        }

        let _ = event_tx.send(ServerEvent::AgentDisconnected {
            project_name,
            stream_id,
        });

        Ok(())
    }

    /// Read a frame asynchronously
    async fn read_frame_async(reader: &mut BufReader<TcpStream>) -> Result<Frame, ProtocolError> {
        // Read length (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        reader
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| ProtocolError::Io(std::io::Error::new(e.kind(), e.to_string())))?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Validate frame size
        if len == 0 {
            return Err(ProtocolError::InvalidFrame("Empty frame".to_string()));
        }
        if len > crate::protocol::MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge(
                len,
                crate::protocol::MAX_FRAME_SIZE,
            ));
        }

        // Read message type (1 byte)
        let mut type_buf = [0u8; 1];
        reader
            .read_exact(&mut type_buf)
            .await
            .map_err(|e| ProtocolError::Io(std::io::Error::new(e.kind(), e.to_string())))?;
        let message_type = MessageType::try_from(type_buf[0])?;

        // Read payload
        let payload_len = len - 1;
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            reader
                .read_exact(&mut payload)
                .await
                .map_err(|e| ProtocolError::Io(std::io::Error::new(e.kind(), e.to_string())))?;
        }

        Ok(Frame {
            message_type,
            payload,
        })
    }

    /// Wait for and process handshake frame asynchronously
    async fn wait_for_handshake_async(
        reader: &mut BufReader<TcpStream>,
    ) -> Result<String, ProtocolError> {
        let frame = Self::read_frame_async(reader).await?;

        if frame.message_type != MessageType::Handshake {
            return Err(ProtocolError::InvalidFrame(
                "Expected handshake frame".to_string(),
            ));
        }

        let handshake = frame.parse_handshake()?;
        Ok(handshake.project_name)
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Sanitize a filename to be safe for filesystem
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test-project"), "test-project");
        assert_eq!(sanitize_filename("test/project"), "test_project");
        assert_eq!(sanitize_filename("test project"), "test_project");
        assert_eq!(sanitize_filename("test.project"), "test_project");
    }
}
