//! Thin command entry point for native timeline actions.

use crate::native_timeline::NativeTimelineAction;
use crate::session::AppDocument;

use super::{NativeActionOutcome, NativeAnimationController, NativeAnimationError};

impl NativeAnimationController {
    /// Apply one immutable panel intent through the document journal boundary.
    pub fn handle_action(
        &mut self,
        document: &mut AppDocument,
        action: NativeTimelineAction,
    ) -> Result<NativeActionOutcome, NativeAnimationError> {
        self.dispatch(document, action)
    }
}
