//! Checking a script's syntax without running it, as `vppc` does for every
//! script before packaging it (`docs/design/0001-code-bin.md`).

use std::ffi::CString;
use std::fmt;

use rquickjs::{Context, Ctx, Exception, Runtime, qjs};

/// A script that does not parse, with where the engine stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// The file name given to [`check_syntax`].
    pub filename: String,
    /// The line, counting from 1, if the engine reported one.
    pub line: Option<u32>,
    /// The column, counting from 1, if the engine reported one.
    pub column: Option<u32>,
    /// The engine's message, such as `unexpected token in expression: ')'`.
    pub message: String,
}

/// `file:line:col: message`, the format every VPP tool uses for source errors.
impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filename)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for SyntaxError {}

/// Compiles `source` as a classic script and discards the result, reporting
/// the first syntax error. Nothing in the script runs.
pub fn check_syntax(source: &str, filename: &str) -> Result<(), SyntaxError> {
    let error = |message: String| SyntaxError {
        filename: filename.to_owned(),
        line: None,
        column: None,
        message,
    };
    // QuickJS reads source and file name as NUL-terminated C strings.
    let source_c =
        CString::new(source).map_err(|_| error("the script contains a NUL character".into()))?;
    let filename_c = CString::new(filename)
        .map_err(|_| error("the file name contains a NUL character".into()))?;

    let runtime = Runtime::new().map_err(|e| error(format!("cannot start QuickJS: {e}")))?;
    let context =
        Context::full(&runtime).map_err(|e| error(format!("cannot start QuickJS: {e}")))?;
    context.with(|ctx| {
        let failed = compile_only(&ctx, &source_c, &filename_c);
        if !failed {
            return Ok(());
        }
        let caught = ctx.catch();
        let exception = caught.into_object().and_then(Exception::from_object);
        let message = exception
            .as_ref()
            .and_then(Exception::message)
            .unwrap_or_else(|| "syntax error".into());
        let (line, column) = exception
            .as_ref()
            .and_then(Exception::stack)
            .and_then(|stack| location(&stack, filename))
            .unzip();
        Err(SyntaxError {
            filename: filename.to_owned(),
            line,
            column,
            message,
        })
    })
}

/// Runs QuickJS's compiler on the source without executing it. Returns
/// whether it failed; the error is then pending in the context.
#[allow(unsafe_code)]
fn compile_only(ctx: &Ctx<'_>, source: &CString, filename: &CString) -> bool {
    let raw = ctx.as_raw().as_ptr();
    let flags = (qjs::JS_EVAL_TYPE_GLOBAL | qjs::JS_EVAL_FLAG_COMPILE_ONLY) as i32;
    // SAFETY: `raw` comes from `ctx`, a live context borrowed for this whole
    // call on this thread. `source` and `filename` are NUL-terminated and
    // outlive the call, and the length excludes the terminator, as JS_Eval
    // requires. COMPILE_ONLY returns the compiled function without running it.
    let value = unsafe {
        qjs::JS_Eval(
            raw,
            source.as_ptr(),
            source.as_bytes().len() as _,
            filename.as_ptr(),
            flags,
        )
    };
    // SAFETY: `value` was just returned by JS_Eval on `raw` and is owned here.
    // It is freed exactly once: either the compiled function, or the exception
    // marker, which holds no reference and is safe to free.
    unsafe {
        let failed = qjs::JS_IsException(value);
        qjs::JS_FreeValue(raw, value);
        failed
    }
}

/// The line and column of `filename` in the first stack line that names it,
/// such as `    at app.js:3:7`.
fn location(stack: &str, filename: &str) -> Option<(u32, u32)> {
    let prefix = format!("{filename}:");
    stack.lines().find_map(|line| {
        let at = line.find(&prefix)?;
        let rest = &line[at + prefix.len()..];
        let mut numbers = rest
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().ok());
        Some((numbers.next()??, numbers.next()??))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_scripts_pass_without_running() {
        assert_eq!(
            check_syntax("let a = 1;\nfunction f() { return a; }", "ok.js"),
            Ok(())
        );
        // Would throw if it ran.
        assert_eq!(check_syntax("throw new Error('ran');", "ok.js"), Ok(()));
        // Sloppy mode, as classic scripts are: `with` is allowed.
        assert_eq!(check_syntax("with (Math) { max(1, 2); }", "ok.js"), Ok(()));
    }

    #[test]
    fn syntax_errors_say_where() {
        let err = check_syntax("let a = 1;\nlet b = (;\n", "app.js").unwrap_err();
        assert_eq!(err.filename, "app.js");
        assert_eq!(err.line, Some(2));
        assert!(err.column.is_some());
        assert!(!err.message.is_empty());
        assert!(err.to_string().starts_with("app.js:2:"), "{err}");
    }

    #[test]
    fn script_only_errors_are_caught() {
        // Legal in a function body, not in a script: why the check compiles a real script.
        let err = check_syntax("return 1;", "app.js").unwrap_err();
        assert!(err.message.contains("return"), "{err}");
    }

    #[test]
    fn nul_characters_are_refused() {
        let err = check_syntax("a\0b", "app.js").unwrap_err();
        assert!(err.message.contains("NUL"));
    }

    #[test]
    fn reads_the_location_from_a_stack_line() {
        assert_eq!(
            location("SyntaxError: x\n    at app.js:12:5\n", "app.js"),
            Some((12, 5))
        );
        assert_eq!(location("    at other.js:1:1", "app.js"), None);
    }
}
