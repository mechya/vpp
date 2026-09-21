//! Properties: a component or include tag's attributes, as `{{ }}` values.

use vpp_dom::Element;

use crate::substitute::Variables;

/// The attributes of `tag` except those in `skip`. An attribute with no value
/// (`<stat-card highlight>`) is the property `"true"`.
pub(crate) fn properties(tag: &Element, skip: &[&str]) -> Variables {
    tag.attributes
        .iter()
        .filter(|a| !skip.contains(&a.name.as_str()))
        .map(|a| {
            let value = if a.value.is_empty() { "true" } else { &a.value };
            (a.name.clone(), value.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_shorthand_and_skipped_names() {
        let doc = vpp_dom::parse_html(r#"<x-card src="/c" value="3" highlight></x-card>"#);
        let card = doc.find_first(doc.root(), "x-card").unwrap();
        let props = properties(doc.element(card).unwrap(), &["src"]);
        assert_eq!(props.get("value").map(String::as_str), Some("3"));
        assert_eq!(props.get("highlight").map(String::as_str), Some("true"));
        assert!(!props.contains_key("src"));
    }
}
