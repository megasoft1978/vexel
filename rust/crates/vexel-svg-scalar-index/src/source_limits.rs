//! Pure source-size admission, also applied to the actual production owner.
use crate::util::limit;
use vexel_scene::{LimitKind, Result};
/// Original decode source ceiling. Native pointer/length providers may call
/// this before retaining source; ScalarIndex checks its actual SourceOwner len.
pub fn validate_source_length(length: usize) -> Result<()> {
    if length > (1usize << 30) {
        return Err(limit(LimitKind::SourceBytes));
    }
    Ok(())
}
