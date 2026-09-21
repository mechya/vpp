//! Scoped component CSS: every element of a component's template gets a scope
//! class, and every selector of its `component.css` requires that class, so
//! the component's rules cannot reach the rest of the page.
//!
//! `component.json` with `{ "style": { "scoped": false } }` opts out.

use std::path::Path;

use vpp_format::ResourceHash;
use vpp_style::StyleSheet;

/// The scope class for a component, from its project path: `vpp-s` and the
/// first six hex digits of the path's SHA-256, the same as the C++ compiler.
pub(crate) fn scope_class(component_path: &str) -> String {
    let hash = ResourceHash::of(component_path.as_bytes()).to_string();
    format!("vpp-s{}", &hash[..6])
}

/// Makes every selector in `sheet` require `class` on its rightmost element.
pub(crate) fn scope_stylesheet(sheet: &mut StyleSheet, class: &str) {
    for rule in &mut sheet.rules {
        for selector in &mut rule.selectors {
            let last = selector
                .compounds
                .last_mut()
                .expect("a selector has at least one compound");
            last.classes.push(class.to_owned());
        }
    }
}

/// Whether the component in `dir` wants its CSS scoped: yes, unless its
/// `component.json` says `"style": { "scoped": false }`.
pub(crate) fn wants_scoping(dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("component.json")) else {
        return true;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return true;
    };
    json["style"]["scoped"].as_bool() != Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;

    #[test]
    fn scope_class_is_stable() {
        let class = scope_class("/components/stat-card");
        assert!(class.starts_with("vpp-s"));
        assert_eq!(class.len(), 11);
        assert_eq!(class, scope_class("/components/stat-card"));
        assert_ne!(class, scope_class("/components/other"));
    }

    #[test]
    fn every_selector_requires_the_class_and_gains_its_specificity() {
        let mut sheet = vpp_style::parse_stylesheet(".stat, .card > p { color: red }");
        scope_stylesheet(&mut sheet, "vpp-s123456");
        let selectors = &sheet.rules[0].selectors;
        assert_eq!(selectors[0].compounds[0].classes, ["stat", "vpp-s123456"]);
        assert_eq!(selectors[0].specificity(), 200);
        assert_eq!(selectors[1].compounds[0].classes, ["card"]);
        assert_eq!(selectors[1].compounds[1].classes, ["vpp-s123456"]);
    }

    #[test]
    fn component_json_can_opt_out() {
        let dir = TestDir::new("scoped");
        assert!(wants_scoping(dir.path()));
        dir.write("component.json", br#"{ "style": { "scoped": true } }"#);
        assert!(wants_scoping(dir.path()));
        dir.write("component.json", br#"{ "style": { "scoped": false } }"#);
        assert!(!wants_scoping(dir.path()));
    }
}
