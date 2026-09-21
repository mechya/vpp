//! Windows services for `vpp-platform`.

use std::env;
use std::path::PathBuf;

use crate::FontFiles;

/// `%LOCALAPPDATA%\VPP\Viewer`: local, not roaming, because installed sites
/// can be large and are downloaded again when needed.
pub(crate) fn data_dir() -> Option<PathBuf> {
    let local = env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
    Some(PathBuf::from(local).join("VPP").join("Viewer"))
}

/// Reads text through `arboard`.
pub(crate) fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Segoe UI, the Windows interface font, then Arial.
pub(crate) fn font_candidates() -> Vec<FontFiles> {
    let windows = env::var_os("WINDIR")
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
    let fonts = windows.join("Fonts");
    [
        ("segoeui.ttf", "segoeuib.ttf"),
        ("arial.ttf", "arialbd.ttf"),
    ]
    .into_iter()
    .map(|(regular, bold)| FontFiles {
        regular: fonts.join(regular),
        bold: Some(fonts.join(bold)),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_folder_is_the_viewers_own() {
        let dir = data_dir().expect("Windows sets LOCALAPPDATA");
        assert!(dir.ends_with(r"VPP\Viewer"), "{}", dir.display());
    }

    #[test]
    fn segoe_ui_comes_first_and_exists() {
        let first = &font_candidates()[0];
        assert!(first.regular.ends_with("segoeui.ttf"));
        assert!(first.regular.is_file(), "{}", first.regular.display());
    }
}
