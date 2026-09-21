//! `vpp.json`, the site configuration at the root of every site.
//!
//! ```json
//! {
//!   "id": "org.vpp.examples.hello-world",
//!   "name": "Hello World",
//!   "version": "1.0.0",
//!   "components": { "stat-card": "/components/stat-card" },
//!   "window": { "theme": "#2563eb", "addressBar": "hidden" }
//! }
//! ```
//!
//! Only `id` is required. The `window` object is kept as the exact JSON text the
//! publisher wrote, because it travels inside every page's signed manifest and
//! the viewer interprets it (`docs/reference/viewer.md`, "The shell"). Unknown
//! fields are ignored, so older tools can read newer sites.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::value::RawValue;
use thiserror::Error;

use crate::manifest::Manifest;

/// Why `vpp.json` could not be read.
#[derive(Debug, Error)]
pub enum SiteConfigError {
    /// The file is not valid JSON, or a field has the wrong type.
    #[error("vpp.json: {0}")]
    Json(#[from] serde_json::Error),
    /// There is no `id`, or it is empty.
    #[error("vpp.json: \"id\" is required, for example \"com.example.hello\"")]
    MissingId,
    /// `window` is present but is not a JSON object.
    #[error("vpp.json: \"window\" must be an object, for example {{ \"theme\": \"#2563eb\" }}")]
    WindowNotObject,
}

/// A site's `vpp.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteConfig {
    /// The site identifier, reverse-DNS style. Shared by every page and never changed.
    pub id: String,
    /// The name users see. Defaults to the id.
    pub name: String,
    /// The site version. Defaults to `0.0.0`.
    pub version: String,
    /// Custom tag names mapped to component folders, read by the compiler.
    pub components: BTreeMap<String, String>,
    /// The `window` object as the exact JSON text in the file, or empty.
    pub window: String,
}

/// The file as JSON, before defaults and checks.
#[derive(Deserialize)]
struct RawSiteConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    components: BTreeMap<String, String>,
    #[serde(default)]
    window: Option<Box<RawValue>>,
}

impl SiteConfig {
    /// Reads the text of a `vpp.json` file.
    pub fn parse(json: &str) -> Result<Self, SiteConfigError> {
        let raw: RawSiteConfig = serde_json::from_str(json)?;
        if raw.id.is_empty() {
            return Err(SiteConfigError::MissingId);
        }
        let window = match raw.window {
            None => String::new(),
            Some(value) if value.get().starts_with('{') => value.get().to_owned(),
            Some(_) => return Err(SiteConfigError::WindowNotObject),
        };
        Ok(Self {
            name: if raw.name.is_empty() {
                raw.id.clone()
            } else {
                raw.name
            },
            version: if raw.version.is_empty() {
                String::from("0.0.0")
            } else {
                raw.version
            },
            id: raw.id,
            components: raw.components,
            window,
        })
    }

    /// The manifest for one page of this site.
    pub fn manifest(&self, page: &str) -> Manifest {
        Manifest {
            site_id: self.id.clone(),
            site_name: self.name.clone(),
            site_version: self.version.clone(),
            page: page.to_owned(),
            window: self.window.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_field() {
        let config = SiteConfig::parse(
            r##"{
                "id": "org.vpp.examples.hello-world",
                "name": "Hello World",
                "version": "1.0.0",
                "components": { "stat-card": "/components/stat-card" },
                "window": { "theme": "#2563eb" }
            }"##,
        )
        .unwrap();
        assert_eq!(config.id, "org.vpp.examples.hello-world");
        assert_eq!(config.name, "Hello World");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.components["stat-card"], "/components/stat-card");
        assert_eq!(config.window, r##"{ "theme": "#2563eb" }"##);
    }

    #[test]
    fn only_the_id_is_required() {
        let config = SiteConfig::parse(r#"{ "id": "com.example.hello" }"#).unwrap();
        assert_eq!(config.name, "com.example.hello");
        assert_eq!(config.version, "0.0.0");
        assert!(config.components.is_empty());
        assert_eq!(config.window, "");
    }

    #[test]
    fn missing_or_empty_id_is_refused() {
        assert!(matches!(
            SiteConfig::parse(r#"{ "name": "x" }"#),
            Err(SiteConfigError::MissingId)
        ));
        assert!(matches!(
            SiteConfig::parse(r#"{ "id": "" }"#),
            Err(SiteConfigError::MissingId)
        ));
    }

    #[test]
    fn window_keeps_the_publishers_exact_text() {
        let window = "{\n    \"width\": 800,\n    \"height\": \"auto\" }";
        let json = format!(r#"{{ "id": "a", "window": {window} }}"#);
        assert_eq!(SiteConfig::parse(&json).unwrap().window, window);
    }

    #[test]
    fn window_must_be_an_object() {
        assert!(matches!(
            SiteConfig::parse(r#"{ "id": "a", "window": "big" }"#),
            Err(SiteConfigError::WindowNotObject)
        ));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert!(SiteConfig::parse(r#"{ "id": "a", "future": [1, 2] }"#).is_ok());
    }

    #[test]
    fn invalid_json_is_refused() {
        assert!(matches!(
            SiteConfig::parse("{ id: a }"),
            Err(SiteConfigError::Json(_))
        ));
    }

    #[test]
    fn manifest_for_a_page() {
        let config = SiteConfig::parse(r#"{ "id": "a", "name": "A", "window": {} }"#).unwrap();
        assert_eq!(
            config.manifest("home"),
            Manifest {
                site_id: "a".into(),
                site_name: "A".into(),
                site_version: "0.0.0".into(),
                page: "home".into(),
                window: "{}".into(),
            }
        );
    }
}
