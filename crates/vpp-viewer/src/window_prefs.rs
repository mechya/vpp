//! What a site asks of the window: the `window` object of its `vpp.json`
//! (`docs/reference/viewer.md`, "The shell").

use serde_json::Value;

/// A site's window preferences. Anything missing or malformed keeps its default.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowPrefs {
    /// `"addressBar": "hidden"` removes the address field but keeps the buttons.
    pub(crate) address_bar: bool,
    /// `"titleBar": false` hides the whole shell bar.
    pub(crate) title_bar: bool,
    /// The bar's colour, as CSS.
    pub(crate) theme: Option<String>,
    /// Rounded window corners, in CSS pixels.
    pub(crate) corner_radius: f32,
    /// The window's first width, in CSS pixels.
    pub(crate) width: Option<f32>,
    /// The window's first height, in CSS pixels.
    pub(crate) height: Option<f32>,
    /// `"height": "auto"`: the window's height follows the page.
    pub(crate) auto_height: bool,
    pub(crate) resizable: bool,
}

impl Default for WindowPrefs {
    fn default() -> Self {
        Self {
            address_bar: true,
            title_bar: true,
            theme: None,
            corner_radius: 0.0,
            width: None,
            height: None,
            auto_height: false,
            resizable: true,
        }
    }
}

/// A window size a site may ask for, in CSS pixels: large enough to use,
/// small enough not to swallow the screen.
const SIZE_RANGE: std::ops::RangeInclusive<f64> = 80.0..=8000.0;

impl WindowPrefs {
    /// Reads the `window` object from its JSON text; empty text is the defaults.
    pub(crate) fn parse(json: &str) -> Self {
        let mut prefs = Self::default();
        let Ok(Value::Object(window)) = serde_json::from_str::<Value>(json) else {
            return prefs;
        };
        if window.get("addressBar").and_then(Value::as_str) == Some("hidden") {
            prefs.address_bar = false;
        }
        if let Some(shown) = window.get("titleBar").and_then(Value::as_bool) {
            prefs.title_bar = shown;
        }
        prefs.theme = window
            .get("theme")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if let Some(radius) = window.get("cornerRadius").and_then(Value::as_f64) {
            prefs.corner_radius = radius.clamp(0.0, 100.0) as f32;
        }
        prefs.width = size(window.get("width"));
        match window.get("height") {
            Some(Value::String(auto)) if auto == "auto" => prefs.auto_height = true,
            height => prefs.height = size(height),
        }
        prefs.resizable = window
            .get("resizable")
            .and_then(Value::as_bool)
            .unwrap_or(!prefs.auto_height);
        prefs
    }
}

fn size(value: Option<&Value>) -> Option<f32> {
    let n = value?.as_f64()?;
    SIZE_RANGE.contains(&n).then_some(n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_broken_is_the_default() {
        assert_eq!(WindowPrefs::parse(""), WindowPrefs::default());
        assert_eq!(WindowPrefs::parse("[1]"), WindowPrefs::default());
        assert_eq!(
            WindowPrefs::parse("{\"titleBar\": \"no\""),
            WindowPrefs::default()
        );
    }

    #[test]
    fn every_option() {
        let prefs = WindowPrefs::parse(
            r##"{"theme": "#2563eb", "addressBar": "hidden", "titleBar": false,
                "cornerRadius": 12, "width": 800, "height": "auto"}"##,
        );
        assert_eq!(
            prefs,
            WindowPrefs {
                address_bar: false,
                title_bar: false,
                theme: Some("#2563eb".into()),
                corner_radius: 12.0,
                width: Some(800.0),
                height: None,
                auto_height: true,
                resizable: false,
            }
        );
        let prefs = WindowPrefs::parse(r#"{"height": "auto", "resizable": true}"#);
        assert!(prefs.auto_height && prefs.resizable);
    }

    #[test]
    fn sizes_out_of_range_are_ignored() {
        let prefs = WindowPrefs::parse(r#"{"width": 5, "height": 1e9}"#);
        assert_eq!((prefs.width, prefs.height), (None, None));
    }
}
