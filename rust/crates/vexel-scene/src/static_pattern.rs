//! Request-only static tile role. No animation is constructed or evaluated.
use crate::{
    types::{invalid, stopped},
    *,
};

/// Immutable sealed child under a live mapped continuation. Private backing
/// tables prevent arbitrary scene lookup and any conversion to a ready frame.
/// ```compile_fail
/// use vexel_scene::*;
/// fn ready(v: StaticPatternView<'_>) { FrameBuilder::new(v, 1024).unwrap(); }
/// ```
/// ```compile_fail
/// use vexel_scene::*;
/// fn parent(v: StaticPatternView<'_>, parent: SceneId) {
///     v.input().scene(parent);
/// }
/// ```
/// ```compile_fail
/// use vexel_scene::*;
/// fn animation(v: StaticPatternView<'_>) { v.input().animation(); }
/// ```
/// ```compile_fail,E0502
/// use vexel_scene::*;
/// fn mutate(b: &mut SceneBuilder, d: &DraftToken, s: &DecodeSource<'_>,
///     child: SceneId, desc: PatternDescriptor, map: PatternMapping, shape: ShapeId,
///     t: &mut PatternTickets, j: &RasterJob, c: &mut dyn WorkControl) {
///     let request = t.request_mapped_static(b, d, s, child, desc, map, shape,
///         PaintChannel::Fill, j, c).unwrap();
///     b.abort_request(d).unwrap();
///     request.child().input().shapes();
/// }
/// ```
/// ```compile_fail,E0505
/// use vexel_scene::*;
/// fn freeze(b: SceneBuilder, d: &DraftToken, s: &DecodeSource<'_>,
///     child: SceneId, desc: PatternDescriptor, map: PatternMapping, shape: ShapeId,
///     t: &mut PatternTickets, j: &RasterJob, c: &mut dyn WorkControl) {
///     let request = t.request_mapped_static(&b, d, s, child, desc, map, shape,
///         PaintChannel::Fill, j, c).unwrap();
///     b.freeze().unwrap();
///     request.child().input().shapes();
/// }
/// ```
#[derive(Clone, Copy)]
pub struct StaticPatternView<'a> {
    pub(crate) input: RasterInput<'a>,
}
impl<'a> StaticPatternView<'a> {
    pub fn input(self) -> RasterInput<'a> {
        self.input
    }
}
#[derive(Clone, Copy)]
pub(crate) struct StaticScope {
    pub generation: u64,
    pub job: LedgerIdentity,
    pub draft: u64,
    pub request: RequestToken,
    pub ticket: PatternTicket,
}
pub struct StaticPatternRequest<'a> {
    pub(crate) ticket: PatternTicket,
    pub(crate) child: StaticPatternView<'a>,
    pub(crate) descriptor: PatternDescriptor,
}
impl<'a> StaticPatternRequest<'a> {
    pub fn ticket(&self) -> PatternTicket {
        self.ticket
    }
    pub fn child(&self) -> StaticPatternView<'a> {
        self.child
    }
    pub fn descriptor(&self) -> PatternDescriptor {
        self.descriptor
    }
}
impl PatternTickets {
    /// Borrows the builder until the tile render ends. The caller then drops the
    /// request and publishes same-job output through `complete_mapped`.
    pub fn request_mapped_static<'a>(
        &mut self,
        builder: &'a SceneBuilder,
        draft: &DraftToken,
        source: &DecodeSource<'_>,
        child: SceneId,
        descriptor: PatternDescriptor,
        mapping: PatternMapping,
        shape: ShapeId,
        channel: PaintChannel,
        job: &RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<StaticPatternRequest<'a>> {
        builder.draft_index(draft)?;
        let request_identity = builder.static_request_child(child)?;
        let view = builder.traversal_scene(child)?;
        let viewport = view
            .viewport
            .ok_or_else(|| invalid(descriptor.error_offset))?;
        crate::validate::viewport(viewport)?;
        // Original C's tile viewport: raster dimensions, origin zero, tile
        // extent and preserveAspectRatio=none. No fabricated parent viewport.
        if viewport.width != descriptor.width
            || viewport.height != descriptor.height
            || viewport.view_x != 0.
            || viewport.view_y != 0.
            || viewport.view_width != descriptor.tile_width
            || viewport.view_height != descriptor.tile_height
            || !viewport.aspect.none
        {
            return Err(invalid(descriptor.error_offset));
        }
        let tables = view.raster_tables(viewport);
        validate_mask_closure(tables, &mut [None; 64], 0, &mut 0, control)?;
        let request = self.request_mapped_view(
            builder, draft, source, child, view, descriptor, mapping, shape, channel, job, control,
        )?;
        let scope = StaticScope {
            generation: builder.generation(),
            job: job.owner_identity(),
            draft: draft.nonce,
            request: request_identity,
            ticket: request.ticket,
        };
        Ok(StaticPatternRequest {
            ticket: request.ticket,
            descriptor,
            child: StaticPatternView {
                input: RasterInput::static_child(tables, scope),
            },
        })
    }
}
fn validate_mask_closure(
    view: SceneView<'_, Viewport, ()>,
    stack: &mut [Option<SceneId>; 64],
    depth: usize,
    visits: &mut usize,
    control: &mut dyn WorkControl,
) -> Result<()> {
    control.checkpoint().map_err(stopped)?;
    if depth == stack.len() || *visits >= 65536 {
        return Err(crate::types::limit(LimitKind::Masks));
    }
    if stack[..depth].contains(&Some(view.id)) {
        return Err(invalid(0));
    }
    *visits = visits
        .checked_add(1)
        .ok_or_else(crate::memory::arithmetic)?;
    stack[depth] = Some(view.id);
    for mask in view.masks {
        control
            .charge(WorkKind::ReferenceProbe, 1)
            .map_err(stopped)?;
        let child = view
            .scene(mask.content)
            .ok_or_else(|| invalid(mask.source_offset))?;
        crate::validate::viewport(child.viewport)?;
        validate_mask_closure(child, stack, depth + 1, visits, control)?;
    }
    // Also require the initial child sealed (scene lookup checks serial/seal).
    view.scene(view.id).ok_or_else(|| invalid(0))?;
    Ok(())
}
