//! The command line (`docs/reference/viewer.md`).

pub(crate) const USAGE: &str = "\
Usage: vpp-viewer [ADDRESS] [--allow-unsigned]

ADDRESS is a page's .html source, a compiled dist folder, a .vpp package,
or an https:// or vpp:// URL. Without one, the viewer shows its start page.

Options:
  --allow-unsigned  Accept unsigned packages. For development only.
  -h, --help        Show this help.
  -V, --version     Show the viewer version.";

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Run(Args),
    Help,
    Version,
}

/// The viewer's settings from the command line.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    /// The page to open, or empty for the start page.
    pub(crate) address: String,
    pub(crate) allow_unsigned: bool,
}

/// Reads the arguments after the program name.
pub(crate) fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut parsed = Args::default();
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--allow-unsigned" => parsed.allow_unsigned = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ if parsed.address.is_empty() => parsed.address = arg,
            _ => return Err(format!("more than one address: {arg}")),
        }
    }
    Ok(Command::Run(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn address_and_flag_in_any_order() {
        let expected = Command::Run(Args {
            address: "home.vpp".into(),
            allow_unsigned: true,
        });
        assert_eq!(parse(&["home.vpp", "--allow-unsigned"]), Ok(expected));
        assert_eq!(
            parse(&["--allow-unsigned", "home.vpp"]),
            parse(&["home.vpp", "--allow-unsigned"])
        );
        assert_eq!(parse(&[]), Ok(Command::Run(Args::default())));
    }

    #[test]
    fn help_version_and_mistakes() {
        assert_eq!(parse(&["x.vpp", "--help"]), Ok(Command::Help));
        assert_eq!(parse(&["-V"]), Ok(Command::Version));
        assert!(parse(&["--allow-unsinged"]).is_err());
        assert!(parse(&["a.vpp", "b.vpp"]).is_err());
    }
}
