//! `dom.bin`: a finished page's tree, as the compiler writes it and the viewer
//! loads it without an HTML parser (`docs/formats/dom.md`).
//!
//! `<script>`, `<style>`, and `<link>` elements are dropped, because their
//! content travels as separate `code/` and `style/` resources, and each run of
//! whitespace in text becomes one space.

use thiserror::Error;
use vpp_format::{ByteReader, ByteWriter, DOM_MAGIC, DOM_VERSION, DecodeError};

use crate::document::Document;
use crate::node::{Element, NodeData, NodeId};

const ELEMENT: u8 = 1;
const TEXT: u8 = 2;

/// The deepest nesting a decoder accepts, so a hostile file cannot exhaust the stack.
pub const MAX_DEPTH: usize = 512;

/// Why a `dom.bin` could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomBinError {
    /// The bytes do not start with `VPPD`.
    #[error("not a dom.bin")]
    NotDomBin,
    /// The file uses a format version this build cannot read.
    #[error("unsupported dom.bin version {found}; this build reads version {supported}")]
    UnsupportedVersion {
        /// The version in the file.
        found: u16,
        /// The version this build reads.
        supported: u16,
    },
    /// The bytes do not decode.
    #[error("corrupt dom.bin: {0}")]
    Corrupt(#[from] DecodeError),
    /// A node kind byte is neither element nor text.
    #[error("corrupt dom.bin: unknown node kind {kind} at byte {offset}")]
    UnknownNodeKind {
        /// The byte found.
        kind: u8,
        /// Where it was.
        offset: usize,
    },
    /// Elements are nested more deeply than [`MAX_DEPTH`].
    #[error("corrupt dom.bin: nested more than {MAX_DEPTH} levels deep")]
    TooDeep,
    /// There are bytes after the tree.
    #[error("corrupt dom.bin: data after the tree")]
    TrailingData,
}

/// Encodes the whole document as `dom.bin`.
pub fn encode_dom(document: &Document) -> Vec<u8> {
    let mut w = ByteWriter::new();
    w.magic(&DOM_MAGIC);
    w.u16(DOM_VERSION);
    write_children(&mut w, document, document.root());
    w.into_bytes()
}

/// Decodes a `dom.bin` into a new document.
pub fn decode_dom(bytes: &[u8]) -> Result<Document, DomBinError> {
    let mut r = ByteReader::new(bytes);
    r.magic(&DOM_MAGIC).map_err(|_| DomBinError::NotDomBin)?;
    let version = r.u16()?;
    if version != DOM_VERSION {
        return Err(DomBinError::UnsupportedVersion {
            found: version,
            supported: DOM_VERSION,
        });
    }
    let mut document = Document::new();
    let root = document.root();
    read_children(&mut r, &mut document, root, 0)?;
    if !r.is_at_end() {
        return Err(DomBinError::TrailingData);
    }
    Ok(document)
}

fn is_dropped(document: &Document, id: NodeId) -> bool {
    let node = &document[id];
    ["script", "style", "link"]
        .iter()
        .any(|tag| node.is_element(tag))
}

fn kept_children(document: &Document, parent: NodeId) -> Vec<NodeId> {
    document[parent]
        .children()
        .iter()
        .copied()
        .filter(|&c| {
            matches!(document[c].data(), NodeData::Text(_) | NodeData::Element(_))
                && !is_dropped(document, c)
        })
        .collect()
}

// Recursion depth follows the document's depth, which the HTML parser and
// template expander produce from trusted build-time sources.
fn write_children(w: &mut ByteWriter, document: &Document, parent: NodeId) {
    let children = kept_children(document, parent);
    w.u32(children.len() as u32);
    for child in children {
        match document[child].data() {
            NodeData::Text(text) => {
                w.u8(TEXT);
                w.str(&collapse_whitespace(text));
            }
            NodeData::Element(element) => {
                w.u8(ELEMENT);
                w.str(&element.tag);
                w.u32(element.attributes.len() as u32);
                for attribute in &element.attributes {
                    w.str(&attribute.name);
                    w.str(&attribute.value);
                }
                write_children(w, document, child);
            }
            NodeData::Document => unreachable!("filtered by kept_children"),
        }
    }
}

fn read_children(
    r: &mut ByteReader<'_>,
    document: &mut Document,
    parent: NodeId,
    depth: usize,
) -> Result<(), DomBinError> {
    if depth > MAX_DEPTH {
        return Err(DomBinError::TooDeep);
    }
    let count = r.count()?;
    for _ in 0..count {
        let offset = r.position();
        match r.u8()? {
            TEXT => {
                let text = document.create_text(r.str()?);
                document.append_child(parent, text);
            }
            ELEMENT => {
                let mut element = Element::new(r.str()?);
                let attributes = r.count()?;
                for _ in 0..attributes {
                    let name = r.str()?;
                    let value = r.str()?;
                    element.set_attribute(name, value);
                }
                let node = document.create(NodeData::Element(element));
                document.append_child(parent, node);
                read_children(r, document, node, depth + 1)?;
            }
            kind => return Err(DomBinError::UnknownNodeKind { kind, offset }),
        }
    }
    Ok(())
}

/// Each run of whitespace becomes one space. Spaces at the start and end are
/// kept: they separate this text from the inline content next to it.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        if c.is_ascii_whitespace() {
            space = true;
        } else {
            if space {
                out.push(' ');
            }
            space = false;
            out.push(c);
        }
    }
    if space {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_html;

    #[test]
    fn round_trips_elements_attributes_and_text() {
        let doc = parse_html(r#"<main id="m" class="a b"><p>Hello <b>world</b></p></main>"#);
        let decoded = decode_dom(&encode_dom(&doc)).unwrap();
        let main = decoded.find_by_id(decoded.root(), "m").unwrap();
        assert_eq!(
            decoded.element(main).unwrap().attribute("class"),
            Some("a b")
        );
        assert_eq!(decoded.text_content(main), "Hello world");
        assert_eq!(encode_dom(&decoded), encode_dom(&doc));
    }

    #[test]
    fn layout_matches_the_specification() {
        let mut doc = Document::new();
        let p = doc.create_element("p");
        doc.element_mut(p).unwrap().set_attribute("id", "x");
        let text = doc.create_text("a  \n b");
        doc.append_child(doc.root(), p);
        doc.append_child(p, text);

        let mut expected = b"VPPD".to_vec();
        expected.extend_from_slice(&1u16.to_le_bytes());
        expected.extend_from_slice(&1u32.to_le_bytes()); // root: one child
        expected.push(1); // element
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"p");
        expected.extend_from_slice(&1u32.to_le_bytes()); // one attribute
        expected.extend_from_slice(&2u32.to_le_bytes());
        expected.extend_from_slice(b"id");
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(b"x");
        expected.extend_from_slice(&1u32.to_le_bytes()); // p: one child
        expected.push(2); // text
        expected.extend_from_slice(&3u32.to_le_bytes());
        expected.extend_from_slice(b"a b");
        assert_eq!(encode_dom(&doc), expected);
    }

    #[test]
    fn drops_script_style_and_link_and_collapses_whitespace() {
        let doc = parse_html(
            "<head><link rel=stylesheet href=a.css><style>p{}</style></head>\
             <body><script>x()</script><p>  one\n\ttwo  </p></body>",
        );
        let decoded = decode_dom(&encode_dom(&doc)).unwrap();
        for tag in ["script", "style", "link"] {
            assert!(decoded.find_first(decoded.root(), tag).is_none(), "{tag}");
        }
        let p = decoded.find_first(decoded.root(), "p").unwrap();
        assert_eq!(
            decoded[decoded[p].children()[0]].as_text(),
            Some(" one two ")
        );
    }

    #[test]
    fn refuses_other_files_and_versions() {
        assert_eq!(
            decode_dom(b"VPPK\x03\x00").unwrap_err(),
            DomBinError::NotDomBin
        );
        assert_eq!(
            decode_dom(b"VPPD\x02\x00").unwrap_err(),
            DomBinError::UnsupportedVersion {
                found: 2,
                supported: 1
            }
        );
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let bytes = encode_dom(&parse_html("<p class=a>one<b>two</b></p>"));
        for len in 0..bytes.len() {
            assert!(decode_dom(&bytes[..len]).is_err(), "length {len}");
        }
    }

    #[test]
    fn refuses_unknown_node_kinds_and_trailing_bytes() {
        let mut bytes = b"VPPD\x01\x00".to_vec();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(9);
        assert_eq!(
            decode_dom(&bytes).unwrap_err(),
            DomBinError::UnknownNodeKind {
                kind: 9,
                offset: 10
            }
        );

        let mut bytes = encode_dom(&Document::new());
        bytes.push(0);
        assert_eq!(decode_dom(&bytes).unwrap_err(), DomBinError::TrailingData);
    }

    #[test]
    fn refuses_nesting_deeper_than_the_limit() {
        // INTENTIONAL: hostile input. Elements nested past MAX_DEPTH must be refused, not recursed into.
        let mut bytes = b"VPPD\x01\x00".to_vec();
        for _ in 0..=MAX_DEPTH + 1 {
            bytes.extend_from_slice(&1u32.to_le_bytes()); // one child
            bytes.push(1); // element
            bytes.extend_from_slice(&3u32.to_le_bytes());
            bytes.extend_from_slice(b"div");
            bytes.extend_from_slice(&0u32.to_le_bytes()); // no attributes
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode_dom(&bytes).unwrap_err(), DomBinError::TooDeep);
    }
}
