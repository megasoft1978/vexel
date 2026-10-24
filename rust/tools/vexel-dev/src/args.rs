//! Command-line parsing for `vexel-dev` (pure; no I/O).
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Info,
    Build,
    Check,
    CheckCi {
        workflow: PathBuf,
    },
    Bundle {
        out: Option<PathBuf>,
        icon: Option<PathBuf>,
        bundle_id: Option<String>,
        display_name: Option<String>,
    },
    Dmg {
        out: Option<PathBuf>,
    },
    Sbom {
        out: Option<PathBuf>,
    },
    InstallSmoke {
        out: Option<PathBuf>,
    },
    Notarize {
        path: PathBuf,
        execute: bool,
    },
    /// Harness arguments: `run ...` or `check ...`.
    Bench {
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    NoCommand,
    UnknownCommand(String),
    UnknownFlag { command: String, flag: String },
    MissingValue { flag: String },
    MissingArgument { command: String, argument: String },
    DuplicateFlag { flag: String },
    UnexpectedArgument { command: String, argument: String },
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCommand => write!(f, "missing command"),
            Self::UnknownCommand(name) => write!(f, "unknown command `{name}`"),
            Self::UnknownFlag { command, flag } => {
                write!(f, "`{command}` does not accept flag `{flag}`")
            }
            Self::MissingValue { flag } => write!(f, "flag `{flag}` needs a value"),
            Self::MissingArgument { command, argument } => {
                write!(f, "`{command}` needs {argument}")
            }
            Self::DuplicateFlag { flag } => write!(f, "flag `{flag}` was given more than once"),
            Self::UnexpectedArgument { command, argument } => {
                write!(f, "`{command}` takes no argument `{argument}`")
            }
        }
    }
}

pub const USAGE: &str = "Usage: vexel-dev info|build|check\n       \
vexel-dev check-ci WORKFLOW.yml\n       \
vexel-dev bundle [--out DIR] [--icon FILE.icns]\n       \
vexel-dev dmg|sbom|install-smoke [--out DIR]\n       \
vexel-dev bench [--check] [--update-baseline] [--hash-only|--full] [--only TEXT] [--out DIR]";

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Command, UsageError> {
    let (name, rest) = args.split_first().ok_or(UsageError::NoCommand)?;
    match name.as_str() {
        "info" | "build" | "check" => {
            if let Some(extra) = rest.first() {
                return Err(UsageError::UnexpectedArgument {
                    command: name.clone(),
                    argument: extra.clone(),
                });
            }
            Ok(match name.as_str() {
                "info" => Command::Info,
                "build" => Command::Build,
                _ => Command::Check,
            })
        }
        "check-ci" => {
            let workflow = rest.first().ok_or_else(|| UsageError::MissingArgument {
                command: name.clone(),
                argument: "WORKFLOW.yml".into(),
            })?;
            if let Some(extra) = rest.get(1) {
                return Err(UsageError::UnexpectedArgument {
                    command: name.clone(),
                    argument: extra.clone(),
                });
            }
            Ok(Command::CheckCi {
                workflow: PathBuf::from(workflow),
            })
        }
        "bundle" => {
            let (out, icon, bundle_id, display_name) = bundle_flags(rest)?;
            Ok(Command::Bundle {
                out,
                icon,
                bundle_id,
                display_name,
            })
        }
        "dmg" => Ok(Command::Dmg {
            out: flags(name, rest, false)?.0,
        }),
        "sbom" => Ok(Command::Sbom {
            out: flags(name, rest, false)?.0,
        }),
        "install-smoke" => Ok(Command::InstallSmoke {
            out: flags(name, rest, false)?.0,
        }),
        "notarize" => parse_notarize(name, rest),
        "bench" => crate::bench::parse(rest).map(|args| Command::Bench { args }),
        other => Err(UsageError::UnknownCommand(other.to_string())),
    }
}

fn parse_notarize(name: &str, rest: &[String]) -> Result<Command, UsageError> {
    let mut path = None;
    let mut execute = false;
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--execute" if !execute => execute = true,
            "--path" if path.is_none() => {
                path = Some(PathBuf::from(
                    iter.next()
                        .filter(|v| !v.starts_with("--"))
                        .ok_or_else(|| UsageError::MissingValue { flag: flag.clone() })?,
                ));
            }
            "--execute" | "--path" => return Err(UsageError::DuplicateFlag { flag: flag.clone() }),
            f if f.starts_with("--") => {
                return Err(UsageError::UnknownFlag {
                    command: name.into(),
                    flag: flag.clone(),
                });
            }
            _ => {
                return Err(UsageError::UnexpectedArgument {
                    command: name.into(),
                    argument: flag.clone(),
                });
            }
        }
    }
    Ok(Command::Notarize {
        path: path.ok_or_else(|| UsageError::MissingArgument {
            command: name.into(),
            argument: "--path APP.app|FILE.dmg".into(),
        })?,
        execute,
    })
}

fn bundle_flags(
    rest: &[String],
) -> Result<
    (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<String>,
        Option<String>,
    ),
    UsageError,
> {
    let mut out = None;
    let mut icon = None;
    let mut bundle_id = None;
    let mut display_name = None;
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        if !flag.starts_with("--") {
            return Err(UsageError::UnexpectedArgument {
                command: "bundle".into(),
                argument: flag.clone(),
            });
        }
        if !["--out", "--icon", "--bundle-id", "--display-name"].contains(&flag.as_str()) {
            return Err(UsageError::UnknownFlag {
                command: "bundle".into(),
                flag: flag.clone(),
            });
        }
        let value = iter
            .next()
            .filter(|v| !v.starts_with("--"))
            .ok_or_else(|| UsageError::MissingValue { flag: flag.clone() })?;
        let slot = match flag.as_str() {
            "--out" => &mut out,
            "--icon" => &mut icon,
            "--bundle-id" => &mut bundle_id,
            "--display-name" => &mut display_name,
            f if f.starts_with("--") => {
                return Err(UsageError::UnknownFlag {
                    command: "bundle".into(),
                    flag: flag.clone(),
                });
            }
            _ => {
                return Err(UsageError::UnexpectedArgument {
                    command: "bundle".into(),
                    argument: flag.clone(),
                });
            }
        };
        if slot.replace(value.clone()).is_some() {
            return Err(UsageError::DuplicateFlag { flag: flag.clone() });
        }
    }
    Ok((
        out.map(PathBuf::from),
        icon.map(PathBuf::from),
        bundle_id,
        display_name,
    ))
}

type OutIcon = (Option<PathBuf>, Option<PathBuf>);

fn flags(command: &str, rest: &[String], allow_icon: bool) -> Result<OutIcon, UsageError> {
    let (mut out, mut icon) = (None, None);
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        let slot = match flag.as_str() {
            "--out" => &mut out,
            "--icon" if allow_icon => &mut icon,
            f if f.starts_with("--") => {
                return Err(UsageError::UnknownFlag {
                    command: command.to_string(),
                    flag: flag.clone(),
                });
            }
            _ => {
                return Err(UsageError::UnexpectedArgument {
                    command: command.to_string(),
                    argument: flag.clone(),
                });
            }
        };
        let value = iter
            .next()
            .filter(|v| !v.starts_with("--"))
            .ok_or_else(|| UsageError::MissingValue { flag: flag.clone() })?;
        if slot.replace(PathBuf::from(value)).is_some() {
            return Err(UsageError::DuplicateFlag { flag: flag.clone() });
        }
    }
    Ok((out, icon))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, UsageError> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn existing_commands_still_parse() {
        assert_eq!(parse_strs(&["info"]), Ok(Command::Info));
        assert_eq!(parse_strs(&["build"]), Ok(Command::Build));
        assert_eq!(parse_strs(&["check"]), Ok(Command::Check));
    }

    #[test]
    fn check_ci_requires_exactly_one_workflow_path() {
        assert_eq!(
            parse_strs(&["check-ci"]),
            Err(UsageError::MissingArgument {
                command: "check-ci".into(),
                argument: "WORKFLOW.yml".into(),
            })
        );
        assert_eq!(
            parse_strs(&["check-ci", "one.yml", "two.yml"]),
            Err(UsageError::UnexpectedArgument {
                command: "check-ci".into(),
                argument: "two.yml".into(),
            })
        );
        assert_eq!(
            parse_strs(&["check-ci", "workflow.yml"]),
            Ok(Command::CheckCi {
                workflow: "workflow.yml".into()
            })
        );
    }

    #[test]
    fn bundle_accepts_out_and_icon_in_any_order() {
        let expected = Command::Bundle {
            out: Some("o".into()),
            icon: Some("i.icns".into()),
            bundle_id: None,
            display_name: None,
        };
        assert_eq!(
            parse_strs(&["bundle", "--out", "o", "--icon", "i.icns"]),
            Ok(expected.clone())
        );
        assert_eq!(
            parse_strs(&["bundle", "--icon", "i.icns", "--out", "o"]),
            Ok(expected)
        );
        assert_eq!(
            parse_strs(&["bundle"]),
            Ok(Command::Bundle {
                out: None,
                icon: None,
                bundle_id: None,
                display_name: None
            })
        );
        assert_eq!(
            parse_strs(&[
                "bundle",
                "--bundle-id",
                "io.vexel.test",
                "--display-name",
                "Vexel Test"
            ]),
            Ok(Command::Bundle {
                out: None,
                icon: None,
                bundle_id: Some("io.vexel.test".into()),
                display_name: Some("Vexel Test".into())
            })
        );
        assert_eq!(
            parse_strs(&["notarize", "--path", "x.dmg"]),
            Ok(Command::Notarize {
                path: "x.dmg".into(),
                execute: false
            })
        );
    }

    #[test]
    fn release_commands_take_only_out() {
        for name in ["dmg", "sbom", "install-smoke"] {
            assert!(parse_strs(&[name, "--out", "d"]).is_ok(), "{name}");
            assert_eq!(
                parse_strs(&[name, "--icon", "x.icns"]),
                Err(UsageError::UnknownFlag {
                    command: name.to_string(),
                    flag: "--icon".to_string()
                })
            );
        }
    }

    #[test]
    fn argument_errors_are_typed_and_readable() {
        assert_eq!(parse_strs(&[]), Err(UsageError::NoCommand));
        assert_eq!(
            parse_strs(&["ship"]),
            Err(UsageError::UnknownCommand("ship".into()))
        );
        assert_eq!(
            parse_strs(&["bundle", "--out"]),
            Err(UsageError::MissingValue {
                flag: "--out".into()
            })
        );
        assert_eq!(
            parse_strs(&["bundle", "--out", "--icon", "x"]),
            Err(UsageError::MissingValue {
                flag: "--out".into()
            })
        );
        assert_eq!(
            parse_strs(&["bundle", "--out", "a", "--out", "b"]),
            Err(UsageError::DuplicateFlag {
                flag: "--out".into()
            })
        );
        assert_eq!(
            parse_strs(&["bundle", "stray"]),
            Err(UsageError::UnexpectedArgument {
                command: "bundle".into(),
                argument: "stray".into()
            })
        );
        assert_eq!(
            parse_strs(&["info", "x"]),
            Err(UsageError::UnexpectedArgument {
                command: "info".into(),
                argument: "x".into()
            })
        );
        assert_eq!(
            parse_strs(&["bundle", "--bogus", "1"])
                .unwrap_err()
                .to_string(),
            "`bundle` does not accept flag `--bogus`"
        );
    }
}
