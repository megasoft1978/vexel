//! Actual decoder-owned draft authorization passed by disjoint borrow to media.
//! Compatibility operations remain unchanged; production implements this complete
//! combined surface explicitly, with no global token or reconstructed authority.
use crate::*;

pub trait ScopedMediaDecodeOperations {
    fn decode_text(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn decode_image(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId>;
}
pub trait ScopedProductionDecodeOperations:
    CachedPaintResolver
    + DefinitionPlanner
    + PaintChannelDefinitionPlanner
    + ScopedMediaDecodeOperations
{
}

/// New event type keeps old exhaustive DecodeEvent matches source compatible.
pub enum ScopedDecodeEvent<'a> {
    Progress,
    Pattern(StaticPatternRequest<'a>),
    Complete,
}
pub trait ScopedProductionDecodeSession {
    fn step<'s>(
        &'s mut self,
        quantum: u64,
        job: &RasterJob,
        operations: &mut dyn ScopedProductionDecodeOperations,
        control: &mut dyn WorkControl,
    ) -> Result<ScopedDecodeEvent<'s>>;
    fn resume_pattern(
        &mut self,
        ticket: PatternTicket,
        job: &mut RasterJob,
        result: Result<JobRenderOutput>,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn finish(self, control: &mut dyn WorkControl) -> Result<SceneGraph>
    where
        Self: Sized;
}

/// Validate the actual guarded token and source before entering the lower media
/// adapter. The adapter receives this exact borrow for begin/commit/rollback and
/// cumulative text-command admission. It owns request cleanup on late failure.
pub fn decode_scoped_text(
    operations: &mut dyn ScopedProductionDecodeOperations,
    request: TextDecodeRequest<'_, '_>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    builder.validate_media_draft(draft, request.scene)?;
    request.validate_source_map(builder, control)?;
    operations.decode_text(request, builder, draft, control)
}
pub fn decode_scoped_image(
    operations: &mut dyn ScopedProductionDecodeOperations,
    request: ImageDecodeRequest<'_, '_>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    builder.validate_media_draft(draft, request.scene)?;
    request.validate_source_map(builder, control)?;
    operations.decode_image(request, builder, draft, control)
}
impl SceneBuilder {
    fn validate_media_draft(&self, draft: &DraftToken, scene: SceneId) -> Result<()> {
        self.draft_index(draft)?;
        if draft.mark.scene != scene {
            return Err(crate::types::invalid(0));
        }
        Ok(())
    }
}
