//! [`ScriptEngine`] on QuickJS, through `rquickjs`.
//!
//! One runtime per page, as each page is its own program
//! (`ARCHITECTURE.md`, invariant 6). The runtime has a memory limit, and each
//! script run or event dispatch has a time limit, so a runaway page cannot
//! exhaust memory or freeze the viewer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use rquickjs::context::EvalOptions;
use rquickjs::{CatchResultExt, CaughtError, Context, Function, Persistent, Runtime};
use vpp_dom::{Document, NodeId};
use vpp_format::CodeBin;

use crate::bindings::{Handles, Shared, natives};
use crate::engine::{HostRequest, ScriptEngine, ScriptError};

const PRELUDE: &str = include_str!("prelude.js");

/// The most memory one page's scripts may use.
pub const MEMORY_LIMIT: usize = 256 * 1024 * 1024;

/// The longest one script run or one event dispatch may take.
pub const TIME_LIMIT: Duration = Duration::from_secs(5);

/// A page's scripts, running on QuickJS.
pub struct QuickJsEngine {
    // Dropped before the runtime: the context and the saved function belong to it.
    dispatch: Persistent<Function<'static>>,
    context: Context,
    runtime: Runtime,
    shared: Rc<Shared>,
    deadline: Rc<Cell<Option<Instant>>>,
    time_limit: Duration,
}

impl QuickJsEngine {
    /// An engine for `document`, which scripts read and change. The viewer
    /// keeps its own reference to lay out and paint the same document.
    pub fn new(document: Rc<RefCell<Document>>) -> Result<Self, ScriptError> {
        Self::with_time_limit(document, TIME_LIMIT)
    }

    /// An engine with a different time limit per run and per dispatch.
    pub fn with_time_limit(
        document: Rc<RefCell<Document>>,
        time_limit: Duration,
    ) -> Result<Self, ScriptError> {
        let setup = |e: rquickjs::Error| ScriptError {
            filename: "(setup)".into(),
            message: e.to_string(),
        };
        let runtime = Runtime::new().map_err(setup)?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        let deadline: Rc<Cell<Option<Instant>>> = Rc::default();
        let watched = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            watched.get().is_some_and(|d| Instant::now() > d)
        })));

        let context = Context::full(&runtime).map_err(setup)?;
        let shared = Rc::new(Shared {
            document,
            handles: RefCell::new(Handles::default()),
            requests: RefCell::new(Vec::new()),
        });
        let dispatch = context.with(|ctx| -> rquickjs::Result<_> {
            let prelude: Function = ctx.eval(PRELUDE)?;
            let dispatch: Function = prelude.call((natives(&ctx, &shared)?,))?;
            Ok(Persistent::save(&ctx, dispatch))
        });
        Ok(Self {
            dispatch: dispatch.map_err(setup)?,
            context,
            runtime,
            shared,
            deadline,
            time_limit,
        })
    }

    /// Runs `work` with the time limit armed, then any promise jobs it queued.
    fn guarded<T>(&self, work: impl FnOnce(&Context) -> T) -> T {
        self.deadline.set(Some(Instant::now() + self.time_limit));
        let result = work(&self.context);
        while let Ok(true) = self.runtime.execute_pending_job() {}
        self.deadline.set(None);
        result
    }

    fn log(&self, line: String) {
        self.shared
            .requests
            .borrow_mut()
            .push(HostRequest::Log(line));
    }
}

impl ScriptEngine for QuickJsEngine {
    fn run(&mut self, script: &CodeBin) -> Result<(), ScriptError> {
        let filename = script.filename.clone();
        self.guarded(|context| {
            context.with(|ctx| {
                let mut options = EvalOptions::default();
                options.global = true;
                options.strict = false;
                options.filename = Some(filename.clone());
                ctx.eval_with_options::<(), _>(script.source.as_str(), options)
                    .catch(&ctx)
                    .map_err(|caught| ScriptError {
                        filename,
                        message: describe(caught),
                    })
            })
        })
    }

    fn dispatch_click(&mut self, target: NodeId) {
        let chain: Vec<u32> = {
            let document = self.shared.document.borrow();
            if !document.contains(target) {
                return;
            }
            let mut handles = self.shared.handles.borrow_mut();
            std::iter::successors(Some(target), |&n| document[n].parent())
                .filter(|&n| document.element(n).is_some())
                .map(|n| handles.handle(n))
                .collect()
        };
        if chain.is_empty() {
            return;
        }
        // Listeners' own errors are caught and logged by the dispatcher;
        // what reaches here is the dispatch itself failing, such as the time limit.
        let dispatch = self.dispatch.clone();
        let outcome = self.guarded(|context| {
            context.with(|ctx| -> Result<(), String> {
                let dispatch = dispatch.restore(&ctx).map_err(|e| e.to_string())?;
                dispatch
                    .call::<_, ()>(("click", chain))
                    .catch(&ctx)
                    .map_err(describe)
            })
        });
        if let Err(message) = outcome {
            self.log(format!("uncaught: {message}"));
        }
    }

    fn take_requests(&mut self) -> Vec<HostRequest> {
        std::mem::take(&mut self.shared.requests.borrow_mut())
    }
}

/// An error from JavaScript as one message, with the stack trace when there is one.
fn describe(caught: CaughtError<'_>) -> String {
    match caught {
        CaughtError::Exception(exception) => {
            let message = exception.message().unwrap_or_else(|| "error".into());
            match exception.stack().filter(|s| !s.trim().is_empty()) {
                Some(stack) => format!("{message}\n{}", stack.trim_end()),
                None => message,
            }
        }
        other => other.to_string(),
    }
}
