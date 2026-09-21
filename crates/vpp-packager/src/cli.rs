//! The command line, kept the same as the C++ `vpppack` (`docs/reference/packager.md`):
//!
//! ```text
//! vpppack keygen [-o <name>]
//! vpppack <project dir> [--key <file.key>] [-o <out dir>] [--publish]
//! vpppack --inspect <file.vpp | file.vppm>
//! ```

use std::path::PathBuf;

use clap::Parser;

/// Builds, signs, publishes, and inspects VPP page packages.
#[derive(Debug, Parser)]
#[command(name = "vpppack", version, override_usage = USAGE)]
pub(crate) struct Cli {
    /// `keygen`, or the project directory whose compiled pages (dist/) to package.
    target: Option<String>,

    /// Secret key file to sign with.
    #[arg(short, long, value_name = "FILE")]
    key: Option<PathBuf>,

    /// Output directory (default <project>/out), or the key name for keygen (default "publisher").
    #[arg(short, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Also write the hosting layout: <page>.vppm manifests and res/ with every resource by SHA-256.
    #[arg(short, long)]
    publish: bool,

    /// Show a package's or manifest's contents and verify it.
    #[arg(short, long, value_name = "FILE", conflicts_with_all = ["target", "key", "publish"])]
    inspect: Option<PathBuf>,
}

const USAGE: &str = "vpppack keygen [-o <name>]
       vpppack <project dir> [--key <file.key>] [-o <out dir>] [--publish]
       vpppack --inspect <file.vpp | file.vppm>";

/// What to do, once the arguments are checked.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Create `<name>.key` and `<name>.pub`.
    Keygen { name: PathBuf },
    /// Package every compiled page of a project.
    Pack {
        project: PathBuf,
        key: Option<PathBuf>,
        output: Option<PathBuf>,
        publish: bool,
    },
    /// Show and verify a package or manifest.
    Inspect { file: PathBuf },
}

impl Cli {
    /// The command these arguments ask for, or a usage error message.
    pub(crate) fn command(self) -> Result<Command, String> {
        if let Some(file) = self.inspect {
            return Ok(Command::Inspect { file });
        }
        match self.target.as_deref() {
            Some("keygen") => {
                if self.key.is_some() || self.publish {
                    return Err("keygen takes only -o <name>".into());
                }
                Ok(Command::Keygen {
                    name: self.output.unwrap_or_else(|| PathBuf::from("publisher")),
                })
            }
            Some(project) => Ok(Command::Pack {
                project: PathBuf::from(project),
                key: self.key,
                output: self.output,
                publish: self.publish,
            }),
            None => Err("nothing to do: give a project directory, keygen, or --inspect".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(args: &[&str]) -> Result<Command, String> {
        let cli = Cli::try_parse_from(std::iter::once("vpppack").chain(args.iter().copied()))
            .map_err(|e| e.to_string())?;
        cli.command()
    }

    #[test]
    fn keygen_with_and_without_a_name() {
        assert_eq!(
            command(&["keygen"]),
            Ok(Command::Keygen {
                name: "publisher".into()
            })
        );
        assert_eq!(
            command(&["keygen", "-o", "site"]),
            Ok(Command::Keygen {
                name: "site".into()
            })
        );
    }

    #[test]
    fn pack_with_every_option() {
        assert_eq!(
            command(&["site", "--key", "p.key", "-o", "public", "--publish"]),
            Ok(Command::Pack {
                project: "site".into(),
                key: Some("p.key".into()),
                output: Some("public".into()),
                publish: true,
            })
        );
        assert_eq!(
            command(&["site", "-k", "p.key", "-p"]),
            Ok(Command::Pack {
                project: "site".into(),
                key: Some("p.key".into()),
                output: None,
                publish: true,
            })
        );
    }

    #[test]
    fn inspect() {
        assert_eq!(
            command(&["--inspect", "home.vpp"]),
            Ok(Command::Inspect {
                file: "home.vpp".into()
            })
        );
        assert!(command(&["site", "--inspect", "home.vpp"]).is_err());
    }

    #[test]
    fn nothing_to_do_is_a_usage_error() {
        assert!(command(&[]).is_err());
        assert!(command(&["keygen", "--publish"]).is_err());
    }
}
