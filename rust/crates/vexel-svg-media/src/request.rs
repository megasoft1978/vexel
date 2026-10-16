use vexel_base::WorkControl;
use vexel_scene::{DraftToken, RequestToken, Result, SceneBuilder};

/// Keep request cleanup independent of cancellation. A failed commit still
/// owns its request token, so scene validation failures follow the same path.
pub(crate) fn atomic_request<T>(
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
    operation: impl FnOnce(&mut SceneBuilder, &mut dyn WorkControl, RequestToken) -> Result<T>,
) -> Result<T> {
    let token = builder.begin_request(draft, control)?;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        operation(builder, control, token).and_then(|value| {
            builder.commit_request(draft, token, control)?;
            Ok(value)
        })
    }));
    let result = match outcome {
        Ok(result) => result,
        Err(panic) => {
            // No cancellation polling during recovery, including abandoned leases.
            let _ = builder.rollback_request(draft, token);
            std::panic::resume_unwind(panic);
        }
    };
    if result.is_err() {
        builder.rollback_request(draft, token)?;
    }
    result
}
