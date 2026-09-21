//! The command line, kept the same as the C++ `vppc` (`docs/reference/compiler.md`):
//!
//! ```text
//! vppc <project dir>
//! vppc <page.html>
//! vppc <app.js> [-o <output.bin>]
//! ```
//!
//! `-g` is still accepted and ignored.

use std::path::PathBuf;

use clap::Parser;

/// Compiles VPP pages into dom.bin, style/*.bin, and code/*.bin, checking every script for syntax errors.
#[derive(Debug, Parser)]
#[command(name = "vppc", version, override_usage = USAGE)]
pub(crate) struct Cli {
    /// A project directory, one .html page, or one .js script.
    input: PathBuf,

    /// Output file, for a single script.
    #[arg(short, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Ignored, kept so old build scripts still work: scripts now ship as
    /// source, which always keeps line numbers (docs/design/0001-code-bin.md).
    #[arg(short = 'g', hide = true)]
    _debug_info: bool,
}

const USAGE: &str = "vppc <project dir>
       vppc <page.html>
       vppc <app.js> [-o <output.bin>]";

/// What to compile.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Every page of a site.
    Project { root: PathBuf },
    /// One page, into its project's `dist/`.
    Page { file: PathBuf },
    /// One script to a `code.bin` container.
    Script {
        file: PathBuf,
        output: Option<PathBuf>,
    },
}

impl Cli {
    /// What the input is, judged by whether it is a directory and by its extension.
    pub(crate) fn command(&self) -> Result<Command, String> {
        if self.input.is_dir() {
            return Ok(Command::Project {
                root: self.input.clone(),
            });
        }
        let extension = self
            .input
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        match extension.as_str() {
            "html" | "htm" => Ok(Command::Page {
                file: self.input.clone(),
            }),
            "js" | "mjs" => Ok(Command::Script {
                file: self.input.clone(),
                output: self.output.clone(),
            }),
            _ => Err(format!(
                "{}: expected a project directory, an .html page, or a .js script",
                self.input.display()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(args: &[&str]) -> Result<Command, String> {
        Cli::try_parse_from(std::iter::once("vppc").chain(args.iter().copied()))
            .map_err(|e| e.to_string())?
            .command()
    }

    #[test]
    fn chooses_by_directory_or_extension() {
        let here = std::env::temp_dir();
        assert_eq!(
            command(&[here.to_str().unwrap()]),
            Ok(Command::Project { root: here.clone() })
        );
        assert_eq!(
            command(&["pages/Home.HTML"]),
            Ok(Command::Page {
                file: "pages/Home.HTML".into()
            })
        );
        assert_eq!(
            command(&["app.js", "-o", "app.bin", "-g"]),
            Ok(Command::Script {
                file: "app.js".into(),
                output: Some("app.bin".into())
            })
        );
        assert!(command(&["notes.txt"]).is_err());
        assert!(command(&[]).is_err());
    }
}
