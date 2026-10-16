//! Complete computed plan handoff. Mapping is carried at suspension, never
//! reconstructed by parsing source on resume. Original descriptors stay intact.
use crate::{
    types::{invalid, stopped},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternMapping {
    pub tile_origin: [f64; 2],
    pub user_to_base: Affine,
    pub pattern_transform: Affine,
    pub user_inverse: Affine,
    pub viewport: [f64; 2],
    pub bounds: [f64; 4],
    pub source_map: SourceMap,
    pub opacity: f64,
}
impl PatternMapping {
    pub fn pixel_inverse(self, descriptor: PatternDescriptor) -> Result<Affine> {
        use vexel_geometry::{affine_compose, affine_valid};
        if !self.tile_origin.into_iter().all(f64::is_finite)
            || !self.viewport.into_iter().all(|v| v.is_finite() && v > 0.)
            || !self.bounds.into_iter().all(f64::is_finite)
            || self.bounds[2] < 0.
            || self.bounds[3] < 0.
            || !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
            || !affine_valid(self.user_inverse)
            || !affine_valid(self.source_map.transform)
        {
            return Err(invalid(descriptor.error_offset));
        }
        // The supplied computed inverse must implement inverse(patternTransform)
        // followed by translation(-tileOrigin), as in the original algorithm.
        let back = affine_compose(self.user_to_base, self.pattern_transform)
            .ok_or_else(|| invalid(descriptor.error_offset))?;
        let expected = [1., 0., 0., 1., -self.tile_origin[0], -self.tile_origin[1]];
        for (a, b) in [back.a, back.b, back.c, back.d, back.e, back.f]
            .into_iter()
            .zip(expected)
        {
            if (a - b).abs() > 1e-9 * a.abs().max(b.abs()).max(1.) {
                return Err(invalid(descriptor.error_offset));
            }
        }
        let scale = Affine {
            a: descriptor.width as f64 / descriptor.tile_width,
            d: descriptor.height as f64 / descriptor.tile_height,
            b: 0.,
            c: 0.,
            e: 0.,
            f: 0.,
        };
        affine_compose(scale, self.user_to_base).ok_or_else(|| invalid(descriptor.error_offset))
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct PatternContinuation {
    mapping: PatternMapping,
    parent: SceneId,
    shape: ShapeId,
    channel: PaintChannel,
    draft: u64,
    generation: u64,
    request: Option<RequestToken>,
    job: LedgerIdentity,
}
impl PatternTickets {
    pub fn request_mapped<'a>(
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
    ) -> Result<PatternRequest<'a>> {
        let i = builder.draft_index(draft)?;
        builder.draft_slot(draft, 3, shape.index(), builder.scenes[i].shapes.len())?;
        job.validate_source(builder.source_owner())?;
        if !std::sync::Arc::ptr_eq(builder.source_owner(), source.source_owner()) {
            return Err(invalid(0));
        }
        let definition = source.element(
            descriptor.definition,
            crate::memory::add(descriptor.definition.0, 1)?,
            control,
        )?;
        if definition.tokens()[descriptor.definition.0]
            .local_name
            .slice(source.source())
            != Some(b"pattern")
        {
            return Err(invalid(descriptor.error_offset));
        }
        let value = builder
            .scene(draft.mark.scene)?
            .shape(shape)
            .copied()
            .ok_or_else(|| invalid(0))?;
        if value.source != mapping.source_map
            || mapping
                .source_map
                .element
                .slice(builder.source_owner())
                .is_none()
        {
            return Err(invalid(0));
        }
        mapping.pixel_inverse(descriptor)?;
        let child_view = builder.scene(child)?;
        if child == draft.mark.scene || child.index() < draft.scene_count {
            return Err(invalid(0));
        }
        let request = self.request(child_view, descriptor, control)?;
        self.pending.as_mut().ok_or_else(|| invalid(0))?.production = Some(PatternContinuation {
            mapping,
            parent: draft.mark.scene,
            shape,
            channel,
            draft: draft.nonce,
            generation: builder.generation,
            request: builder.request_identity(),
            job: job.owner_identity(),
        });
        Ok(request)
    }
    /// Transfer admitted job output directly into the builder's owned pattern;
    /// then publish and attach the exact fill/stroke mapping. No raw Vec or Tile
    /// escapes this operation. Late errors unwind just this publication suffix.
    pub fn complete_mapped(
        &mut self,
        ticket: PatternTicket,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<(PatternId, usize)> {
        let pending = self.pending.as_ref().ok_or_else(|| invalid(0))?;
        if pending.ticket != ticket {
            return Err(invalid(0));
        }
        let p = pending.production.ok_or_else(|| invalid(0))?;
        let descriptor = pending.descriptor;
        job.validate_source(builder.source_owner())?;
        if builder.owner != p.parent.owner || job.owner_identity() != p.job {
            return Err(invalid(0));
        }
        let child = pending.scene;
        let live = builder.draft_index(draft).is_ok()
            && draft.nonce == p.draft
            && draft.mark.scene == p.parent
            && builder.generation == p.generation
            && builder.request_identity() == p.request
            && builder
                .scene(child)
                .is_ok_and(|view| view.scene(child).is_some())
            && builder
                .scene(p.parent)
                .is_ok_and(|view| view.shape(p.shape).is_some());
        if !live {
            self.pending = None;
            if let Ok(output) = result {
                if output.validate(job).is_ok() {
                    job.release(output.pixels)?;
                }
            }
            return Err(invalid(0));
        }
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                self.pending = None;
                return Err(error);
            }
        };
        output.validate(job)?;
        if output.width != descriptor.width
            || output.height != descriptor.height
            || output.generation != p.generation
        {
            return Err(invalid(0));
        }
        let inverse = p.mapping.pixel_inverse(descriptor)?;
        let mut shape = *builder
            .scene(p.parent)?
            .shape(p.shape)
            .ok_or_else(|| invalid(0))?;
        let i = builder.draft_index(draft)?;
        builder.draft_slot(draft, 3, p.shape.index(), builder.scenes[i].shapes.len())?;
        let mut mark = builder.checkpoint_internal(p.parent)?;
        let result = (|| {
            control.checkpoint().map_err(stopped)?;
            let id = job.publish_pattern_output(output, builder, p.parent, descriptor, control)?;
            let paint = Paint {
                kind: PaintKind::Pattern,
                spread: Spread::Repeat,
                object_bounding_box: false,
                coordinates: [
                    p.mapping.tile_origin[0],
                    p.mapping.tile_origin[1],
                    descriptor.tile_width,
                    descriptor.tile_height,
                    0.,
                    0.,
                ],
                bounds: p.mapping.bounds,
                user_inverse: p.mapping.user_inverse,
                gradient_inverse: vexel_geometry::affine_identity(),
                stops: TableRange::EMPTY,
                pattern: Some(id),
                pattern_inverse: inverse,
                pattern_opacity: p.mapping.opacity,
            };
            let index = match p.channel {
                PaintChannel::Fill => builder.append_fill_paint(p.parent, paint, control)?,
                PaintChannel::Stroke => builder.append_stroke_paint(p.parent, paint, control)?,
            };
            match p.channel {
                PaintChannel::Fill => shape.fill_paint = Some(index),
                PaintChannel::Stroke => shape.stroke_paint = Some(index),
            }
            builder.finalize_shape(draft, p.shape, shape, control)?;
            Ok((id, index))
        })();
        if result.is_err() {
            mark.epoch = builder.scenes[i].epoch;
            builder.rollback_internal(mark)?;
            // Output may not yet have transferred. Clean it unconditionally.
            if job.slice(output.pixels).is_ok() {
                job.release(output.pixels)?;
            }
        }
        self.pending = None;
        result
    }
}
