//! Explicit production/fixture dispatch. Production paint never enters the legacy cache.
use vexel_scene::*;
use vexel_xml::XmlSpan;
pub(crate) enum Operations<'a> {
    Fixture(&'a mut dyn DecodeOperations),
    Production(&'a mut dyn ProductionDecodeOperations),
}
impl Operations<'_> {
    pub fn publish_paint(
        &mut self,
        ctx: &mut ResolveContext<'_, '_>,
        builder: &mut SceneBuilder,
        scene: SceneId,
        value: XmlSpan,
        edges: &[Edge],
        inverse: Affine,
        viewport: [f64; 2],
        channel: PaintChannel,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        match self {
            Self::Production(ops) => resolve_publish_cached(
                *ops, ctx.shared, ctx.ledger, builder, scene, value, edges, inverse, viewport,
                channel, control,
            ),
            Self::Fixture(ops) => {
                let value = ops.resolve_paint(ctx, value, edges, inverse, viewport, control)?;
                ctx.validate_cache(control)?;
                match channel {
                    PaintChannel::Fill => builder.append_resolved_fill(scene, &value, control),
                    PaintChannel::Stroke => builder.append_resolved_stroke(scene, &value, control),
                }
            }
        }
    }
    pub fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        match self {
            Self::Production(ops) => {
                MediaDecodeOperations::decode_text(*ops, request, builder, control)
            }
            Self::Fixture(ops) => ops.decode_text(request, builder, control),
        }
    }
    pub fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        match self {
            Self::Production(ops) => {
                MediaDecodeOperations::decode_image(*ops, request, builder, control)
            }
            Self::Fixture(ops) => ops.decode_image(request, builder, control),
        }
    }
}
