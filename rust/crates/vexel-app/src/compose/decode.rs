//! Bytes to `SceneGraph` through the real resumable decoder, with the text
//! layout provider injected and pattern tiles rendered on `NeedPattern`.

use super::error::{ComposeError, from_diagnostic};
use super::render_scene::render_scene;
use std::sync::Arc;
use vexel_raster::Tile;
use vexel_render::{PreserveAspect, RenderError, ViewBox};
use vexel_scene::{
    ChargeLedger, DecodeOptions, DecodeSource, DecodedScene, Diagnostic, ImagePixels,
    JobRenderOutput, LexicalDocument, Limits, MediaError, MediaProvider, RasterJob, RasterPool,
    SceneGraph, Stop, TextLayout, TextRequest, WorkControl, WorkKind,
};
use vexel_svg_decode::TraversalDecodeEvent;
use vexel_svg_engine::SvgEngineSession;

/// Largest source accepted by the compose pipeline: one cap shared by
/// `open_path` and `decode`. Larger sources are refused (`TooLarge` /
/// `LimitDecode`); streaming decode of bigger files is a separate work order.
pub const MAX_SVG_BYTES: usize = 32 * 1024 * 1024;

/// Token-storage budget for the lexical pass (the scene crate clamps it to
/// 256 MiB). Measured: token storage runs about 4 bytes per source byte on
/// the large corpus files, so 8x the cap covers every accepted source; the
/// former 16 MiB limit refused such files at ~4 MiB of input.
const TOKEN_STORAGE_LIMIT: usize = 8 * MAX_SVG_BYTES;

/// Charge-ledger reservation (source copy plus decode work), twice the cap.
const LEDGER_RESERVE: usize = 2 * MAX_SVG_BYTES;

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

/// Present only so the decoder reaches the engine's own providers: text goes
/// through the engine's `TextLayoutProvider`; data-URI PNG images decode here
/// (JPEG and interlaced PNG stay unsupported).
pub(crate) struct Gate;

impl MediaProvider for Gate {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn decode_image(
        &mut self,
        encoded: &[u8],
        control: &mut dyn WorkControl,
    ) -> Result<ImagePixels, MediaError> {
        control.checkpoint().map_err(|_| MediaError::Limit)?;
        // The PNG decoder yields straight alpha; `ImagePixels` is premultiplied.
        let mut pixels = super::png::decode(encoded)?;
        for px in pixels.rgba.chunks_exact_mut(4) {
            let a = u32::from(px[3]);
            if a != 255 {
                for c in &mut px[..3] {
                    *c = ((u32::from(*c) * a + 127) / 255) as u8;
                }
            }
        }
        Ok(pixels)
    }
}

/// Why a decode session stopped.
pub(crate) enum SessionError {
    Decode(Diagnostic),
    Render(RenderError),
}

impl From<Diagnostic> for SessionError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Decode(diagnostic)
    }
}

impl From<RenderError> for SessionError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

/// Installs the CoreText layout provider (macOS); elsewhere text stays unsupported.
pub(crate) fn install_text(session: &mut SvgEngineSession<'_, '_>) {
    #[cfg(target_os = "macos")]
    session.set_text_provider(Box::new(super::text::CoreText));
    #[cfg(not(target_os = "macos"))]
    let _ = session;
}

/// Steps `session` to completion. A `NeedPattern` suspension renders the
/// pattern tile with the scene renderer and resumes the decoder with it.
pub(crate) fn run(
    session: &mut SvgEngineSession<'_, '_>,
    job: &mut RasterJob,
    quantum: u64,
    generation: u64,
    control: &mut dyn WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<(), SessionError> {
    loop {
        let (ticket, width, height, pixels) = match session.step(quantum, job, control)? {
            TraversalDecodeEvent::Progress { .. } => continue,
            TraversalDecodeEvent::Complete => return Ok(()),
            TraversalDecodeEvent::NeedPattern(request) => {
                let mut tile = blank(request.width, request.height)?;
                let view_box = ViewBox {
                    x: 0.0,
                    y: 0.0,
                    width: request.tile_width,
                    height: request.tile_height,
                };
                let aspect = PreserveAspect {
                    align_x: 0.0,
                    align_y: 0.0,
                    none: true,
                    slice: false,
                };
                render_scene(
                    &request.child,
                    view_box,
                    Some(aspect),
                    &mut tile,
                    true,
                    cancel,
                )?;
                (request.ticket, request.width, request.height, tile)
            }
        };
        let buffer = job.buffer::<u8>(RasterPool::Output, pixels.byte_count(), control)?;
        job.reserve(buffer, pixels.byte_count(), control)?;
        job.extend(buffer, pixels.rgba(), control)?;
        let output = JobRenderOutput {
            generation,
            pixels: buffer,
            width,
            height,
            stride: width as usize * 4,
        };
        session.resume_pattern(ticket, job, Ok(output), control)?;
    }
}

fn blank(width: u32, height: u32) -> Result<Tile, RenderError> {
    let bytes = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::LimitExceeded)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes)
        .map_err(|_| RenderError::LimitExceeded)?;
    rgba.resize(bytes, 0);
    Tile::from_owned_rgba(width, height, rgba).map_err(|_| RenderError::LimitExceeded)
}

/// Decodes an already-owned source buffer without copying its bytes.
pub(crate) fn decode_owned(
    owner: Arc<Vec<u8>>,
    cancel: &dyn Fn() -> bool,
) -> Result<SceneGraph, ComposeError> {
    let bytes = owner.as_slice();
    if bytes.len() > MAX_SVG_BYTES {
        return Err(ComposeError::LimitDecode);
    }
    if let Err(error) = std::str::from_utf8(bytes) {
        return Err(ComposeError::InvalidUtf8 {
            valid_up_to: error.valid_up_to(),
        });
    }
    // Entity references in attribute values are expanded up front (XML 1.0
    // 3.3.3). A refused document keeps its source so the lexical pass reports
    // the typed error.
    let owner = match vexel_xml::expand_attribute_entities(bytes) {
        Ok(Some(expanded)) if expanded.len() <= MAX_SVG_BYTES => Arc::new(expanded),
        _ => owner,
    };
    let bytes = owner.as_slice();
    let mut control = CancelControl { cancel };
    let diag = |d| from_diagnostic(d, bytes);
    let document =
        LexicalDocument::parse(owner.clone(), TOKEN_STORAGE_LIMIT, &mut control).map_err(diag)?;
    let mut source = DecodeSource::new(&document, &mut control).map_err(diag)?;
    let ledger = ChargeLedger::new(&owner, LEDGER_RESERVE, 65536).map_err(diag)?;
    let mut job = RasterJob::new(ledger).map_err(|_| ComposeError::LimitDecode)?;
    let mut gate = Gate;
    let options = DecodeOptions {
        limits: Limits {
            source_bytes: MAX_SVG_BYTES,
            ..Limits::default()
        },
        system_language: b"en",
        media: Some(&mut gate),
    };
    let mut session =
        SvgEngineSession::from_source(&document, &mut source, 1, options, &mut control)
            .map_err(diag)?;
    install_text(&mut session);
    run(&mut session, &mut job, 64, 1, &mut control, cancel).map_err(|error| match error {
        SessionError::Decode(d) => from_diagnostic(d, bytes),
        SessionError::Render(e) => e.into(),
    })?;
    match session.finish(&mut control).map_err(diag)? {
        DecodedScene::Drawable(graph) => Ok(graph),
        DecodedScene::Hidden(_) => Err(ComposeError::Hidden),
    }
}
