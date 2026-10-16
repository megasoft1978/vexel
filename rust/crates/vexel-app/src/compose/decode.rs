//! Bytes to `SceneGraph` through the real resumable decoder.

use super::error::{ComposeError, from_diagnostic};
use std::sync::Arc;
use vexel_scene::{
    ChargeLedger, DecodeOptions, DecodeSource, DecodedScene, LexicalDocument, Limits, RasterJob,
    SceneGraph, Stop, WorkControl, WorkKind,
};
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::SvgEngineSession;

/// Largest source accepted by the compose pipeline (matches the open cap).
pub const MAX_SVG_BYTES: usize = 8 * 1024 * 1024;

/// Work control that stops at the next checkpoint once `cancel` is true.
pub(crate) struct CancelControl<'a> {
    pub cancel: &'a dyn Fn() -> bool,
}

impl WorkControl for CancelControl<'_> {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        self.checkpoint()
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        if (self.cancel)() {
            Err(Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

pub(crate) fn decode(bytes: &[u8], cancel: &dyn Fn() -> bool) -> Result<SceneGraph, ComposeError> {
    if bytes.len() > MAX_SVG_BYTES {
        return Err(ComposeError::LimitExceeded);
    }
    if let Err(error) = std::str::from_utf8(bytes) {
        return Err(ComposeError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        });
    }
    let mut control = CancelControl { cancel };
    let owner = Arc::new(bytes.to_vec());
    let diag = |d| from_diagnostic(d, bytes);
    let document = LexicalDocument::parse(owner.clone(), 1 << 24, &mut control).map_err(diag)?;
    let mut source = DecodeSource::new(&document, &mut control).map_err(diag)?;
    let ledger = ChargeLedger::new(&owner, 1 << 24, 65536).map_err(diag)?;
    let job = RasterJob::new(ledger).map_err(|_| ComposeError::LimitExceeded)?;
    let options = DecodeOptions {
        limits: Limits {
            source_bytes: MAX_SVG_BYTES,
            ..Limits::default()
        },
        system_language: b"en",
        media: None,
    };
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 1, options, &mut control)
            .map_err(diag)?;
    loop {
        match session.step(64, &job, &mut control).map_err(diag)? {
            TraversalDecodeEvent::Progress { .. } => {}
            TraversalDecodeEvent::Complete => break,
            TraversalDecodeEvent::NeedPattern(_) => {
                return Err(ComposeError::Unsupported {
                    feature: "pattern paint",
                    offset: None,
                    element: None,
                });
            }
        }
    }
    match session.finish(&mut control).map_err(diag)? {
        DecodedScene::Drawable(graph) => Ok(graph),
        DecodedScene::Hidden(_) => Err(ComposeError::Hidden),
    }
}
