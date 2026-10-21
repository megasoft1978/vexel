//! Export popover: animated PNG and PNG frame sequence of the native
//! timeline loop. Both go through the MCP `export_animation` apng/frames
//! writers, so size, frame and byte caps and validation are the same.
//!
//! An APNG renders into a private staging directory (the only grant) and is
//! renamed over the chosen file. A sequence grants only the chosen folder,
//! refuses to overwrite existing frames and removes what it wrote on failure.

use crate::animation::{driver::Driver, playback::MonotonicClock};
use crate::preview::{FramesExport, Operation};
use crate::session::{AppDocument, McpGrants, McpHost};
use crate::toolbar_actions::Stage;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use vexel_mcp_access::AccessPolicy;

const PATH_BAD: &str = "Export path is not usable";

fn run(
    doc: &mut AppDocument,
    policy: AccessPolicy,
    path: &[u8],
    apng: bool,
) -> Result<(), &'static str> {
    let path = std::str::from_utf8(path).map_err(|_| PATH_BAD)?.to_owned();
    let request = FramesExport {
        path,
        apng,
        fps: None,
        loops: 0,
        size: None,
        background: None,
    };
    let mut driver = Driver::new(MonotonicClock::new());
    McpHost::with_policy(policy)
        .handle_with(doc, Operation::ExportFrames(Box::new(request)), &mut driver)
        .map(|_| ())
}

/// Writes `target` (an `.apng` path); the status line on success.
pub fn write_apng(doc: &mut AppDocument, target: PathBuf) -> Result<&'static str, &'static str> {
    let stage = Stage::create(&target)?;
    let rendered = stage
        .policy()
        .and_then(|policy| run(doc, policy, stage.file.as_os_str().as_bytes(), true));
    match rendered {
        Ok(()) => stage.publish(&target).map(|()| "Exported APNG"),
        Err(message) => {
            stage.discard();
            Err(message)
        }
    }
}

/// Writes `name_0000.png` ... beside `target`. A leaf holding `####` is used
/// as the pattern; otherwise its `.png` extension is dropped and the leaf is
/// the name prefix.
pub fn write_frames(doc: &mut AppDocument, target: PathBuf) -> Result<&'static str, &'static str> {
    let directory = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or(PATH_BAD)?;
    let leaf = target.file_name().ok_or(PATH_BAD)?.as_bytes();
    let leaf = if leaf.contains(&b'#') {
        leaf
    } else {
        leaf.strip_suffix(b".png")
            .or_else(|| leaf.strip_suffix(b".PNG"))
            .unwrap_or(leaf)
    };
    let mut path = directory.as_os_str().as_bytes().to_vec();
    path.push(b'/');
    path.extend_from_slice(leaf);
    let policy = McpGrants::from_dirs([directory], None)
        .map(|grants| grants.policy_with(None, None))
        .map_err(|_| PATH_BAD)?;
    run(doc, policy, &path, false).map(|()| "Exported PNG frames")
}

#[cfg(test)]
#[path = "frames_export_tests.rs"]
mod tests;
