//! Generation invalidation rules shared by explicit destroy and app shutdown.
pub(crate) fn next_generation(generation: u32) -> Option<u32> { generation.checked_add(1).filter(|v| *v != 0) }
