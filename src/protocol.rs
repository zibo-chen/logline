//! Logline Protocol (LLP) - Simple protocol for remote log streaming
//!
//! Frame Structure:
//! [Length: u32][Type: u8][Payload: bytes]
//!
//! - Length: Total length of Type + Payload (big-endian)
//! - Type: Message type identifier
//! - Payload: Message body

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use thiserror::Error;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Default server port
pub const DEFAULT_PORT: u16 = 12500;

/// Message type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Agent -> App: Initial handshake
    Handshake = 0x01,
    /// Agent -> App: Log data stream
    LogData = 0x02,
    /// Bidirectional: Keepalive/heartbeat
    Keepalive = 0xFF,
}

impl TryFrom<u8> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MessageType::Handshake),
            0x02 => Ok(MessageType::LogData),
            0xFF => Ok(MessageType::Keepalive),
            _ => Err(ProtocolError::UnknownMessageType(value)),
        }
    }
}

/// Protocol errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Unknown message type: {0}")]
    UnknownMessageType(u8),

    #[error("Invalid frame: {0}")]
    InvalidFrame(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Frame too large: {0} bytes (max: {1})")]
    FrameTooLarge(usize, usize),
}

/// Maximum frame size (10MB)
pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;

/// Handshake message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakePayload {
    /// Project/service name identifier
    pub project_name: String,
    /// Protocol version
    #[serde(default = "default_version")]
    pub version: u8,
    /// Unique agent ID (hash of log file path)
    #[serde(default)]
    pub agent_id: Option<String>,
}

fn default_version() -> u8 {
    PROTOCOL_VERSION
}

impl HandshakePayload {
    #[allow(dead_code)]
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project_name: project_name.into(),
            version: PROTOCOL_VERSION,
            agent_id: None,
        }
    }
}

/// Log data message payload
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LogDataPayload {
    /// Raw log bytes
    pub data: Vec<u8>,
}

impl LogDataPayload {
    #[allow(dead_code)]
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

/// A protocol frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame
    #[allow(dead_code)]
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            payload,
        }
    }

    /// Create a handshake frame
    #[allow(dead_code)]
    pub fn handshake(project_name: impl Into<String>) -> Result<Self, ProtocolError> {
        let payload = HandshakePayload::new(project_name);
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))?;
        Ok(Self::new(MessageType::Handshake, bytes))
    }

    /// Create a log data frame
    #[allow(dead_code)]
    pub fn log_data(data: Vec<u8>) -> Self {
        Self::new(MessageType::LogData, data)
    }

    /// Create a keepalive frame
    #[allow(dead_code)]
    pub fn keepalive() -> Self {
        Self::new(MessageType::Keepalive, Vec::new())
    }

    /// Parse handshake payload
    pub fn parse_handshake(&self) -> Result<HandshakePayload, ProtocolError> {
        if self.message_type != MessageType::Handshake {
            return Err(ProtocolError::InvalidFrame(
                "Not a handshake frame".to_string(),
            ));
        }
        serde_json::from_slice(&self.payload)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))
    }

    /// Encode frame to bytes
    #[allow(dead_code)]
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() + 1; // +1 for message type
        let mut buf = Vec::with_capacity(4 + payload_len);

        // Length (big-endian u32)
        buf.extend_from_slice(&(payload_len as u32).to_be_bytes());
        // Message type
        buf.push(self.message_type as u8);
        // Payload
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Decode frame from reader
    pub fn decode<R: Read>(reader: &mut R) -> Result<Self, ProtocolError> {
        // Read length (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Validate frame size
        if len == 0 {
            return Err(ProtocolError::InvalidFrame("Empty frame".to_string()));
        }
        if len > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge(len, MAX_FRAME_SIZE));
        }

        // Read message type (1 byte)
        let mut type_buf = [0u8; 1];
        reader.read_exact(&mut type_buf)?;
        let message_type = MessageType::try_from(type_buf[0])?;

        // Read payload
        let payload_len = len - 1;
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            reader.read_exact(&mut payload)?;
        }

        Ok(Self {
            message_type,
            payload,
        })
    }

    /// Write frame to writer
    #[allow(dead_code)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ProtocolError> {
        let encoded = self.encode();
        writer.write_all(&encoded)?;
        writer.flush()?;
        Ok(())
    }
}

/// Frame reader for async/buffered reading
#[allow(dead_code)]
pub struct FrameReader<R> {
    reader: R,
}

impl<R: Read> FrameReader<R> {
    #[allow(dead_code)]
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    #[allow(dead_code)]
    pub fn read_frame(&mut self) -> Result<Frame, ProtocolError> {
        Frame::decode(&mut self.reader)
    }
}

/// Frame writer for buffered writing
#[allow(dead_code)]
pub struct FrameWriter<W> {
    writer: W,
}

impl<W: Write> FrameWriter<W> {
    #[allow(dead_code)]
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    #[allow(dead_code)]
    pub fn write_frame(&mut self, frame: &Frame) -> Result<(), ProtocolError> {
        frame.write_to(&mut self.writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_handshake_frame() {
        let frame = Frame::handshake("test-project").unwrap();
        let encoded = frame.encode();

        let mut cursor = Cursor::new(encoded);
        let decoded = Frame::decode(&mut cursor).unwrap();

        assert_eq!(decoded.message_type, MessageType::Handshake);
        let payload = decoded.parse_handshake().unwrap();
        assert_eq!(payload.project_name, "test-project");
    }

    #[test]
    fn test_log_data_frame() {
        let data = b"2024-01-01 12:00:00 INFO test log message".to_vec();
        let frame = Frame::log_data(data.clone());
        let encoded = frame.encode();

        let mut cursor = Cursor::new(encoded);
        let decoded = Frame::decode(&mut cursor).unwrap();

        assert_eq!(decoded.message_type, MessageType::LogData);
        assert_eq!(decoded.payload, data);
    }

    #[test]
    fn test_keepalive_frame() {
        let frame = Frame::keepalive();
        let encoded = frame.encode();

        let mut cursor = Cursor::new(encoded);
        let decoded = Frame::decode(&mut cursor).unwrap();

        assert_eq!(decoded.message_type, MessageType::Keepalive);
        assert!(decoded.payload.is_empty());
    }
}
