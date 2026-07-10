//! Length-prefixed stdio framing: `u32` little-endian payload length plus
//! payload bytes. Payload size is capped before allocation.

use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    Oversized { length: usize, max: usize },
    Eof,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "frame I/O failed: {error}"),
            Self::Oversized { length, max } => {
                write!(
                    formatter,
                    "frame of {length} bytes exceeds cap of {max} bytes"
                )
            }
            Self::Eof => write!(formatter, "protocol stream ended"),
        }
    }
}

pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

pub fn read_frame(reader: &mut impl Read, max_len: usize) -> Result<Vec<u8>, FrameError> {
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Err(FrameError::Eof),
        Err(error) => return Err(FrameError::Io(error)),
    }
    let length = u32::from_le_bytes(header) as usize;
    if length > max_len {
        return Err(FrameError::Oversized {
            length,
            max: max_len,
        });
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            FrameError::Eof
        } else {
            FrameError::Io(error)
        }
    })?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrips_frames() {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, b"hello").expect("writes");
        write_frame(&mut buffer, b"").expect("writes");
        let mut cursor = Cursor::new(buffer);
        assert_eq!(read_frame(&mut cursor, 16).expect("reads"), b"hello");
        assert_eq!(read_frame(&mut cursor, 16).expect("reads"), b"");
        assert!(matches!(read_frame(&mut cursor, 16), Err(FrameError::Eof)));
    }

    #[test]
    fn caps_oversized_frames_before_allocation() {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &[0u8; 32]).expect("writes");
        let mut cursor = Cursor::new(buffer);
        assert!(matches!(
            read_frame(&mut cursor, 16),
            Err(FrameError::Oversized {
                length: 32,
                max: 16
            })
        ));
    }

    #[test]
    fn truncated_payload_reports_eof() {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, b"hello").expect("writes");
        buffer.truncate(6);
        let mut cursor = Cursor::new(buffer);
        assert!(matches!(read_frame(&mut cursor, 16), Err(FrameError::Eof)));
    }
}
