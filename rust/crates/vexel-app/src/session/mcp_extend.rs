//! Native tracks need no prior `native_timeline_set_duration`: a key past the
//! current duration lengthens the timeline inside the same edit, up to the
//! limits `set_duration` enforces (14 400 000 ticks, 3600 seconds).

use crate::project_session::{ProjectTimelineCandidate, ProjectTimelineError};

const MAX_TICKS: u64 = 14_400_000;
const MAX_SECONDS: u64 = 3600;

/// The candidate lengthened to hold `max_tick`, or `None` when it already does
/// or when the needed length is past the limits (the edit then fails as before).
pub(super) fn grown(
    candidate: &ProjectTimelineCandidate,
    max_tick: Option<u32>,
) -> Result<Option<ProjectTimelineCandidate>, ProjectTimelineError> {
    let Some(max_tick) = max_tick else {
        return Ok(None);
    };
    let (duration, tps) = {
        let view = candidate.view()?;
        (view.duration_ticks(), view.ticks_per_second())
    };
    let needed = u64::from(max_tick) + 1;
    if u64::from(duration) >= needed || needed > MAX_TICKS || needed > u64::from(tps) * MAX_SECONDS
    {
        return Ok(None);
    }
    candidate.set_timing(needed as u32, None).map(Some)
}
