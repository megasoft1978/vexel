//! Source preserving timeline editing with bounded snapshot history.
mod initialize;
#[cfg(test)]
mod initialize_tests;
mod ops;
mod ops_bulk;
mod ops_ext;
mod ops_timing;
#[cfg(test)]
mod visibility_tests;

use crate::{decode, AssetRef, DecodeFailure, Error, LoopMode, TimelineContext, MAX_SOURCE_BYTES};
use std::sync::Arc;

pub const DEFAULT_HISTORY_COMMANDS: usize = 16;
pub const DEFAULT_HISTORY_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_HISTORY_COMMANDS: usize = 1024;
pub const MAX_HISTORY_BYTES: usize = 512 * 1024 * 1024;
const VALUE_JSON_LIMIT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditLimits {
    pub commands: usize,
    pub bytes: usize,
}
impl Default for EditLimits {
    fn default() -> Self {
        Self {
            commands: DEFAULT_HISTORY_COMMANDS,
            bytes: DEFAULT_HISTORY_BYTES,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditFailure {
    pub error: Error,
    pub offset: usize,
}
impl From<DecodeFailure> for EditFailure {
    fn from(value: DecodeFailure) -> Self {
        Self {
            error: value.error,
            offset: value.offset,
        }
    }
}
pub type EditResult<T = ()> = Result<T, EditFailure>;

pub use initialize::{initialize_empty_timeline, EmptyTimelineSettings};
pub use ops_bulk::{BulkFailure, BulkKey, BulkTrack};

/// Owns source snapshots. Editor methods require exclusive access; cloned leases
/// are immutable, `Send + Sync`, and remain valid after history eviction/drop.
pub struct TimelineEdit {
    snapshots: Vec<Arc<Vec<u8>>>,
    cursor: usize,
    command_limit: usize,
    byte_limit: usize,
}
#[derive(Clone)]
pub struct TimelineSourceLease(Arc<Vec<u8>>);
impl TimelineSourceLease {
    pub fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TimelineEdit {
    pub fn new(source: &[u8], context: &TimelineContext, limits: EditLimits) -> EditResult<Self> {
        decode(source, context).map_err(EditFailure::from)?;
        Self::new_decoded(source, limits)
    }
    /// Like `new` for a source the caller already decoded against the same
    /// context (for example a candidate opened through `new`); skips the
    /// repeated full decode. Every committed edit is still decoded.
    pub fn new_decoded(source: &[u8], limits: EditLimits) -> EditResult<Self> {
        if limits.commands == 0
            || limits.commands > MAX_HISTORY_COMMANDS
            || limits.bytes == 0
            || limits.bytes > MAX_HISTORY_BYTES
        {
            return Err(fail(Error::InvalidArgument, 0));
        }
        if source.is_empty() || source.len() > limits.bytes {
            return Err(fail(Error::Limit, 0));
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(source.len())
            .map_err(|_| fail(Error::Limit, 0))?;
        owned.extend_from_slice(source);
        let mut snapshots = Vec::new();
        snapshots
            .try_reserve_exact(
                limits
                    .commands
                    .checked_add(1)
                    .ok_or_else(|| fail(Error::Limit, 0))?,
            )
            .map_err(|_| fail(Error::Limit, 0))?;
        // Arc's allocation is provided by std and cannot report allocation failure.
        snapshots.push(Arc::new(owned));
        Ok(Self {
            snapshots,
            cursor: 0,
            command_limit: limits.commands,
            byte_limit: limits.bytes,
        })
    }
    pub fn source(&self) -> &[u8] {
        self.snapshots[self.cursor].as_slice()
    }
    pub fn source_lease(&self) -> TimelineSourceLease {
        TimelineSourceLease(self.snapshots[self.cursor].clone())
    }
    pub fn undo_count(&self) -> usize {
        self.cursor
    }
    pub fn redo_count(&self) -> usize {
        self.snapshots.len().saturating_sub(self.cursor + 1)
    }

    pub fn undo(&mut self, context: &TimelineContext) -> EditResult {
        if self.cursor == 0 {
            return Err(fail(Error::InvalidValue, 0));
        }
        let next = self.cursor - 1;
        decode(&self.snapshots[next], context).map_err(EditFailure::from)?;
        self.cursor = next;
        Ok(())
    }
    pub fn redo(&mut self, context: &TimelineContext) -> EditResult {
        let next = self
            .cursor
            .checked_add(1)
            .ok_or_else(|| fail(Error::Limit, 0))?;
        let bytes = self
            .snapshots
            .get(next)
            .ok_or_else(|| fail(Error::InvalidValue, 0))?;
        decode(bytes, context).map_err(EditFailure::from)?;
        self.cursor = next;
        Ok(())
    }
    pub(super) fn commit(&mut self, context: &TimelineContext, candidate: Vec<u8>) -> EditResult {
        if candidate.as_slice() == self.source() {
            return Ok(());
        }
        if candidate.len() > MAX_SOURCE_BYTES || candidate.len() > self.byte_limit {
            return Err(fail(Error::Limit, 0));
        }
        decode(&candidate, context).map_err(EditFailure::from)?;
        let mut retained = self.cursor + 1;
        let mut removed = 0usize;
        let mut retained_bytes = 0usize;
        for bytes in &self.snapshots[..retained] {
            retained_bytes = retained_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| fail(Error::Limit, 0))?;
        }
        while retained + 1 > self.command_limit + 1
            || retained_bytes > self.byte_limit - candidate.len()
        {
            if retained <= 1 {
                return Err(fail(Error::Limit, 0));
            }
            retained_bytes -= self.snapshots[removed].len();
            removed += 1;
            retained -= 1;
        }
        // Arc allocates its reference-count header through std (not fallibly).
        // The already validated Vec buffer itself is moved without another copy.
        let next = Arc::new(candidate);
        self.snapshots.truncate(self.cursor + 1);
        if removed != 0 {
            self.snapshots.drain(..removed);
            self.cursor -= removed;
        }
        self.snapshots.push(next);
        self.cursor = self.snapshots.len() - 1;
        Ok(())
    }

    pub fn add_drawing_track(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        target_id: &[u8],
        asset_name: &[u8],
        track_name: &[u8],
    ) -> EditResult {
        ops::add_drawing_track(self, context, track_id, target_id, asset_name, track_name)
    }
    pub fn add_property_track(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        target_id: &[u8],
        property: &[u8],
        value_type: &[u8],
        interpolation: &[u8],
        value_json: &[u8],
        track_name: &[u8],
    ) -> EditResult {
        ops::add_property_track(
            self,
            context,
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            value_json,
            track_name,
        )
    }
    pub fn add_property_track_key(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        target_id: &[u8],
        property: &[u8],
        value_type: &[u8],
        interpolation: &[u8],
        initial_json: &[u8],
        tick: u32,
        value_json: &[u8],
        track_name: &[u8],
    ) -> EditResult {
        ops::add_property_track_key(
            self,
            context,
            track_id,
            target_id,
            property,
            value_type,
            interpolation,
            initial_json,
            tick,
            value_json,
            track_name,
        )
    }
    pub fn set_property_key(&mut self, context: &TimelineContext, track_id: &[u8], tick: u32, value_json: &[u8]) -> EditResult {
        self.set_property_key_with_ease(context, track_id, tick, value_json, None)
    }
    pub fn set_property_key_with_ease(&mut self, context: &TimelineContext, track_id: &[u8], tick: u32, value_json: &[u8], ease_json: Option<&[u8]>) -> EditResult {
        ops::set_property_key(self, context, track_id, tick, value_json, ease_json)
    }
    /// Adds many property tracks, each with its keys, in one splice and one
    /// validation. Equivalent to `add_property_track` followed by
    /// `set_property_key_with_ease` for each key, in order, but O(total) once.
    pub fn add_property_tracks(&mut self, context: &TimelineContext, tracks: &[BulkTrack<'_>]) -> Result<(), BulkFailure> {
        ops_bulk::add_property_tracks(self, context, tracks)
    }
    pub fn set_cel(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        tick: u32,
        asset_name: &[u8],
        label: &[u8],
    ) -> EditResult {
        ops::set_cel(self, context, track_id, tick, asset_name, label)
    }
    pub fn remove_cel(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        tick: u32,
    ) -> EditResult {
        ops::remove_cel(self, context, track_id, tick)
    }
    pub fn move_item(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        from_tick: u32,
        to_tick: u32,
    ) -> EditResult {
        ops::move_item(self, context, track_id, from_tick, to_tick)
    }
    pub fn set_loop(&mut self, context: &TimelineContext, mode: LoopMode) -> EditResult {
        ops::set_loop(self, context, mode)
    }
    /// Sets the duration in ticks and optionally the ticks per second.
    pub fn set_timing(
        &mut self,
        context: &TimelineContext,
        duration_ticks: u32,
        ticks_per_second: Option<u32>,
    ) -> EditResult {
        ops_timing::set_timing(self, context, duration_ticks, ticks_per_second)
    }
    pub fn set_onion(
        &mut self,
        context: &TimelineContext,
        before: u32,
        after: u32,
        opacity: f64,
    ) -> EditResult {
        ops::set_onion(self, context, before, after, opacity)
    }
    pub fn set_track_visibility(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        visible: bool,
    ) -> EditResult {
        ops::set_track_visibility(self, context, track_id, visible)
    }
    pub fn remove_property_key(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        tick: u32,
    ) -> EditResult {
        ops_ext::remove_property_key(self, context, track_id, tick)
    }
    pub fn move_property_key(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        from_tick: u32,
        to_tick: u32,
    ) -> EditResult {
        ops_ext::move_property_key(self, context, track_id, from_tick, to_tick)
    }
    pub fn remove_track(&mut self, context: &TimelineContext, track_id: &[u8]) -> EditResult {
        ops_ext::remove_track(self, context, track_id)
    }
    pub fn rename_track(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        new_name: &[u8],
    ) -> EditResult {
        ops_ext::rename_track(self, context, track_id, new_name)
    }
    pub fn duplicate_track(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        new_track_id: &[u8],
    ) -> EditResult {
        ops_ext::duplicate_track(self, context, track_id, new_track_id)
    }
    pub fn set_track_interpolation(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        interpolation: &[u8],
    ) -> EditResult {
        ops_ext::set_track_interpolation(self, context, track_id, interpolation)
    }
    pub fn reorder_track(
        &mut self,
        context: &TimelineContext,
        track_id: &[u8],
        new_index: usize,
    ) -> EditResult {
        ops_ext::reorder_track(self, context, track_id, new_index)
    }
}

pub(super) fn fail(error: Error, offset: usize) -> EditFailure {
    EditFailure { error, offset }
}
pub(super) fn validate_utf8(value: &[u8]) -> bool {
    std::str::from_utf8(value).is_ok()
}
pub(super) fn valid_id(value: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'-'))
}
pub(super) fn valid_target(context: &TimelineContext, id: &[u8]) -> bool {
    context
        .scene_ids
        .binary_search_by(|candidate| candidate.as_slice().cmp(id))
        .is_ok()
}
pub(super) fn valid_asset(context: &TimelineContext, name: &[u8]) -> bool {
    context
        .assets
        .iter()
        .any(|a: &AssetRef| a.name == name && a.media_type == b"image/svg+xml")
}
