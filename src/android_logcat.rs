//! Android logcat reader using ADB protocol
//!
//! This module uses the adb_client crate to communicate directly with the
//! ADB server using the ADB protocol. Supports both USB and TCP connected devices.

use adb_client::server::{ADBServer, DeviceLong, DeviceState};
use adb_client::server_device::ADBServerDevice;
use adb_client::ADBDeviceExt;
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::io::Write;
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Default ADB server address
pub const DEFAULT_ADB_ADDRESS: &str = "127.0.0.1:5037";

/// Android device information
#[derive(Debug, Clone)]
pub struct AndroidDevice {
    /// Device serial number (identifier)
    pub serial: String,
    /// Device model name
    pub model: String,
    /// Device product name
    pub product: String,
    /// Device state (device, offline, unauthorized, etc.)
    #[allow(dead_code)]
    pub state: String,
    /// Connection type (USB or TCP)
    pub connection_type: ConnectionType,
    /// Is device online and ready
    pub is_online: bool,
    /// Transport ID
    #[allow(dead_code)]
    pub transport_id: u32,
}

/// Connection type for Android device
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// USB connection
    Usb,
    /// TCP/IP connection (WiFi debugging)
    Tcp,
    /// Unknown connection type
    Unknown,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionType::Usb => write!(f, "USB"),
            ConnectionType::Tcp => write!(f, "TCP"),
            ConnectionType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Logcat priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LogcatPriority {
    /// Verbose
    Verbose,
    /// Debug
    Debug,
    /// Info
    Info,
    /// Warning
    Warn,
    /// Error
    Error,
    /// Fatal
    Fatal,
    /// Silent (suppress all)
    Silent,
}

impl std::fmt::Display for LogcatPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            LogcatPriority::Verbose => "V",
            LogcatPriority::Debug => "D",
            LogcatPriority::Info => "I",
            LogcatPriority::Warn => "W",
            LogcatPriority::Error => "E",
            LogcatPriority::Fatal => "F",
            LogcatPriority::Silent => "S",
        };
        write!(f, "{}", c)
    }
}

/// Logcat filter options
#[derive(Debug, Clone, Default)]
pub struct LogcatOptions {
    /// Filter by minimum log level
    pub priority: Option<LogcatPriority>,
    /// Filter by tag (can use wildcards)
    pub tag_filter: Option<String>,
    /// Filter by process ID
    #[allow(dead_code)]
    pub pid: Option<u32>,
    /// Clear log before reading
    pub clear_before_read: bool,
    /// Use threadtime format (includes PID and TID)
    pub threadtime_format: bool,
}

/// ADB Manager for handling Android devices
#[derive(Debug)]
pub struct AdbManager {
    /// ADB server address
    server_addr: SocketAddrV4,
}

impl Default for AdbManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AdbManager {
    /// Create a new ADB manager with default server address
    pub fn new() -> Self {
        Self {
            server_addr: SocketAddrV4::from_str(DEFAULT_ADB_ADDRESS)
                .expect("Invalid default ADB address"),
        }
    }

    /// Create a new ADB manager with custom server address
    #[allow(dead_code)]
    pub fn with_address(addr: &str) -> Result<Self> {
        let server_addr = SocketAddrV4::from_str(addr)
            .context(format!("Invalid ADB server address: {}", addr))?;
        Ok(Self { server_addr })
    }

    /// Get the ADB server address
    #[allow(dead_code)]
    pub fn server_address(&self) -> String {
        self.server_addr.to_string()
    }

    /// List all connected Android devices with detailed info
    pub fn list_devices(&self) -> Result<Vec<AndroidDevice>> {
        let mut server = ADBServer::new(self.server_addr);

        let devices = server
            .devices_long()
            .context("Failed to get device list. Make sure ADB server is running.")?;

        let android_devices: Vec<AndroidDevice> = devices
            .into_iter()
            .map(|d| self.device_long_to_android_device(d))
            .collect();

        Ok(android_devices)
    }

    /// Convert DeviceLong to AndroidDevice
    fn device_long_to_android_device(&self, device: DeviceLong) -> AndroidDevice {
        let connection_type = if device.identifier.contains(':') {
            ConnectionType::Tcp
        } else if device.usb.starts_with("usb:") || !device.usb.is_empty() {
            ConnectionType::Usb
        } else {
            ConnectionType::Unknown
        };

        let is_online = matches!(device.state, DeviceState::Device);

        AndroidDevice {
            serial: device.identifier,
            model: if device.model == "Unk" {
                "Unknown".to_string()
            } else {
                device.model
            },
            product: if device.product == "Unk" {
                "Unknown".to_string()
            } else {
                device.product
            },
            state: format!("{}", device.state),
            connection_type,
            is_online,
            transport_id: device.transport_id,
        }
    }

    /// Connect to a device over TCP (WiFi debugging)
    pub fn connect_tcp(&self, address: &str) -> Result<String> {
        let addr = if address.contains(':') {
            SocketAddrV4::from_str(address)
                .context(format!("Invalid address format: {}", address))?
        } else {
            // Default ADB port is 5555
            SocketAddrV4::from_str(&format!("{}:5555", address))
                .context(format!("Invalid IP address: {}", address))?
        };

        let mut server = ADBServer::new(self.server_addr);
        server
            .connect_device(addr)
            .context(format!("Failed to connect to device at {}", addr))?;

        Ok(format!("Connected to {}", addr))
    }

    /// Disconnect a TCP connected device
    pub fn disconnect_tcp(&self, address: &str) -> Result<String> {
        let addr = if address.contains(':') {
            SocketAddrV4::from_str(address)
                .context(format!("Invalid address format: {}", address))?
        } else {
            SocketAddrV4::from_str(&format!("{}:5555", address))
                .context(format!("Invalid IP address: {}", address))?
        };

        let mut server = ADBServer::new(self.server_addr);
        server
            .disconnect_device(addr)
            .context(format!("Failed to disconnect device at {}", addr))?;

        Ok(format!("Disconnected from {}", addr))
    }

    /// Get a device by serial number
    #[allow(dead_code)]
    pub fn get_device(&self, serial: &str) -> Result<ADBServerDevice> {
        let mut server = ADBServer::new(self.server_addr);
        server
            .get_device_by_name(serial)
            .context(format!("Device not found: {}", serial))
    }

    /// Check if ADB server is running
    #[allow(dead_code)]
    pub fn is_server_running(&self) -> bool {
        let mut server = ADBServer::new(self.server_addr);
        server.version().is_ok()
    }

    /// Get ADB server version
    #[allow(dead_code)]
    pub fn server_version(&self) -> Result<String> {
        let mut server = ADBServer::new(self.server_addr);
        let version = server
            .version()
            .context("Failed to get ADB server version")?;
        Ok(format!("{}", version))
    }

    /// Kill ADB server
    #[allow(dead_code)]
    pub fn kill_server(&self) -> Result<()> {
        let mut server = ADBServer::new(self.server_addr);
        server.kill().context("Failed to kill ADB server")
    }
}

/// Android logcat reader that streams logs from a device
pub struct LogcatReader {
    /// Device serial number
    device_serial: String,
    /// ADB server address
    server_addr: SocketAddrV4,
    /// Logcat options
    options: LogcatOptions,
    /// Channel to send log entries
    log_sender: Sender<String>,
    /// Channel to receive log entries
    log_receiver: Receiver<String>,
    /// Is running flag
    is_running: Arc<AtomicBool>,
    /// Stop signal
    stop_signal: Arc<AtomicBool>,
}

impl LogcatReader {
    /// Create a new logcat reader for the specified device
    pub fn new(device_serial: String, options: LogcatOptions) -> Self {
        let (log_sender, log_receiver) = crossbeam_channel::unbounded();

        Self {
            device_serial,
            server_addr: SocketAddrV4::from_str(DEFAULT_ADB_ADDRESS)
                .expect("Invalid default ADB address"),
            options,
            log_sender,
            log_receiver,
            is_running: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a logcat reader with custom ADB server address
    #[allow(dead_code)]
    pub fn with_server(
        device_serial: String,
        server_addr: &str,
        options: LogcatOptions,
    ) -> Result<Self> {
        let addr = SocketAddrV4::from_str(server_addr)
            .context(format!("Invalid ADB server address: {}", server_addr))?;

        let (log_sender, log_receiver) = crossbeam_channel::unbounded();

        Ok(Self {
            device_serial,
            server_addr: addr,
            options,
            log_sender,
            log_receiver,
            is_running: Arc::new(AtomicBool::new(false)),
            stop_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get the log receiver channel
    pub fn get_receiver(&self) -> Receiver<String> {
        self.log_receiver.clone()
    }

    /// Check if reader is running
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Start streaming logcat in background
    pub fn start_streaming(&self) -> Result<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.is_running.store(true, Ordering::Relaxed);
        self.stop_signal.store(false, Ordering::Relaxed);

        let device_serial = self.device_serial.clone();
        let server_addr = self.server_addr;
        let options = self.options.clone();
        let log_sender = self.log_sender.clone();
        let is_running = Arc::clone(&self.is_running);
        let stop_signal = Arc::clone(&self.stop_signal);

        thread::spawn(move || {
            Self::streaming_loop(
                &device_serial,
                server_addr,
                &options,
                &log_sender,
                &is_running,
                &stop_signal,
            );
        });

        Ok(())
    }

    /// Stop streaming logcat
    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        // Give the thread time to stop
        thread::sleep(Duration::from_millis(100));
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// Internal streaming loop
    fn streaming_loop(
        device_serial: &str,
        server_addr: SocketAddrV4,
        options: &LogcatOptions,
        log_sender: &Sender<String>,
        is_running: &Arc<AtomicBool>,
        stop_signal: &Arc<AtomicBool>,
    ) {
        // Build logcat command arguments
        let format_arg = if options.threadtime_format {
            "threadtime"
        } else {
            "time"
        };

        let priority_filter = options.priority.as_ref().map(|p| format!("*:{}", p));
        let tag_filter = options.tag_filter.as_ref().map(|t| format!("{}:*", t));

        // Track if this is the first connection (for clearing logs)
        let mut first_run = true;
        // Track the number of lines received for reconnection handling
        let mut total_lines_received: usize = 0;

        while !stop_signal.load(Ordering::Relaxed) {
            // Create device connection
            let mut device = ADBServerDevice::new(device_serial.to_string(), Some(server_addr));

            // Clear log only on first run if requested
            if first_run && options.clear_before_read {
                let mut output = Vec::new();
                let _ = device.shell_command(&"logcat -c", &mut output);
                // Wait a moment after clearing
                thread::sleep(Duration::from_millis(100));
            }

            first_run = false;

            // Build command string - avoid quotes by using simpler options
            // Using -T with a number (count) instead of timestamp to avoid shell quoting issues
            let mut cmd = format!("logcat -v {}", format_arg);

            // Use -T with a count to limit history
            // -T 1 means start from the 1 most recent line
            if total_lines_received > 0 {
                // After first connection, only show new logs (start from last line)
                cmd.push_str(" -T 1");
            }

            if let Some(ref pf) = priority_filter {
                cmd.push_str(&format!(" {}", pf));
            }

            if let Some(ref tf) = tag_filter {
                cmd.push_str(&format!(" {} *:S", tf));
            }

            // Create a custom writer that sends lines to the channel and tracks count
            let mut line_writer = LogcatLineWriter::new(log_sender.clone(), stop_signal.clone());

            // Run logcat command with streaming output
            // This will block until the connection is lost or stop signal is set
            match device.shell_command(&cmd, &mut line_writer) {
                Ok(_) => {
                    // Command completed (connection closed or device disconnected)
                }
                Err(e) => {
                    if !stop_signal.load(Ordering::Relaxed) {
                        eprintln!("Logcat connection error: {}", e);
                    }
                }
            }

            // Update total lines received
            total_lines_received += line_writer.get_lines_count();

            if stop_signal.load(Ordering::Relaxed) {
                break;
            }

            // Wait before reconnecting to avoid rapid reconnection loops
            thread::sleep(Duration::from_secs(2));
        }

        is_running.store(false, Ordering::Relaxed);
    }
}

impl Drop for LogcatReader {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Custom writer that buffers and sends complete lines to a channel
/// Also tracks the number of lines for reconnection handling
/// Handles shell_v2 protocol which prefixes data with ID bytes
struct LogcatLineWriter {
    sender: Sender<String>,
    stop_signal: Arc<AtomicBool>,
    buffer: String,
    /// Number of lines processed
    lines_count: usize,
    /// Track if we're at the start of a new shell_v2 packet
    /// shell_v2 format: [id:1][length:4][data:length]
    /// id: 0=stdin, 1=stdout, 2=stderr, 3=exit
    packet_state: PacketState,
}

/// State machine for parsing shell_v2 packets
#[derive(Debug, Clone)]
enum PacketState {
    /// Waiting for packet ID byte
    WaitingForId,
    /// Reading the 4-byte length field
    ReadingLength { id: u8, length_bytes: Vec<u8> },
    /// Reading packet data
    ReadingData { id: u8, remaining: usize },
}

impl LogcatLineWriter {
    fn new(sender: Sender<String>, stop_signal: Arc<AtomicBool>) -> Self {
        Self {
            sender,
            stop_signal,
            buffer: String::new(),
            lines_count: 0,
            packet_state: PacketState::WaitingForId,
        }
    }

    fn flush_line(&mut self) {
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            // Filter out lines that are clearly not logcat output
            // (e.g., shell prompts, error messages)
            if Self::is_valid_logcat_line(&line) {
                self.lines_count += 1;
                let _ = self.sender.send(line);
            }
        }
    }

    /// Check if a line looks like valid logcat output
    fn is_valid_logcat_line(line: &str) -> bool {
        // Empty lines are not valid
        if line.trim().is_empty() {
            return false;
        }

        // Logcat lines typically start with a timestamp (MM-DD HH:MM:SS)
        // or "--------- beginning of" marker
        let trimmed = line.trim_start();

        // Check for "beginning of" marker
        if trimmed.starts_with("--------- beginning of") {
            return true;
        }

        // Check for timestamp format: "MM-DD HH:MM:SS"
        if trimmed.len() >= 18 {
            let bytes = trimmed.as_bytes();
            // Format: "01-21 00:31:21.947"
            if bytes.len() >= 18
                && bytes[2] == b'-'
                && bytes[5] == b' '
                && bytes[8] == b':'
                && bytes[11] == b':'
                && bytes[14] == b'.'
            {
                return true;
            }
        }

        // If it doesn't match known patterns, still include it
        // but filter out obvious garbage (single control characters, etc.)
        line.len() > 1 && line.chars().all(|c| !c.is_control() || c == '\t')
    }

    /// Process a byte as part of stdout/stderr data
    fn process_data_byte(&mut self, byte: u8) {
        let ch = byte as char;
        if ch == '\n' {
            self.flush_line();
        } else if ch != '\r' && !ch.is_control() {
            // Filter out control characters except newline
            self.buffer.push(ch);
        } else if ch == '\t' {
            // Allow tabs
            self.buffer.push(ch);
        }
    }

    /// Get the number of lines processed
    fn get_lines_count(&self) -> usize {
        self.lines_count
    }
}

impl Write for LogcatLineWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.stop_signal.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Stop signal received",
            ));
        }

        let mut i = 0;
        while i < buf.len() {
            // Clone the state to avoid borrow issues
            let current_state = self.packet_state.clone();
            match current_state {
                PacketState::WaitingForId => {
                    let id = buf[i];
                    i += 1;
                    // id: 0=stdin, 1=stdout, 2=stderr, 3=exit
                    if id <= 3 {
                        self.packet_state = PacketState::ReadingLength {
                            id,
                            length_bytes: Vec::with_capacity(4),
                        };
                    } else {
                        // Not a valid shell_v2 packet, treat as raw data
                        // This handles the case where shell_v2 is not being used
                        self.process_data_byte(id);
                    }
                }
                PacketState::ReadingLength {
                    id,
                    mut length_bytes,
                } => {
                    length_bytes.push(buf[i]);
                    i += 1;
                    if length_bytes.len() == 4 {
                        let length = u32::from_le_bytes([
                            length_bytes[0],
                            length_bytes[1],
                            length_bytes[2],
                            length_bytes[3],
                        ]) as usize;
                        if length == 0 {
                            self.packet_state = PacketState::WaitingForId;
                        } else {
                            self.packet_state = PacketState::ReadingData {
                                id,
                                remaining: length,
                            };
                        }
                    } else {
                        self.packet_state = PacketState::ReadingLength { id, length_bytes };
                    }
                }
                PacketState::ReadingData { id, remaining } => {
                    // Only process stdout (1) and stderr (2) data
                    if id == 1 || id == 2 {
                        self.process_data_byte(buf[i]);
                    }
                    i += 1;
                    if remaining <= 1 {
                        self.packet_state = PacketState::WaitingForId;
                    } else {
                        self.packet_state = PacketState::ReadingData {
                            id,
                            remaining: remaining - 1,
                        };
                    }
                }
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Helper function to check if ADB server is available
#[allow(dead_code)]
pub fn is_adb_available() -> bool {
    let manager = AdbManager::new();
    manager.is_server_running()
}

/// Helper function to start ADB server (requires adb in PATH)
#[allow(dead_code)]
pub fn start_adb_server() -> Result<()> {
    use std::process::Command;

    let output = Command::new("adb")
        .arg("start-server")
        .output()
        .context("Failed to start adb server. Make sure 'adb' is installed and in PATH.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to start adb server: {}", stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires ADB server and connected device
    fn test_list_devices() {
        let manager = AdbManager::new();
        match manager.list_devices() {
            Ok(devices) => {
                println!("Found {} devices:", devices.len());
                for device in devices {
                    println!(
                        "  {} - {} ({}) [{}] transport:{}",
                        device.serial,
                        device.model,
                        device.connection_type,
                        device.state,
                        device.transport_id
                    );
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    #[test]
    #[ignore] // Requires ADB server and connected device
    fn test_logcat_streaming() {
        let manager = AdbManager::new();
        let devices = manager.list_devices().unwrap();

        if let Some(device) = devices.first() {
            let reader = LogcatReader::new(device.serial.clone(), LogcatOptions::default());

            reader.start_streaming().unwrap();

            let receiver = reader.get_receiver();
            for _ in 0..10 {
                if let Ok(line) = receiver.recv_timeout(Duration::from_secs(5)) {
                    println!("LOG: {}", line);
                }
            }

            reader.stop();
        }
    }
}
