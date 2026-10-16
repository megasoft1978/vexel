//! Command-line interpretation: `--mcp-preview` keeps its meaning, a single
//! positional path opens that SVG in the shell, and `--grant-dir <path>`
//! (repeatable) grants the MCP file tools access to a directory. Unknown `-`
//! arguments are ignored exactly as before this module existed.

use crate::session::McpGrants;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// MCP preview with no explicit grants (environment and config only).
    McpPreview,
    /// MCP preview with at least one `--grant-dir`.
    McpPreviewGranted(McpGrants),
    Shell {
        open: Option<PathBuf>,
    },
    /// More than one file was given, or a `--grant-dir` was refused.
    Usage(&'static str),
}

pub fn parse_launch(args: impl IntoIterator<Item = OsString>) -> Launch {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    parse_launch_with_home(args, home.as_deref())
}

/// [`parse_launch`] with an explicit home directory (`--grant-dir` may not
/// name it), so tests do not depend on the process environment.
pub fn parse_launch_with_home(
    args: impl IntoIterator<Item = OsString>,
    home: Option<&Path>,
) -> Launch {
    let mut open: Option<PathBuf> = None;
    let mut literal = false;
    let mut extra = false;
    let mut mcp = false;
    let mut grants = McpGrants::none();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--mcp-preview" {
            mcp = true;
        } else if !literal && arg == "--grant-dir" {
            let Some(path) = args.next() else {
                return Launch::Usage("usage: --grant-dir <directory>");
            };
            if let Err(error) = grants.add(Path::new(&path), home) {
                return Launch::Usage(error.message());
            }
        } else if !literal && arg == "--" {
            literal = true;
        } else if !literal && arg.as_bytes().first() == Some(&b'-') {
            // Unknown options (for example launcher-injected flags) are ignored.
        } else if open.is_some() {
            extra = true;
        } else {
            open = Some(PathBuf::from(arg));
        }
    }
    if mcp {
        if grants.dirs().is_empty() {
            Launch::McpPreview
        } else {
            Launch::McpPreviewGranted(grants)
        }
    } else if extra {
        Launch::Usage("usage: vexel [file.svg]")
    } else {
        Launch::Shell { open }
    }
}
