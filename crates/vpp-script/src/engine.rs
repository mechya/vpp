//! The boundary between the viewer and a JavaScript engine. Everything the
//! viewer does with scripts goes through [`ScriptEngine`], so the engine
//! (QuickJS today) can be replaced without touching the viewer
//! (`docs/rust-port.md` §3).

use std::fmt;

use vpp_dom::NodeId;
use vpp_format::CodeBin;

/// A JavaScript engine running one page.
pub trait ScriptEngine {
    /// Runs one of the page's scripts. The page's scripts run in load order,
    /// in one shared global scope.
    fn run(&mut self, script: &CodeBin) -> Result<(), ScriptError>;

    /// Fires a click at `target` (an element, or text inside one) and lets it
    /// bubble up through the element's ancestors. An error in a listener is
    /// logged, never returned: one broken handler does not stop the page.
    fn dispatch_click(&mut self, target: NodeId);

    /// What scripts asked the viewer to do since the last call, in order.
    fn take_requests(&mut self) -> Vec<HostRequest>;
}

/// Something a script asks of the viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRequest {
    /// `console.log`: a line for the developer console.
    Log(String),
    /// `VPP.window.popup(message)`.
    Popup(String),
    /// `VPP.window.close()`.
    Close,
    /// `VPP.window.minimize()`.
    Minimize,
    /// `VPP.window.maximize()`: maximise, or restore if already maximised.
    Maximize,
    /// The document changed; style and layout must run again before the next paint.
    Invalidate,
}

/// A script that failed: a syntax error, an uncaught exception, or a limit reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    /// The script's file name.
    pub filename: String,
    /// The engine's message, with its stack trace when there is one.
    pub message: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.filename, self.message)
    }
}

impl std::error::Error for ScriptError {}
