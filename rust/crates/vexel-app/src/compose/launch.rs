//! Command-line interpretation: `--mcp-preview` keeps its meaning, a single
//! positional path opens that SVG in the shell. Unknown `-` arguments are
//! ignored exactly as before this module existed.

use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    McpPreview,
    Shell {
        open: Option<PathBuf>,
    },
    /// More than one file was given.
    Usage(&'static str),
}

pub fn parse_launch(args: impl IntoIterator<Item = OsString>) -> Launch {
    let mut open: Option<PathBuf> = None;
    let mut literal = false;
    let mut extra = false;
    for arg in args {
        if arg == "--mcp-preview" {
            return Launch::McpPreview;
        }
        if !literal && arg == "--" {
            literal = true;
        } else if !literal && arg.as_bytes().first() == Some(&b'-') {
            // Unknown options (for example launcher-injected flags) are ignored.
        } else if open.is_some() {
            extra = true;
        } else {
            open = Some(PathBuf::from(arg));
        }
    }
    if extra {
        Launch::Usage("usage: vexel [file.svg]")
    } else {
        Launch::Shell { open }
    }
}
