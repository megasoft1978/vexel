//! Required production media callbacks borrow the actual active draft. The engine
//! opens and closes the request; adapters cannot fabricate a draft or leave a
//! successful callback with an uncommitted request. Legacy delegates stay explicit.
use vexel_scene::*;

pub trait DecoderProductionOperations: ChannelAwareProductionDecodeOperations {
    fn decode_text_scoped(
        &mut self,
        request: TextDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<bool>;
    fn decode_image_scoped(
        &mut self,
        request: ImageDecodeRequest<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId>;
}

/// Rollback is deliberately not controlled: cancellation cannot skip cleanup.
/// Failed commit retains the active request, so the same cleanup handles both
/// adapter failure and publication validation failure.
pub(crate) fn complete<T>(
    result: Result<T>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    request: RequestToken,
    control: &mut dyn WorkControl,
) -> Result<T> {
    let result = result.and_then(|value| {
        builder.commit_request(draft, request, control)?;
        Ok(value)
    });
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            builder.rollback_request(draft, request)?;
            Err(error)
        }
    }
}
