//! Per-window tick scheduling state. Timer installation is owned by the native record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TickState { pub enabled: bool, pub generation: u32 }
impl TickState { pub(crate) fn set(&mut self, enabled: bool) { self.generation = self.generation.wrapping_add(1).max(1); self.enabled = enabled; } }
