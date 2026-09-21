//! `vpp-viewer`, the desktop viewer.
//!
//! Starts the viewer on the desktop: reads the command line, creates the window's event loop, and hands it to `vpp-viewer`.
//!
//! # Where it sits
//!
//! An entry point: nothing depends on it. Windows first; macOS and Linux use this same crate later.
//!
//! # Not in this crate
//!
//! Anything the viewer does once it runs (`vpp-viewer`), and operating-system services (`vpp-platform`).
//!
//! # Where to start reading
//!
//! `main` below, then `window.rs`, which passes window events to the viewer
//! (keys through `keys.rs`) and shows what it draws with `present.rs`.

// A release build is a windowed program, so starting it from Explorer opens
// no console. Debug builds keep the console for the log.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod cli;
mod keys;
mod present;
mod window;

use std::path::PathBuf;
use std::process::ExitCode;

use vpp_viewer::{Font, FontSet, HttpFetcher, ViewerOptions};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::cli::{Command, parse_args};
use crate::window::DesktopApp;

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(Command::Run(args)) => args,
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Ok(Command::Version) => {
            println!("vpp-viewer {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("vpp-viewer: {message}\n\n{}", cli::USAGE);
            return ExitCode::FAILURE;
        }
    };
    let Some(fonts) = load_fonts() else {
        eprintln!("vpp-viewer: no system font found, so no text could be drawn");
        return ExitCode::FAILURE;
    };
    let options = ViewerOptions {
        fonts,
        data_dir: data_dir(),
        allow_unsigned: args.allow_unsigned,
        fetch: Box::new(HttpFetcher::new()),
    };

    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(e) => {
            eprintln!("vpp-viewer: cannot start the window system: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Sleep until something happens: the viewer only draws when asked.
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = DesktopApp::new(options, args.address);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("vpp-viewer: {e}");
        return ExitCode::FAILURE;
    }
    app.exit_code()
}

/// The first system font family that loads.
fn load_fonts() -> Option<FontSet> {
    vpp_platform::font_candidates()
        .into_iter()
        .find_map(|files| {
            let regular = Font::load(&files.regular).ok()?;
            let bold = files.bold.and_then(|bold| Font::load(&bold).ok());
            println!(
                "font: {} (bold {})",
                files.regular.display(),
                if bold.is_some() { "yes" } else { "no" }
            );
            Some(FontSet { regular, bold })
        })
}

/// The platform's data folder, or a temporary one if it has none.
fn data_dir() -> PathBuf {
    vpp_platform::data_dir().unwrap_or_else(|| {
        let dir = std::env::temp_dir().join("vpp-viewer");
        println!(
            "warning: no data folder on this system; using {} until the viewer closes",
            dir.display()
        );
        dir
    })
}
