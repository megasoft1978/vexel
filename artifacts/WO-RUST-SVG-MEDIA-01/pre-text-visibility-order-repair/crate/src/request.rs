use vexel_base::WorkControl;
use vexel_scene::{DraftToken, Result, SceneBuilder};

/// Keep request cleanup independent of cancellation. A failed commit still
/// owns its request token, so scene validation failures follow the same path.
pub(crate) fn atomic_request<T>(
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
    operation: impl FnOnce(&mut SceneBuilder, &mut dyn WorkControl) -> Result<T>,
) -> Result<T> {
    let token = builder.begin_request(draft, control)?;
    let result = operation(builder, control).and_then(|value| {
        builder.commit_request(draft, token, control)?;
        Ok(value)
    });
    if result.is_err() {
        builder.rollback_request(draft, token)?;
    }
    result
}
