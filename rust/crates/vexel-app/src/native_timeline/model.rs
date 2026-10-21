//! Bounded, read-only presentation model for the native animation timeline.
//!
//! It intentionally copies only strings and primitive values from the
//! coordinator snapshot. No document or renderer handles survive this step.

use super::{NativeTimelineSnapshot, NativeTrackKind};

pub const MAX_VISIBLE_TRACKS: usize = 256;
pub const MAX_ITEMS_PER_TRACK: usize = 100_000;
pub const MAX_VISIBLE_ITEMS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineModel {
    pub ticks_per_second: u32,
    pub duration_ticks: u32,
    pub current_tick: u32,
    pub tracks: Vec<Track>,
    pub selected_target: Option<String>,
    pub playing: bool,
    pub loop_enabled: bool,
    pub onion_before: u32,
    pub onion_after: u32,
    pub onion_opacity: f64,
    pub can_add_drawing_track: bool,
    pub can_import_asset: bool,
    pub can_edit_selected_item: bool,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub target: String,
    pub kind: NativeTrackKind,
    pub visible: bool,
    pub selected: bool,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub tick: u32,
    pub label: String,
    pub selected: bool,
    pub ease: Option<String>,
}

impl TimelineModel {
    pub fn from_snapshot(snapshot: &NativeTimelineSnapshot) -> Self {
        let duration_ticks = snapshot.duration_ticks.max(1);
        let mut remaining_items = MAX_VISIBLE_ITEMS;
        let tracks = snapshot
            .tracks
            .iter()
            .take(MAX_VISIBLE_TRACKS)
            .map(|track| {
                let item_count = track
                    .items
                    .len()
                    .min(MAX_ITEMS_PER_TRACK)
                    .min(remaining_items);
                remaining_items -= item_count;
                Track {
                    id: bounded(&track.id, 256),
                    name: bounded(&track.name, 256),
                    target: bounded(&track.target, 256),
                    kind: track.kind,
                    visible: track.visible,
                    selected: track.selected,
                    items: track
                        .items
                        .iter()
                        .take(item_count)
                        .map(|item| Item {
                            tick: item.tick.min(duration_ticks - 1),
                            label: bounded(&item.label, 256),
                            selected: item.selected,
                            ease: item.ease.clone(),
                        })
                        .collect(),
                }
            })
            .collect();
        Self {
            ticks_per_second: snapshot.ticks_per_second.max(1),
            duration_ticks,
            current_tick: snapshot.current_tick.min(duration_ticks - 1),
            tracks,
            selected_target: snapshot
                .selected_target
                .as_deref()
                .map(|target| bounded(target, 256)),
            playing: snapshot.playing,
            loop_enabled: snapshot.loop_enabled,
            onion_before: snapshot.onion_before.min(8),
            onion_after: snapshot.onion_after.min(8),
            onion_opacity: if snapshot.onion_opacity.is_finite() {
                snapshot.onion_opacity.clamp(0.0, 1.0)
            } else {
                0.25
            },
            can_add_drawing_track: snapshot.can_add_drawing_track,
            can_import_asset: snapshot.can_import_asset,
            can_edit_selected_item: snapshot.can_edit_selected_item,
            status: snapshot.status.as_deref().map(|s| bounded(s, 512)),
        }
    }

    pub fn selected_track(&self) -> Option<(usize, &Track)> {
        self.tracks
            .iter()
            .enumerate()
            .find(|(_, track)| track.selected)
    }

    pub fn selected_item(&self) -> Option<(usize, usize, &Track, &Item)> {
        self.tracks.iter().enumerate().find_map(|(ti, track)| {
            track
                .items
                .iter()
                .enumerate()
                .find(|(_, item)| item.selected)
                .map(|(ii, item)| (ti, ii, track, item))
        })
    }
}

fn bounded(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
