//! Template errors, reported against the file they came from.

use std::fmt;
use std::path::{Path, PathBuf};

/// A template could not be expanded. Displays as `file: message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateError {
    /// The file where the problem is.
    pub file: PathBuf,
    /// What is wrong, in words a site author can act on.
    pub message: String,
}

impl TemplateError {
    pub(crate) fn new(file: &Path, message: impl Into<String>) -> Self {
        Self {
            file: file.to_path_buf(),
            message: message.into(),
        }
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.message)
    }
}

impl std::error::Error for TemplateError {}
