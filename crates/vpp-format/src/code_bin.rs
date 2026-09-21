//! `code.bin`: one script of a page, as JavaScript source (`docs/formats/code.md`).
//!
//! Scripts travel as source, not bytecode: the viewer compiles them itself,
//! because loading bytecode made elsewhere would let a hostile publisher
//! attack the engine (`docs/design/0001-code-bin.md`).

use thiserror::Error;

use crate::bytes::{ByteReader, ByteWriter, DecodeError};
use crate::version::{CODE_MAGIC, CODE_VERSION};

/// What kind of JavaScript a `code.bin` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    /// A classic script, run in the page's global scope.
    Classic,
}

impl ScriptKind {
    fn byte(self) -> u8 {
        match self {
            ScriptKind::Classic => 1,
        }
    }

    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(ScriptKind::Classic),
            // 2 is reserved for ES modules.
            _ => None,
        }
    }
}

/// One script: its kind, its file name for messages, and its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBin {
    /// What kind of script it is.
    pub kind: ScriptKind,
    /// The source file name, such as `app.js`, for error messages and stack traces.
    pub filename: String,
    /// The JavaScript source.
    pub source: String,
}

/// Why a `code.bin` could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodeBinError {
    /// The bytes do not start with `VPPC`: for example raw bytecode from the C++ compiler.
    #[error("not a code.bin; recompile the page with this version of vppc")]
    NotCodeBin,
    /// The file uses a format version this build cannot read.
    #[error("unsupported code.bin version {found}; this build reads version {supported}")]
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
        /// The version this build reads.
        supported: u16,
    },
    /// The kind byte is not one this build knows.
    #[error("unsupported script kind {0}")]
    UnsupportedKind(u8),
    /// The bytes do not decode.
    #[error("corrupt code.bin: {0}")]
    Corrupt(#[from] DecodeError),
    /// There are bytes after the source.
    #[error("corrupt code.bin: data after the source")]
    TrailingData,
}

impl CodeBin {
    /// A classic script.
    pub fn classic(filename: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            kind: ScriptKind::Classic,
            filename: filename.into(),
            source: source.into(),
        }
    }

    /// Encodes the script as `code.bin`.
    ///
    /// # Panics
    ///
    /// If the source or file name is longer than [`crate::MAX_STRING`] bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = ByteWriter::new();
        w.magic(&CODE_MAGIC);
        w.u16(CODE_VERSION);
        w.u8(self.kind.byte());
        w.str(&self.filename);
        w.str(&self.source);
        w.into_bytes()
    }

    /// Decodes a `code.bin`.
    pub fn decode(bytes: &[u8]) -> Result<Self, CodeBinError> {
        let mut r = ByteReader::new(bytes);
        r.magic(&CODE_MAGIC).map_err(|_| CodeBinError::NotCodeBin)?;
        let version = r.u16()?;
        if version != CODE_VERSION {
            return Err(CodeBinError::UnsupportedVersion {
                found: version,
                supported: CODE_VERSION,
            });
        }
        let kind_byte = r.u8()?;
        let kind =
            ScriptKind::from_byte(kind_byte).ok_or(CodeBinError::UnsupportedKind(kind_byte))?;
        let filename = r.str()?.to_owned();
        let source = r.str()?.to_owned();
        if !r.is_at_end() {
            return Err(CodeBinError::TrailingData);
        }
        Ok(Self {
            kind,
            filename,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let code = CodeBin::classic("app.js", "console.log('héllo');\n");
        assert_eq!(CodeBin::decode(&code.encode()).unwrap(), code);
    }

    #[test]
    fn layout_matches_the_specification() {
        let mut expected = b"VPPC".to_vec();
        expected.extend_from_slice(&1u16.to_le_bytes());
        expected.push(1); // classic script
        expected.extend_from_slice(&4u32.to_le_bytes());
        expected.extend_from_slice(b"a.js");
        expected.extend_from_slice(&3u32.to_le_bytes());
        expected.extend_from_slice(b"f()");
        assert_eq!(CodeBin::classic("a.js", "f()").encode(), expected);
    }

    #[test]
    fn refuses_raw_bytecode_other_versions_and_kinds() {
        // The C++ compiler wrote bare QuickJS bytecode, which starts with its own version byte.
        assert_eq!(
            CodeBin::decode(&[0x05, 0x01, 0x02]).unwrap_err(),
            CodeBinError::NotCodeBin
        );
        assert!(matches!(
            CodeBin::decode(b"VPPC\x02\x00").unwrap_err(),
            CodeBinError::UnsupportedVersion { found: 2, .. }
        ));
        assert_eq!(
            CodeBin::decode(b"VPPC\x01\x00\x02").unwrap_err(),
            CodeBinError::UnsupportedKind(2)
        );
    }

    #[test]
    fn every_truncation_and_trailing_byte_is_an_error() {
        let bytes = CodeBin::classic("a.js", "f()").encode();
        for len in 0..bytes.len() {
            assert!(CodeBin::decode(&bytes[..len]).is_err(), "length {len}");
        }
        let mut longer = bytes;
        longer.push(0);
        assert_eq!(
            CodeBin::decode(&longer).unwrap_err(),
            CodeBinError::TrailingData
        );
    }
}
