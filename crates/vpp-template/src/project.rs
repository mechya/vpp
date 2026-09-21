//! The project directory, and resolving references inside it.
//!
//! References starting with `/` are from the project root; others are relative
//! to the file that contains them. A reference that leaves the project
//! directory is an error (`docs/reference/compiler.md`, "Templates").

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::error::TemplateError;

/// Where a site lives and how its custom tags map to components.
#[derive(Debug, Clone, Default)]
pub struct TemplateOptions {
    /// The directory containing `vpp.json`. `/x` references resolve here.
    pub project_root: PathBuf,
    /// Custom tag name to the project path of its component directory, from
    /// `"components"` in `vpp.json`, for example `stat-card` → `/components/stat-card`.
    pub component_aliases: BTreeMap<String, String>,
}

/// The nearest directory containing `vpp.json`, searching upward from `file`;
/// the file's own directory if none is found.
pub fn find_project_root(file: &Path) -> PathBuf {
    let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    dir.ancestors()
        .find(|d| d.join("vpp.json").is_file())
        .map(Path::to_path_buf)
        .unwrap_or(dir)
}

/// The project root in the two forms that references are checked against.
pub(crate) struct Project {
    /// Absolute and normalised, but with symbolic links as written.
    pub(crate) root: PathBuf,
    /// With symbolic links resolved, to check existing files against.
    canonical_root: PathBuf,
}

impl Project {
    pub(crate) fn new(root: &Path) -> Result<Self, TemplateError> {
        let absolute = std::path::absolute(root)
            .map_err(|e| TemplateError::new(root, format!("cannot use as project root: {e}")))?;
        let root = normalize(&absolute);
        let canonical_root = root.canonicalize().map_err(|e| {
            TemplateError::new(&root, format!("cannot read project directory: {e}"))
        })?;
        Ok(Self {
            root,
            canonical_root,
        })
    }

    /// Resolves `reference` against `base_dir` (or the root, for `/x`), refusing
    /// anything outside the project. Errors are reported against `in_file`.
    pub(crate) fn resolve(
        &self,
        reference: &str,
        base_dir: &Path,
        in_file: &Path,
    ) -> Result<PathBuf, TemplateError> {
        let candidate = match reference.strip_prefix(['/', '\\']) {
            Some(from_root) => self.root.join(from_root),
            None => base_dir.join(reference),
        };
        let candidate = normalize(&candidate);

        let mut inside = candidate.starts_with(&self.root);
        // A symbolic link inside the project may point outside it.
        if inside {
            if let Ok(real) = candidate.canonicalize() {
                inside = real.starts_with(&self.canonical_root);
            }
        }
        if !inside {
            return Err(TemplateError::new(
                in_file,
                format!("reference \"{reference}\" leaves the project directory"),
            ));
        }
        Ok(candidate)
    }

    /// `path` as a project-absolute reference, such as `/styles/global.css`.
    pub(crate) fn project_path(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        let parts: Vec<_> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        format!("/{}", parts.join("/"))
    }
}

/// Removes `.` and resolves `..` without touching the file system, so paths to
/// files that do not exist yet (optional includes) still resolve.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                // Never above the root or drive: `pop` returns false there.
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TestDir;

    #[test]
    fn resolves_absolute_and_relative_references() {
        let dir = TestDir::new("resolve");
        dir.write("vpp.json", b"{}");
        let project = Project::new(dir.path()).unwrap();
        let pages = project.root.join("pages");
        let page = pages.join("home.html");

        let global = project
            .resolve("/styles/global.css", &pages, &page)
            .unwrap();
        assert_eq!(project.project_path(&global), "/styles/global.css");
        let sibling = project
            .resolve("../includes/x.html", &pages, &page)
            .unwrap();
        assert_eq!(project.project_path(&sibling), "/includes/x.html");
    }

    #[test]
    fn refuses_references_outside_the_project() {
        let dir = TestDir::new("escape");
        let project = Project::new(dir.path()).unwrap();
        let page = project.root.join("home.html");
        for reference in ["../secret.txt", "/../secret.txt", "a/../../b"] {
            let err = project
                .resolve(reference, &project.root, &page)
                .unwrap_err();
            assert!(err.message.contains("leaves the project"), "{reference}");
            assert_eq!(err.file, page);
        }
    }

    #[test]
    fn finds_the_nearest_vpp_json() {
        let dir = TestDir::new("root");
        dir.write("vpp.json", b"{}");
        dir.write("pages/deep/page.html", b"");
        let found = find_project_root(&dir.path().join("pages/deep/page.html"));
        assert_eq!(
            found.canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }
}
