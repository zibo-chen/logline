//! Log file reading and parsing module

use crate::log_entry::LogEntry;
use anyhow::{Context, Result};
use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Configuration for the log reader
#[derive(Debug, Clone)]
pub struct LogReaderConfig {
    /// Buffer size for reading
    pub buffer_size: usize,
    /// Encoding to use (None for auto-detect)
    pub encoding: Option<&'static Encoding>,
    /// Maximum line length before truncation
    pub max_line_length: usize,
}

impl Default for LogReaderConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB buffer
            encoding: None,
            max_line_length: 10_000, // 10KB max line
        }
    }
}

/// Log file reader with incremental reading support
pub struct LogReader {
    /// Path to the log file
    path: PathBuf,
    /// Current byte offset in the file
    offset: u64,
    /// Line counter
    line_count: usize,
    /// Configuration
    config: LogReaderConfig,
    /// Detected or specified encoding
    encoding: &'static Encoding,
    /// File size at last read
    last_file_size: u64,
}

impl LogReader {
    /// Create a new log reader for the given file
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(path, LogReaderConfig::default())
    }

    /// Create a new log reader with custom configuration
    pub fn with_config(path: impl AsRef<Path>, config: LogReaderConfig) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Detect encoding from the first few bytes
        let encoding = if let Some(enc) = config.encoding {
            enc
        } else {
            Self::detect_encoding(&path)?
        };

        let metadata = std::fs::metadata(&path).context("Failed to get file metadata")?;

        Ok(Self {
            path,
            offset: 0,
            line_count: 0,
            config,
            encoding,
            last_file_size: metadata.len(),
        })
    }

    /// Detect the encoding of a file
    fn detect_encoding(path: &Path) -> Result<&'static Encoding> {
        let mut file = File::open(path).context("Failed to open file for encoding detection")?;
        let mut buffer = [0u8; 8192];
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            return Ok(encoding_rs::UTF_8);
        }

        let mut detector = EncodingDetector::new();
        detector.feed(&buffer[..bytes_read], true);
        let encoding = detector.guess(None, true);

        Ok(encoding)
    }

    /// Read all lines from the current offset to the end
    pub fn read_new_lines(&mut self) -> Result<Vec<LogEntry>> {
        let file = File::open(&self.path).context("Failed to open log file")?;
        let metadata = file.metadata()?;
        let current_size = metadata.len();

        // Handle file truncation (log rotation)
        if current_size < self.offset {
            self.offset = 0;
            self.line_count = 0;
        }

        // No new content
        if current_size == self.offset {
            return Ok(Vec::new());
        }

        self.last_file_size = current_size;

        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);
        reader.seek(SeekFrom::Start(self.offset))?;

        let mut entries = Vec::new();
        let mut line_buffer = Vec::new();

        loop {
            line_buffer.clear();
            let bytes_read = reader.read_until(b'\n', &mut line_buffer)?;

            if bytes_read == 0 {
                break;
            }

            let line_offset = self.offset;
            self.offset += bytes_read as u64;
            self.line_count += 1;

            // Decode the line
            let content = self.decode_line(&line_buffer);

            // Truncate if too long
            let content = if content.len() > self.config.max_line_length {
                format!(
                    "{}... [truncated, {} bytes total]",
                    &content[..self.config.max_line_length],
                    content.len()
                )
            } else {
                content
            };

            entries.push(LogEntry::new(self.line_count, content, line_offset));
        }

        Ok(entries)
    }

    /// Decode a line from bytes to string
    fn decode_line(&self, bytes: &[u8]) -> String {
        let (decoded, _, had_errors) = self.encoding.decode(bytes);

        let mut line = decoded.into_owned();

        // Remove trailing newline characters
        if line.ends_with('\n') {
            line.pop();
        }
        if line.ends_with('\r') {
            line.pop();
        }

        // Replace invalid characters if there were decoding errors
        if had_errors {
            line = line.replace('\u{FFFD}', "?");
        }

        line
    }

    /// Seek to a specific byte offset
    #[allow(dead_code)]
    pub fn seek(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Seek to a specific byte offset and set line count
    pub fn seek_with_line_count(&mut self, offset: u64, line_count: usize) {
        self.offset = offset;
        self.line_count = line_count;
    }

    /// Seek to the end of the file
    #[allow(dead_code)]
    pub fn seek_to_end(&mut self) -> Result<()> {
        let metadata = std::fs::metadata(&self.path)?;
        self.offset = metadata.len();
        Ok(())
    }

    /// Get the current byte offset
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get the current line count
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// Get the file path
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the file size
    pub fn file_size(&self) -> u64 {
        self.last_file_size
    }

    /// Check if there's new content available
    pub fn has_new_content(&self) -> Result<bool> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len() > self.offset || metadata.len() < self.offset)
    }

    /// Get the detected/configured encoding name
    pub fn encoding_name(&self) -> &'static str {
        self.encoding.name()
    }

    /// Read a specific line range (1-indexed, inclusive)
    #[allow(dead_code)]
    pub fn read_line_range(&mut self, start: usize, end: usize) -> Result<Vec<LogEntry>> {
        // Reset to beginning
        self.offset = 0;
        self.line_count = 0;

        let file = File::open(&self.path)?;
        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut entries = Vec::new();
        let mut line_buffer = Vec::new();

        while self.line_count < end {
            line_buffer.clear();
            let bytes_read = reader.read_until(b'\n', &mut line_buffer)?;

            if bytes_read == 0 {
                break;
            }

            self.line_count += 1;
            let line_offset = self.offset;
            self.offset += bytes_read as u64;

            if self.line_count >= start {
                let content = self.decode_line(&line_buffer);
                entries.push(LogEntry::new(self.line_count, content, line_offset));
            }
        }

        Ok(entries)
    }

    /// Read the last N lines from the file (tail mode)
    /// Returns (entries, start_byte_offset, total_lines_in_file)
    /// This is optimized for large files - it reads from the end backwards
    pub fn read_tail(&mut self, max_lines: usize) -> Result<(Vec<LogEntry>, u64, usize)> {
        let file = File::open(&self.path).context("Failed to open log file")?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        if file_size == 0 {
            return Ok((Vec::new(), 0, 0));
        }

        self.last_file_size = file_size;

        // Strategy: Read from the end in chunks to find line boundaries
        // Start with a reasonable chunk size (1MB) and expand if needed
        const CHUNK_SIZE: u64 = 1024 * 1024; // 1MB chunks

        let mut all_lines: Vec<(u64, Vec<u8>)> = Vec::new(); // (byte_offset, line_bytes)
        let mut search_start: u64 = file_size;
        let mut found_enough = false;

        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);

        while !found_enough && search_start > 0 {
            // Calculate chunk boundaries
            let chunk_start = search_start.saturating_sub(CHUNK_SIZE);
            let chunk_end = search_start;

            // Seek to chunk start
            reader.seek(SeekFrom::Start(chunk_start))?;

            // Read the chunk
            let chunk_len = (chunk_end - chunk_start) as usize;
            let mut chunk = vec![0u8; chunk_len];
            reader.read_exact(&mut chunk)?;

            // Parse lines from this chunk (backwards)
            let mut lines_in_chunk: Vec<(u64, Vec<u8>)> = Vec::new();
            let mut line_end = chunk.len();

            for (i, &byte) in chunk.iter().enumerate().rev() {
                if byte == b'\n' || i == 0 {
                    let line_start = if byte == b'\n' { i + 1 } else { i };
                    if line_start < line_end {
                        let line_bytes = chunk[line_start..line_end].to_vec();
                        let byte_offset = chunk_start + line_start as u64;
                        lines_in_chunk.push((byte_offset, line_bytes));
                    }
                    line_end = i;
                }
            }

            // lines_in_chunk is in reverse order (last line first), which is what we want
            all_lines.extend(lines_in_chunk);

            if all_lines.len() >= max_lines {
                found_enough = true;
            }

            search_start = chunk_start;
        }

        // Now we need to count total lines and also get the correct line numbers
        // We need to scan from the beginning to count lines up to our start point
        let start_offset = if all_lines.len() > max_lines {
            all_lines.truncate(max_lines);
            all_lines.last().map(|(off, _)| *off).unwrap_or(0)
        } else {
            all_lines.last().map(|(off, _)| *off).unwrap_or(0)
        };

        // Count total lines by scanning from beginning
        let file = File::open(&self.path)?;
        let mut count_reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut total_lines = 0;
        let mut lines_before_start = 0;
        let mut current_offset: u64 = 0;
        let mut line_buffer = Vec::new();

        loop {
            line_buffer.clear();
            let bytes_read = count_reader.read_until(b'\n', &mut line_buffer)?;
            if bytes_read == 0 {
                break;
            }
            total_lines += 1;
            if current_offset < start_offset {
                lines_before_start += 1;
            }
            current_offset += bytes_read as u64;
        }

        // Reverse all_lines to get chronological order and create entries
        all_lines.reverse();
        let entries: Vec<LogEntry> = all_lines
            .into_iter()
            .enumerate()
            .map(|(i, (byte_offset, line_bytes))| {
                let line_number = lines_before_start + i + 1;
                let content = self.decode_line(&line_bytes);
                // Truncate if too long
                let content = if content.len() > self.config.max_line_length {
                    format!(
                        "{}... [truncated, {} bytes total]",
                        &content[..self.config.max_line_length],
                        content.len()
                    )
                } else {
                    content
                };
                LogEntry::new(line_number, content, byte_offset)
            })
            .collect();

        // Update reader state to be at the end of file for incremental reads
        self.offset = file_size;
        self.line_count = total_lines;

        Ok((entries, start_offset, total_lines))
    }

    /// Read a chunk of lines before the given byte offset
    /// Used for lazy loading when user scrolls up
    /// Returns (entries, new_start_offset)
    pub fn read_previous_chunk(
        &mut self,
        before_offset: u64,
        max_lines: usize,
    ) -> Result<(Vec<LogEntry>, u64)> {
        if before_offset == 0 {
            return Ok((Vec::new(), 0));
        }

        let file = File::open(&self.path).context("Failed to open log file")?;
        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);

        // Strategy: Read backwards from before_offset
        const CHUNK_SIZE: u64 = 512 * 1024; // 512KB chunks

        let mut all_lines: Vec<(u64, Vec<u8>)> = Vec::new();
        let mut search_start = before_offset;

        while all_lines.len() < max_lines && search_start > 0 {
            let chunk_start = search_start.saturating_sub(CHUNK_SIZE);
            let chunk_end = search_start;

            reader.seek(SeekFrom::Start(chunk_start))?;

            let chunk_len = (chunk_end - chunk_start) as usize;
            let mut chunk = vec![0u8; chunk_len];
            reader.read_exact(&mut chunk)?;

            // Parse lines from this chunk (backwards)
            let mut lines_in_chunk: Vec<(u64, Vec<u8>)> = Vec::new();
            let mut line_end = chunk.len();

            for (i, &byte) in chunk.iter().enumerate().rev() {
                if byte == b'\n' || i == 0 {
                    let line_start = if byte == b'\n' { i + 1 } else { i };
                    if line_start < line_end {
                        let line_bytes = chunk[line_start..line_end].to_vec();
                        let byte_offset = chunk_start + line_start as u64;
                        lines_in_chunk.push((byte_offset, line_bytes));
                    }
                    line_end = i;
                }
            }

            all_lines.extend(lines_in_chunk);
            search_start = chunk_start;
        }

        if all_lines.len() > max_lines {
            all_lines.truncate(max_lines);
        }

        let new_start_offset = all_lines.last().map(|(off, _)| *off).unwrap_or(0);

        // Count lines before new_start_offset
        let file = File::open(&self.path)?;
        let mut count_reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut lines_before = 0;
        let mut current_offset: u64 = 0;
        let mut line_buffer = Vec::new();

        while current_offset < new_start_offset {
            line_buffer.clear();
            let bytes_read = count_reader.read_until(b'\n', &mut line_buffer)?;
            if bytes_read == 0 {
                break;
            }
            lines_before += 1;
            current_offset += bytes_read as u64;
        }

        // Reverse to get chronological order
        all_lines.reverse();
        let entries: Vec<LogEntry> = all_lines
            .into_iter()
            .enumerate()
            .map(|(i, (byte_offset, line_bytes))| {
                let line_number = lines_before + i + 1;
                let content = self.decode_line(&line_bytes);
                let content = if content.len() > self.config.max_line_length {
                    format!(
                        "{}... [truncated, {} bytes total]",
                        &content[..self.config.max_line_length],
                        content.len()
                    )
                } else {
                    content
                };
                LogEntry::new(line_number, content, byte_offset)
            })
            .collect();

        Ok((entries, new_start_offset))
    }
}

/// Async log reader for background reading
#[allow(dead_code)]
pub struct AsyncLogReader {
    reader: LogReader,
}

#[allow(dead_code)]
impl AsyncLogReader {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            reader: LogReader::new(path)?,
        })
    }

    /// Read new lines asynchronously
    pub async fn read_new_lines(&mut self) -> Result<Vec<LogEntry>> {
        // Run in blocking task for file I/O
        let path = self.reader.path.clone();
        let offset = self.reader.offset;
        let line_count = self.reader.line_count;
        let encoding = self.reader.encoding;
        let config = self.reader.config.clone();

        let (entries, new_offset, new_line_count) = tokio::task::spawn_blocking(move || {
            let mut reader = LogReader {
                path,
                offset,
                line_count,
                config,
                encoding,
                last_file_size: 0,
            };
            let entries = reader.read_new_lines()?;
            Ok::<_, anyhow::Error>((entries, reader.offset, reader.line_count))
        })
        .await??;

        self.reader.offset = new_offset;
        self.reader.line_count = new_line_count;

        Ok(entries)
    }

    /// Check if there's new content
    pub async fn has_new_content(&self) -> Result<bool> {
        let path = self.reader.path.clone();
        let offset = self.reader.offset;

        tokio::task::spawn_blocking(move || {
            let metadata = std::fs::metadata(&path)?;
            Ok::<_, anyhow::Error>(metadata.len() > offset || metadata.len() < offset)
        })
        .await?
    }

    /// Get the underlying reader (for sync operations)
    pub fn inner(&self) -> &LogReader {
        &self.reader
    }

    /// Get mutable reference to underlying reader
    pub fn inner_mut(&mut self) -> &mut LogReader {
        &mut self.reader
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_incremental_read() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        file.flush().unwrap();

        let mut reader = LogReader::new(file.path()).unwrap();
        let entries = reader.read_new_lines().unwrap();
        assert_eq!(entries.len(), 1);

        // Add more lines
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();
        file.flush().unwrap();

        let entries = reader.read_new_lines().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].content, "Line 2");
        assert_eq!(entries[0].line_number, 2);
    }
}
