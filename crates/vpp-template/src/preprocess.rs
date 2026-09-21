//! Text fixes applied before HTML parsing, for two things HTML itself cannot express:
//!
//! 1. Self-closing custom tags: `<stat-card a="1" />` becomes
//!    `<stat-card a="1"></stat-card>`. HTML ignores the `/` on unknown
//!    elements, which would make everything after them their children.
//! 2. Slots inside `<head>`: `<vpp-slot name="x"></vpp-slot>` becomes
//!    `<meta name="vpp-slot" content="x">`. An HTML parser moves unknown
//!    elements out of `<head>`, but leaves `<meta>` where it is.

/// Applies both fixes to a template's HTML source.
pub fn preprocess_template_html(html: &str) -> String {
    slots_in_head(&self_closing_custom_tags(html))
}

fn is_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':')
}

fn self_closing_custom_tags(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while i < bytes.len() {
        let starts_tag =
            bytes[i] == b'<' && bytes.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic());
        if !starts_tag {
            // Copy up to the next '<'. Step over a '<' that starts no tag; any
            // other byte here starts a character, so slicing from it is safe.
            let from = if bytes[i] == b'<' { i + 1 } else { i };
            let next = html[from..].find('<').map_or(bytes.len(), |n| from + n);
            out.push_str(&html[i..next]);
            i = next;
            continue;
        }
        let mut name_end = i + 1;
        while name_end < bytes.len() && is_name_char(bytes[name_end]) {
            name_end += 1;
        }
        let name = html[i + 1..name_end].to_ascii_lowercase();

        // The closing '>', skipping any inside quoted attribute values.
        let mut k = name_end;
        let mut quote = None;
        while k < bytes.len() {
            match (quote, bytes[k]) {
                (Some(q), c) if c == q => quote = None,
                (None, c @ (b'"' | b'\'')) => quote = Some(c),
                (None, b'>') => break,
                _ => {}
            }
            k += 1;
        }
        if k >= bytes.len() {
            out.push_str(&html[i..]);
            break;
        }
        if name.contains('-') && bytes[k - 1] == b'/' {
            out.push_str(&html[i..k - 1]);
            out.push_str("></");
            out.push_str(&name);
            out.push('>');
        } else {
            out.push_str(&html[i..=k]);
        }
        i = k + 1;
    }
    out
}

fn slots_in_head(html: &str) -> String {
    let lowered = html.to_ascii_lowercase();
    let (Some(head_start), Some(head_end)) = (lowered.find("<head"), lowered.find("</head>"))
    else {
        return html.to_owned();
    };
    if head_end < head_start {
        return html.to_owned();
    }

    let head = &html[head_start..head_end];
    let head_lower = &lowered[head_start..head_end];
    let mut rebuilt = String::with_capacity(head.len());
    let mut p = 0;
    while let Some(s) = head_lower[p..].find("<vpp-slot").map(|n| p + n) {
        let Some(gt) = head[s..].find('>').map(|n| s + n) else {
            break;
        };
        let name = slot_name(&head[s..gt]);
        rebuilt.push_str(&head[p..s]);
        rebuilt.push_str(&format!("<meta name=\"vpp-slot\" content=\"{name}\">"));
        p = gt + 1;
        if let Some(close) = head_lower[p..].find("</vpp-slot>") {
            p += close + "</vpp-slot>".len();
        }
    }
    rebuilt.push_str(&head[p..]);
    format!("{}{rebuilt}{}", &html[..head_start], &html[head_end..])
}

/// The quoted value of `name=` in a tag's text, or empty.
fn slot_name(tag: &str) -> &str {
    let Some(at) = tag.find("name=") else {
        return "";
    };
    let rest = &tag[at + "name=".len()..];
    let Some(quote) = rest.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
        return "";
    };
    rest[1..].find(quote).map_or("", |end| &rest[1..1 + end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_closes_self_closing_custom_tags() {
        assert_eq!(
            preprocess_template_html(r#"<p><stat-card value="a/b" label='x > y' /></p>"#),
            r#"<p><stat-card value="a/b" label='x > y' ></stat-card></p>"#
        );
    }

    #[test]
    fn leaves_other_tags_alone() {
        let html = r#"<br/><img src="a.png" /><div>t</div> 1 < 2 <!-- c -->"#;
        assert_eq!(preprocess_template_html(html), html);
    }

    #[test]
    fn slots_in_head_become_meta_placeholders() {
        let html = "<head>\n  <vpp-slot name=\"head\" />\n  <title>t</title>\n</head><body><vpp-slot /></body>";
        assert_eq!(
            preprocess_template_html(html),
            "<head>\n  <meta name=\"vpp-slot\" content=\"head\">\n  <title>t</title>\n</head><body><vpp-slot ></vpp-slot></body>"
        );
    }

    #[test]
    fn keeps_non_ascii_text_intact() {
        let html = "<p>日本語 — ✓</p><x-y a=\"é\"/>";
        assert_eq!(
            preprocess_template_html(html),
            "<p>日本語 — ✓</p><x-y a=\"é\"></x-y>"
        );
    }

    #[test]
    fn unterminated_tag_is_copied_as_is() {
        assert_eq!(preprocess_template_html("<x-y a=\"1"), "<x-y a=\"1");
    }
}
