//! Little-endian byte reading and writing, shared by every VPP binary format.
//!
//! Readers never trust the data. Every read checks how many bytes remain, and
//! strings and counts have upper limits, so a truncated or hostile file gives a
//! [`DecodeError`] instead of a panic or a huge allocation
//! (`ARCHITECTURE.md`, invariant 7).

use thiserror::Error;

/// The longest length-prefixed string or blob a reader accepts: 16 MiB.
pub const MAX_STRING: usize = 16 * 1024 * 1024;

/// The largest item count a reader accepts: 2^20.
pub const MAX_COUNT: u32 = 1 << 20;

/// Why a read failed. Offsets are byte positions from the start of the data.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The data ended before the value did.
    #[error("unexpected end of data at byte {offset}: {needed} more bytes needed")]
    Truncated {
        /// Where the value starts.
        offset: usize,
        /// How many bytes the value needed.
        needed: usize,
    },
    /// The file does not start with the expected magic bytes.
    #[error("wrong file type: expected \"{}\"", String::from_utf8_lossy(expected))]
    WrongMagic {
        /// The magic bytes that were expected.
        expected: [u8; 4],
    },
    /// A length prefix is larger than [`MAX_STRING`].
    #[error("string at byte {offset} claims {len} bytes, more than the {MAX_STRING}-byte limit")]
    StringTooLarge {
        /// Where the length prefix starts.
        offset: usize,
        /// The length it claims.
        len: u32,
    },
    /// A count is larger than [`MAX_COUNT`].
    #[error("count at byte {offset} is {count}, more than the limit of {MAX_COUNT}")]
    CountTooLarge {
        /// Where the count starts.
        offset: usize,
        /// The count it claims.
        count: u32,
    },
    /// A text field is not valid UTF-8.
    #[error("text at byte {offset} is not valid UTF-8")]
    InvalidUtf8 {
        /// Where the text's length prefix starts.
        offset: usize,
    },
}

/// Builds a byte buffer in VPP's little-endian encoding.
#[derive(Debug, Default)]
pub struct ByteWriter {
    buf: Vec<u8>,
}

impl ByteWriter {
    /// An empty writer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes four magic bytes.
    pub fn magic(&mut self, magic: &[u8; 4]) {
        self.buf.extend_from_slice(magic);
    }

    /// Writes one byte.
    pub fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    /// Writes a 2-byte unsigned integer.
    pub fn u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a 4-byte unsigned integer.
    pub fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes an 8-byte unsigned integer.
    pub fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a length-prefixed blob: a `u32` length, then the bytes.
    ///
    /// # Panics
    ///
    /// If `bytes` is longer than [`MAX_STRING`], which no reader would accept.
    pub fn blob(&mut self, bytes: &[u8]) {
        assert!(
            bytes.len() <= MAX_STRING,
            "blob of {} bytes exceeds the {MAX_STRING}-byte limit readers accept",
            bytes.len()
        );
        // Cannot truncate: MAX_STRING fits in a u32.
        self.u32(bytes.len() as u32);
        self.buf.extend_from_slice(bytes);
    }

    /// Writes a length-prefixed UTF-8 string.
    ///
    /// # Panics
    ///
    /// If `text` is longer than [`MAX_STRING`] bytes.
    pub fn str(&mut self, text: &str) {
        self.blob(text.as_bytes());
    }

    /// Writes bytes as they are, with no length prefix.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// The number of bytes written so far.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether nothing has been written yet.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// The bytes written so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Finishes writing and returns the bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

/// Reads VPP's little-endian encoding from a byte slice, checking every read.
#[derive(Debug, Clone)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    /// A reader positioned at the start of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// The current byte position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Whether every byte has been read.
    pub fn is_at_end(&self) -> bool {
        self.pos == self.data.len()
    }

    /// Checks for four magic bytes.
    pub fn magic(&mut self, expected: &[u8; 4]) -> Result<(), DecodeError> {
        if self.take(4)? == expected {
            Ok(())
        } else {
            Err(DecodeError::WrongMagic {
                expected: *expected,
            })
        }
    }

    /// Reads one byte.
    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.array::<1>()?[0])
    }

    /// Reads a 2-byte unsigned integer.
    pub fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    /// Reads a 4-byte unsigned integer.
    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// Reads an 8-byte unsigned integer.
    pub fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// Reads a `u32` item count, refusing counts above [`MAX_COUNT`].
    pub fn count(&mut self) -> Result<u32, DecodeError> {
        let offset = self.pos;
        let count = self.u32()?;
        if count > MAX_COUNT {
            return Err(DecodeError::CountTooLarge { offset, count });
        }
        Ok(count)
    }

    /// Reads a length-prefixed blob, refusing lengths above [`MAX_STRING`].
    pub fn blob(&mut self) -> Result<&'a [u8], DecodeError> {
        let offset = self.pos;
        let len = self.u32()?;
        if len as usize > MAX_STRING {
            return Err(DecodeError::StringTooLarge { offset, len });
        }
        self.take(len as usize)
    }

    /// Reads a length-prefixed UTF-8 string.
    pub fn str(&mut self) -> Result<&'a str, DecodeError> {
        let offset = self.pos;
        let bytes = self.blob()?;
        std::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 { offset })
    }

    /// Reads exactly `N` bytes.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let bytes = self.take(N)?;
        let mut out = [0; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    /// Takes the next `n` bytes, or fails without moving if fewer remain.
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let remaining = self.data.len() - self.pos;
        if n > remaining {
            return Err(DecodeError::Truncated {
                offset: self.pos,
                needed: n,
            });
        }
        let bytes = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_round_trips() {
        let mut w = ByteWriter::new();
        w.magic(b"TEST");
        w.u8(0xab);
        w.u16(0x1234);
        w.u32(0xdead_beef);
        w.u64(0x0102_0304_0506_0708);
        w.str("héllo");
        w.blob(&[1, 2, 3]);
        w.blob(&[]);
        w.raw(&[9, 9]);
        let bytes = w.into_bytes();

        let mut r = ByteReader::new(&bytes);
        r.magic(b"TEST").unwrap();
        assert_eq!(r.u8().unwrap(), 0xab);
        assert_eq!(r.u16().unwrap(), 0x1234);
        assert_eq!(r.u32().unwrap(), 0xdead_beef);
        assert_eq!(r.u64().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(r.str().unwrap(), "héllo");
        assert_eq!(r.blob().unwrap(), &[1, 2, 3]);
        assert_eq!(r.blob().unwrap(), &[] as &[u8]);
        assert_eq!(r.array::<2>().unwrap(), [9, 9]);
        assert!(r.is_at_end());
    }

    #[test]
    fn integers_are_little_endian() {
        let mut w = ByteWriter::new();
        w.u16(0x0102);
        w.u32(0x0304_0506);
        assert_eq!(w.as_bytes(), &[0x02, 0x01, 0x06, 0x05, 0x04, 0x03]);
    }

    #[test]
    fn truncated_read_reports_where_and_does_not_move() {
        let mut r = ByteReader::new(&[1, 2, 3]);
        r.u8().unwrap();
        assert_eq!(
            r.u32(),
            Err(DecodeError::Truncated {
                offset: 1,
                needed: 4
            })
        );
        assert_eq!(r.position(), 1);
    }

    #[test]
    fn oversized_string_is_refused_before_reading_it() {
        // INTENTIONAL: hostile input. The length claims 4 GiB; the reader must refuse, not allocate.
        let bytes = u32::MAX.to_le_bytes();
        assert_eq!(
            ByteReader::new(&bytes).str(),
            Err(DecodeError::StringTooLarge {
                offset: 0,
                len: u32::MAX
            })
        );
    }

    #[test]
    fn string_longer_than_the_data_is_truncated() {
        let mut bytes = 10u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(b"short");
        assert_eq!(
            ByteReader::new(&bytes).str(),
            Err(DecodeError::Truncated {
                offset: 4,
                needed: 10
            })
        );
    }

    #[test]
    fn count_above_the_limit_is_refused() {
        let bytes = (MAX_COUNT + 1).to_le_bytes();
        assert_eq!(
            ByteReader::new(&bytes).count(),
            Err(DecodeError::CountTooLarge {
                offset: 0,
                count: MAX_COUNT + 1
            })
        );
        assert_eq!(
            ByteReader::new(&MAX_COUNT.to_le_bytes()).count(),
            Ok(MAX_COUNT)
        );
    }

    #[test]
    fn wrong_magic_is_refused() {
        assert_eq!(
            ByteReader::new(b"NOPE").magic(b"VPPK"),
            Err(DecodeError::WrongMagic { expected: *b"VPPK" })
        );
    }

    #[test]
    fn invalid_utf8_is_refused() {
        let mut w = ByteWriter::new();
        w.blob(&[0xff, 0xfe]);
        assert_eq!(
            ByteReader::new(w.as_bytes()).str(),
            Err(DecodeError::InvalidUtf8 { offset: 0 })
        );
    }

    #[test]
    #[should_panic(expected = "exceeds")]
    fn writer_refuses_a_blob_no_reader_would_accept() {
        ByteWriter::new().blob(&vec![0; MAX_STRING + 1]);
    }
}
