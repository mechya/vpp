//! `style.bin`: one parsed stylesheet, as the compiler writes it and the viewer
//! loads it without a CSS parser (`docs/formats/style.md`). Selector matching
//! still happens at run time, because scripts can change the DOM.

use thiserror::Error;
use vpp_format::{ByteReader, ByteWriter, DecodeError, STYLE_MAGIC, STYLE_VERSION};

use crate::selector::{Combinator, CompoundSelector, Selector};
use crate::stylesheet::{Declaration, Rule, StyleSheet};

const DESCENDANT: u8 = b' ';
const CHILD: u8 = b'>';

/// Why a `style.bin` could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StyleBinError {
    /// The bytes do not start with `VPPS`.
    #[error("not a style.bin")]
    NotStyleBin,
    /// The file uses a format version this build cannot read.
    #[error("unsupported style.bin version {found}; this build reads version {supported}")]
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
        /// The version this build reads.
        supported: u16,
    },
    /// The bytes do not decode.
    #[error("corrupt style.bin: {0}")]
    Corrupt(#[from] DecodeError),
    /// A selector's parts do not fit together, or its specificity does not match them.
    #[error("corrupt style.bin: malformed selector at byte {offset}")]
    BadSelector {
        /// Where the selector starts.
        offset: usize,
    },
    /// There are bytes after the last rule.
    #[error("corrupt style.bin: data after the last rule")]
    TrailingData,
}

/// Encodes a stylesheet as `style.bin`.
pub fn encode_stylesheet(sheet: &StyleSheet) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.magic(&STYLE_MAGIC);
    w.u16(STYLE_VERSION);
    w.u32(sheet.rules.len() as u32);
    for rule in &sheet.rules {
        w.u32(rule.selectors.len() as u32);
        for selector in &rule.selectors {
            write_selector(&mut w, selector);
        }
        w.u32(rule.declarations.len() as u32);
        for declaration in &rule.declarations {
            w.str(&declaration.property);
            w.str(&declaration.value);
            w.u8(u8::from(declaration.important));
        }
    }
    w.into_bytes()
}

/// Decodes a `style.bin`.
pub fn decode_stylesheet(bytes: &[u8]) -> Result<StyleSheet, StyleBinError> {
    let mut r = ByteReader::new(bytes);
    r.magic(&STYLE_MAGIC)
        .map_err(|_| StyleBinError::NotStyleBin)?;
    let version = r.u16()?;
    if version != STYLE_VERSION {
        return Err(StyleBinError::UnsupportedVersion {
            found: version,
            supported: STYLE_VERSION,
        });
    }

    let mut sheet = StyleSheet::default();
    for _ in 0..r.count()? {
        let mut selectors = Vec::new();
        for _ in 0..r.count()? {
            selectors.push(read_selector(&mut r)?);
        }
        let mut declarations = Vec::new();
        for _ in 0..r.count()? {
            declarations.push(Declaration {
                property: r.str()?.to_owned(),
                value: r.str()?.to_owned(),
                important: r.u8()? != 0,
            });
        }
        sheet.rules.push(Rule {
            selectors,
            declarations,
        });
    }
    if !r.is_at_end() {
        return Err(StyleBinError::TrailingData);
    }
    Ok(sheet)
}

/// An absent tag or id is stored as an empty string, as in the C++ format.
fn write_selector(w: &mut ByteWriter, selector: &Selector) {
    w.u32(selector.specificity() as u32);
    w.u32(selector.compounds.len() as u32);
    for compound in &selector.compounds {
        w.str(compound.tag.as_deref().unwrap_or_default());
        w.str(compound.id.as_deref().unwrap_or_default());
        w.u32(compound.classes.len() as u32);
        for class in &compound.classes {
            w.str(class);
        }
    }
    w.u32(selector.combinators.len() as u32);
    for combinator in &selector.combinators {
        w.u8(match combinator {
            Combinator::Descendant => DESCENDANT,
            Combinator::Child => CHILD,
        });
    }
}

fn read_selector(r: &mut ByteReader<'_>) -> Result<Selector, StyleBinError> {
    let offset = r.position();
    let bad = StyleBinError::BadSelector { offset };
    // Stored as a signed 32-bit number; the bits round-trip through u32.
    let specificity = r.u32()? as i32;

    let mut compounds = Vec::new();
    for _ in 0..r.count()? {
        let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        let tag = non_empty(r.str()?);
        let id = non_empty(r.str()?);
        let mut classes = Vec::new();
        for _ in 0..r.count()? {
            classes.push(r.str()?.to_owned());
        }
        compounds.push(CompoundSelector { tag, id, classes });
    }
    let mut combinators = Vec::new();
    for _ in 0..r.count()? {
        combinators.push(match r.u8()? {
            DESCENDANT => Combinator::Descendant,
            CHILD => Combinator::Child,
            _ => return Err(bad),
        });
    }

    let selector = Selector {
        compounds,
        combinators,
    };
    let fits = !selector.compounds.is_empty()
        && selector.combinators.len() + 1 == selector.compounds.len();
    if !fits || selector.specificity() != specificity {
        return Err(bad);
    }
    Ok(selector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_stylesheet;

    const CSS: &str =
        "body { margin: 0 } .card > p, #x b.big { color: red !important; padding: 1px 2px }";

    #[test]
    fn round_trips() {
        let sheet = parse_stylesheet(CSS);
        assert_eq!(
            decode_stylesheet(&encode_stylesheet(&sheet)).unwrap(),
            sheet
        );
    }

    #[test]
    fn layout_matches_the_specification() {
        let sheet = parse_stylesheet("p.a{x:1}");
        let mut expected = b"VPPS".to_vec();
        expected.extend_from_slice(&1u16.to_le_bytes());
        expected.extend_from_slice(&1u32.to_le_bytes()); // one rule
        expected.extend_from_slice(&1u32.to_le_bytes()); // one selector
        expected.extend_from_slice(&101i32.to_le_bytes()); // specificity
        expected.extend_from_slice(&1u32.to_le_bytes()); // one compound
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"p");
        expected.extend_from_slice(&0u32.to_le_bytes()); // no id
        expected.extend_from_slice(&1u32.to_le_bytes()); // one class
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"a");
        expected.extend_from_slice(&0u32.to_le_bytes()); // no combinators
        expected.extend_from_slice(&1u32.to_le_bytes()); // one declaration
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"x");
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"1");
        expected.push(0); // not important
        assert_eq!(encode_stylesheet(&sheet), expected);
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let bytes = encode_stylesheet(&parse_stylesheet(CSS));
        for len in 0..bytes.len() {
            assert!(decode_stylesheet(&bytes[..len]).is_err(), "length {len}");
        }
    }

    #[test]
    fn refuses_other_files_versions_and_trailing_bytes() {
        assert_eq!(
            decode_stylesheet(b"VPPD\x01\x00").unwrap_err(),
            StyleBinError::NotStyleBin
        );
        assert!(matches!(
            decode_stylesheet(b"VPPS\x09\x00").unwrap_err(),
            StyleBinError::UnsupportedVersion { found: 9, .. }
        ));
        let mut bytes = encode_stylesheet(&StyleSheet::default());
        bytes.push(1);
        assert_eq!(
            decode_stylesheet(&bytes).unwrap_err(),
            StyleBinError::TrailingData
        );
    }

    #[test]
    fn refuses_selectors_that_do_not_fit_together() {
        let mut w = ByteWriter::new();
        w.magic(&STYLE_MAGIC);
        w.u16(STYLE_VERSION);
        w.u32(1); // one rule
        w.u32(1); // one selector
        w.u32(0); // specificity
        w.u32(1); // one compound: *
        w.str("");
        w.str("");
        w.u32(0);
        w.u32(1); // one combinator, but only one compound
        w.u8(b'>');
        w.u32(0);
        assert!(matches!(
            decode_stylesheet(w.as_bytes()).unwrap_err(),
            StyleBinError::BadSelector { .. }
        ));
    }

    #[test]
    fn refuses_a_specificity_that_does_not_match_the_selector() {
        let mut bytes = encode_stylesheet(&parse_stylesheet("p{x:1}"));
        // The specificity follows magic, version, and two counts.
        bytes[14] = 99;
        assert!(matches!(
            decode_stylesheet(&bytes).unwrap_err(),
            StyleBinError::BadSelector { offset: 14 }
        ));
    }
}
