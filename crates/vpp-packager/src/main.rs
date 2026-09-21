//! `vpppack`, the VPP packager.
//!
//! Creates publisher keys, and hashes, signs, and publishes compiled pages as
//! `.vpp` packages, `.vppm` manifests, and a shared `res/` folder. Its
//! behaviour and output follow the C++ tool (`docs/reference/packager.md`).
//!
//! # Where it sits
//!
//! The second development step, after `vppc` has compiled a site into `dist/`.
//! Its output is what a static server hosts.
//!
//! # Not in this crate
//!
//! Compiling pages (`vpp-compiler`), and the package format itself (`vpp-format`).
//!
//! # Where to start reading
//!
//! `cli.rs` for the command line, then `publish.rs` for packaging a site.

mod cli;
mod inspect;
mod keygen;
mod publish;
#[cfg(test)]
mod test_dir;

use std::io;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::publish::PackOptions;

fn main() -> ExitCode {
    let command = match Cli::parse().command() {
        Ok(command) => command,
        Err(message) => {
            eprintln!("vpppack: {message}\n\nRun vpppack --help for usage.");
            return ExitCode::from(2);
        }
    };

    let mut out = io::stdout().lock();
    let result = match &command {
        Command::Keygen { name } => keygen::keygen(name, &mut out),
        Command::Pack {
            project,
            key,
            output,
            publish,
        } => publish::pack(
            &PackOptions {
                project,
                key: key.as_deref(),
                output: output.as_deref(),
                publish: *publish,
            },
            &mut out,
        ),
        Command::Inspect { file } => match inspect::inspect(file, &mut out) {
            Ok(true) => Ok(()),
            Ok(false) => Err(anyhow::anyhow!("verification failed")),
            Err(e) => Err(e),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vpppack: {error:#}");
            ExitCode::FAILURE
        }
    }
}
