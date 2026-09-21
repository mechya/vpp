//! `vppc`, the VPP compiler.
//!
//! Compiles a page and its layouts, components, CSS, and JavaScript into
//! `dom.bin`, `style.bin`, and `code.bin`. Its behaviour and output follow the
//! C++ tool (`docs/reference/compiler.md`).
//!
//! # Where it sits
//!
//! The first development step. Its output goes to the packager.
//!
//! # Not in this crate
//!
//! Template expansion (`vpp-template`), the binary formats (`vpp-dom`,
//! `vpp-style`, `vpp-format`), the syntax check (`vpp-script`), and signing
//! and publishing (`vpp-packager`).
//!
//! Scripts are checked for syntax errors with QuickJS and shipped as source
//! in `code.bin` (`docs/design/0001-code-bin.md`).
//!
//! # Where to start reading
//!
//! `cli.rs` for the command line, then `compile.rs`.

mod cli;
mod compile;
#[cfg(test)]
mod test_dir;

use std::io;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let command = match cli.command() {
        Ok(command) => command,
        Err(message) => {
            eprintln!("vppc: {message}\n\nRun vppc --help for usage.");
            return ExitCode::from(2);
        }
    };

    let mut out = io::stdout().lock();
    let result = match &command {
        Command::Project { root } => std::path::absolute(root)
            .map_err(anyhow::Error::from)
            .and_then(|root| compile::compile_project(&root, &root.join("dist"), &mut out)),
        Command::Page { file } => compile::compile_single_page(file, &mut out),
        Command::Script { file, output } => {
            compile::compile_script(file, output.as_deref(), &mut out)
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vppc: {error:#}");
            ExitCode::FAILURE
        }
    }
}
