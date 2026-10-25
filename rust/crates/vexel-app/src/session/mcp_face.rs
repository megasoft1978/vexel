//! Face tools (`mouth_set`, `lip_sync`, `eyes_set`, `blink`): the operation
//! type, its routing (lip-sync reads audio, so it needs the host's grant
//! policy) and the shared writer for generated property tracks.

use super::mcp_mouth::{self, Mouth};
use crate::preview::DocOp;
use crate::preview::native_args::rig::Op;
use crate::session::{AppDocument, McpOutcome};
use vexel_mcp_access::AccessPolicy;

#[derive(Clone, Debug, PartialEq)]
pub enum FaceOp {
    Mouth(Mouth),
}

/// `Operation::Doc` dispatch: face tools get the policy, the rest go on to
/// the plain document families.
pub(crate) fn route(
    app: &mut AppDocument,
    policy: &AccessPolicy,
    op: DocOp,
) -> Result<McpOutcome, &'static str> {
    match op {
        DocOp::Rig(Op::Face(face)) => apply(app, policy, face),
        other => crate::preview::apply_doc(app, other),
    }
}

fn apply(
    app: &mut AppDocument,
    _policy: &AccessPolicy,
    op: FaceOp,
) -> Result<McpOutcome, &'static str> {
    match op {
        FaceOp::Mouth(spec) => mcp_mouth::apply(app, spec),
    }
}
