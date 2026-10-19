//! Explicit recovery decision for a project selected in the Open panel.

use super::FlowOutcome;
use crate::project_session::{ProjectSession, RecoveryStatus, SessionError};
use vexel_platform_apple::{PanelBackend, RecoveryChoice, RecoveryText};

/// Opens a selected file into a temporary session, then adopts it only after
/// the recovery prompt and any requested restore/discard have succeeded.
/// This keeps the current session intact on every error and on Cancel.
pub(super) fn open_selected(
    session: &mut ProjectSession,
    path: &[u8],
    backend: &dyn PanelBackend,
) -> FlowOutcome {
    let mut opened = match ProjectSession::open(path) {
        Ok(opened) => opened,
        Err(error) => return fail(backend, error),
    };

    // SVG imports are untitled and cannot have a recovery capsule.
    if opened.path().is_some() {
        let status = match opened.recovery_status() {
            Ok(status) => status,
            Err(error) => return fail(backend, error),
        };
        if status != RecoveryStatus::Absent {
            let conflict = status == RecoveryStatus::Conflict;
            let text = RecoveryText {
                message: if conflict {
                    "A recovery snapshot exists, but this project has changed."
                } else {
                    "A recovery snapshot is available for this project."
                },
                detail: if conflict {
                    "Restoring it keeps the changed source file intact. The recovered document will need Save As."
                } else {
                    "Choose whether to restore the recovered document or discard the snapshot."
                },
                restore: "Restore Recovery",
                discard: "Discard Recovery",
                cancel: "Cancel",
            };
            match backend.confirm_recovery(&text) {
                RecoveryChoice::Cancel => return FlowOutcome::Cancelled,
                RecoveryChoice::Restore => {
                    if let Err(error) = opened.restore_recovery() {
                        return fail(backend, error);
                    }
                }
                RecoveryChoice::Discard => {
                    if let Err(error) = opened.discard_recovery() {
                        return fail(backend, error);
                    }
                }
            }
        }
    }

    *session = opened;
    FlowOutcome::Opened
}

fn fail(backend: &dyn PanelBackend, error: SessionError) -> FlowOutcome {
    backend.show_error("Could not open", error.user_message());
    FlowOutcome::Failed(error)
}
