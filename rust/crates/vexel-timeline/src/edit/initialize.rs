use super::{EditFailure, EditResult, fail};
use crate::{Error, LoopMode, MAX_SOURCE_BYTES, TimelineContext, decode};
use std::fmt::{self, Write as _};

/// Caller supplied project timing and display values for a new empty timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmptyTimelineSettings {
    pub ticks_per_second: u32,
    pub duration_ticks: u32,
    pub fps_numerator: u32,
    pub fps_denominator: u32,
    pub loop_mode: LoopMode,
    pub onion_before: u32,
    pub onion_after: u32,
    pub onion_opacity: f64,
}

/// Builds and validates the complete v1 representation for a new empty timeline.
pub fn initialize_empty_timeline(
    settings: EmptyTimelineSettings,
    context: &TimelineContext,
) -> EditResult<Vec<u8>> {
    if settings.ticks_per_second == 0
        || settings.ticks_per_second > 240
        || settings.duration_ticks == 0
        || settings.duration_ticks > 14_400_000
        || settings.fps_numerator == 0
        || settings.fps_denominator == 0
        || (u64::from(settings.ticks_per_second) * u64::from(settings.fps_denominator))
            % u64::from(settings.fps_numerator)
            != 0
        || settings.onion_before > 8
        || settings.onion_after > 8
        || !settings.onion_opacity.is_finite()
        || !(0.0..=1.0).contains(&settings.onion_opacity)
    {
        return Err(fail(Error::InvalidArgument, 0));
    }

    let mut opacity = StackBuffer {
        bytes: [0; 32],
        len: 0,
    };
    write!(&mut opacity, "{}", settings.onion_opacity)
        .map_err(|_| fail(Error::InvalidArgument, 0))?;
    let loop_mode = match settings.loop_mode {
        LoopMode::Once => b"once".as_slice(),
        LoopMode::Repeat => b"loop",
        LoopMode::PingPong => b"ping_pong",
    };

    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_SOURCE_BYTES.min(256))
        .map_err(|_| fail(Error::Limit, 0))?;
    bytes.extend_from_slice(b"{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":");
    push_u32(&mut bytes, settings.ticks_per_second);
    bytes.extend_from_slice(b",\"duration_ticks\":");
    push_u32(&mut bytes, settings.duration_ticks);
    bytes.extend_from_slice(b",\"fps\":{\"numerator\":");
    push_u32(&mut bytes, settings.fps_numerator);
    bytes.extend_from_slice(b",\"denominator\":");
    push_u32(&mut bytes, settings.fps_denominator);
    bytes.extend_from_slice(b"},\"loop\":\"");
    bytes.extend_from_slice(loop_mode);
    bytes.extend_from_slice(b"\",\"onion\":{\"before\":");
    push_u32(&mut bytes, settings.onion_before);
    bytes.extend_from_slice(b",\"after\":");
    push_u32(&mut bytes, settings.onion_after);
    bytes.extend_from_slice(b",\"opacity\":");
    bytes.extend_from_slice(&opacity.bytes[..opacity.len]);
    bytes.extend_from_slice(b"},\"tracks\":[]}");

    decode(&bytes, context).map_err(EditFailure::from)?;
    Ok(bytes)
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    let mut digits = [0u8; 10];
    let mut cursor = digits.len();
    let mut remaining = value;
    loop {
        cursor -= 1;
        digits[cursor] = b'0' + (remaining % 10) as u8;
        remaining /= 10;
        if remaining == 0 {
            break;
        }
    }
    bytes.extend_from_slice(&digits[cursor..]);
}

struct StackBuffer {
    bytes: [u8; 32],
    len: usize,
}
impl fmt::Write for StackBuffer {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self
            .len
            .checked_add(value.len())
            .filter(|end| *end <= self.bytes.len())
            .ok_or(fmt::Error)?;
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}
