//! Transactional native-timeline and drawing-asset payload candidates.

use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering};

use super::svg_asset::{OwnedProjectAsset, SvgAssetError};
use crate::animation::native_drawing::SvgGeometry;
use crate::profile::phase;
use vexel_timeline::{
    AssetRef, BulkFailure, BulkTrack, EditFailure, EditLimits, EmptyTimelineSettings,
    Error as TimelineError, LoopMode, MAX_SOURCE_BYTES, TimelineContext, TimelineEdit, classify,
    initialize_empty_timeline,
};

static NEXT_CANDIDATE_LINEAGE: AtomicU64 = AtomicU64::new(1);

fn next_lineage() -> Result<u64, ProjectTimelineError> {
    NEXT_CANDIDATE_LINEAGE
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| ProjectTimelineError::Limit)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectTimelineError {
    UnsupportedTimeline,
    InvalidTimeline(TimelineError),
    Edit(EditFailure),
    Asset(SvgAssetError),
    Limit,
    OutOfMemory,
}

impl From<EditFailure> for ProjectTimelineError {
    fn from(value: EditFailure) -> Self {
        Self::Edit(value)
    }
}

/// One owned candidate for the app journal. Opaque timelines can be retained
/// and saved unchanged, but cannot be edited by the v1 helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectTimelineCandidate {
    timeline: Vec<u8>,
    scene_ids: Vec<Vec<u8>>,
    assets: Vec<OwnedProjectAsset>,
    editable: bool,
    limits: EditLimits,
    lineage: u64,
}

impl ProjectTimelineCandidate {
    /// Open supported v1 data, initialize an explicitly empty timeline, or
    /// retain a valid but unsupported timeline byte-for-byte.
    pub fn open(
        timeline: &[u8],
        scene_ids: Vec<Vec<u8>>,
        assets: Vec<OwnedProjectAsset>,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
    ) -> Result<Self, ProjectTimelineError> {
        let lineage = next_lineage()?;
        if timeline.len() > MAX_SOURCE_BYTES {
            return Err(ProjectTimelineError::Limit);
        }
        let context = make_context(&scene_ids, &assets)?;
        if timeline.is_empty() {
            let initialized = initialize_empty_timeline(settings, &context)
                .map_err(ProjectTimelineError::Edit)?;
            return Ok(Self {
                timeline: initialized,
                scene_ids,
                assets,
                editable: true,
                limits,
                lineage,
            });
        }
        if !classify(timeline)
            .map_err(|failure| ProjectTimelineError::InvalidTimeline(failure.error))?
        {
            return Ok(Self {
                timeline: copy_bytes(timeline)?,
                scene_ids,
                assets,
                editable: false,
                limits,
                lineage,
            });
        }
        let editor =
            TimelineEdit::new(timeline, &context, limits).map_err(ProjectTimelineError::Edit)?;
        Ok(Self {
            timeline: copy_bytes(editor.source())?,
            scene_ids,
            assets,
            editable: true,
            limits,
            lineage,
        })
    }

    pub fn timeline_bytes(&self) -> &[u8] {
        &self.timeline
    }
    pub fn assets(&self) -> &[OwnedProjectAsset] {
        &self.assets
    }

    pub(super) fn scene_ids(&self) -> &[Vec<u8>] {
        &self.scene_ids
    }
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Decodes this candidate while borrowing its owned bytes and context.
    pub fn view(&self) -> Result<vexel_timeline::TimelineView<'_>, ProjectTimelineError> {
        self.require_editable()?;
        let context = make_context(&self.scene_ids, &self.assets)?;
        vexel_timeline::decode(&self.timeline, &context)
            .map_err(|failure| ProjectTimelineError::InvalidTimeline(failure.error))
    }

    pub(super) fn same_lineage(&self, other: &Self) -> bool {
        self.lineage == other.lineage
    }

    /// True when `self` was derived from `parent` by the editing API (same
    /// lineage, scene ids and limits). Every such derivative's timeline was
    /// decoded against its own assets when it was committed, so re-opening it
    /// would repeat that validation.
    pub(super) fn is_validated_derivative_of(&self, parent: &Self, limits: EditLimits) -> bool {
        self.lineage == parent.lineage
            && self.editable
            && !self.timeline.is_empty()
            && self.limits == limits
            && parent.limits == limits
            && self.scene_ids == parent.scene_ids
    }

    /// Import the source asset and its first cel/track as one candidate.
    pub fn import_drawing_track(
        &self,
        name: &[u8],
        media_type: &[u8],
        source: &[u8],
        geometry: SvgGeometry,
        track_id: &[u8],
        target_id: &[u8],
        track_name: &[u8],
        tick: u32,
        cel_label: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.require_editable()?;
        let asset = OwnedProjectAsset::import(name, media_type, source, geometry, &self.assets)
            .map_err(ProjectTimelineError::Asset)?;
        let mut staged_assets = copy_assets(&self.assets)?;
        staged_assets
            .try_reserve(1)
            .map_err(|_| ProjectTimelineError::OutOfMemory)?;
        staged_assets.push(asset);
        let context = make_context(&self.scene_ids, &staged_assets)?;
        let mut editor = TimelineEdit::new(&self.timeline, &context, self.limits)
            .map_err(ProjectTimelineError::Edit)?;
        editor.add_drawing_track(&context, track_id, target_id, name, track_name)?;
        editor.set_cel(&context, track_id, tick, name, cel_label)?;
        self.with_timeline(editor.source(), staged_assets)
    }

    /// Import a validated SVG asset without changing the timeline.
    pub fn add_drawing_asset(
        &self,
        name: &[u8],
        media_type: &[u8],
        source: &[u8],
        geometry: SvgGeometry,
    ) -> Result<Self, ProjectTimelineError> {
        self.require_editable()?;
        let asset = OwnedProjectAsset::import(name, media_type, source, geometry, &self.assets)
            .map_err(ProjectTimelineError::Asset)?;
        let mut assets = copy_assets(&self.assets)?;
        assets
            .try_reserve(1)
            .map_err(|_| ProjectTimelineError::OutOfMemory)?;
        assets.push(asset);
        self.with_timeline(&self.timeline, assets)
    }

    /// Add a drawing track that references an already imported asset.
    pub fn add_drawing_track(
        &self,
        track_id: &[u8],
        target_id: &[u8],
        asset_name: &[u8],
        track_name: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.add_drawing_track(context, track_id, target_id, asset_name, track_name)
        })
    }

    pub fn add_property_track(
        &self,
        track_id: &[u8],
        target_id: &[u8],
        property: &[u8],
        value_type: &[u8],
        interpolation: &[u8],
        value_json: &[u8],
        name: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.add_property_track(
                context,
                track_id,
                target_id,
                property,
                value_type,
                interpolation,
                value_json,
                name,
            )
        })
    }

    pub fn set_property_key(
        &self,
        track_id: &[u8],
        tick: u32,
        value_json: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.set_property_key_with_ease(track_id, tick, value_json, None)
    }
    pub fn set_property_key_with_ease(
        &self,
        track_id: &[u8],
        tick: u32,
        value_json: &[u8],
        ease: Option<&[u8]>,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.set_property_key_with_ease(context, track_id, tick, value_json, ease)
        })
    }

    pub fn remove_property_key(
        &self,
        track_id: &[u8],
        tick: u32,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.remove_property_key(context, track_id, tick))
    }

    pub fn move_property_key(
        &self,
        track_id: &[u8],
        from_tick: u32,
        to_tick: u32,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.move_property_key(context, track_id, from_tick, to_tick)
        })
    }

    pub fn remove_track(&self, track_id: &[u8]) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.remove_track(context, track_id))
    }

    pub fn rename_track(&self, track_id: &[u8], name: &[u8]) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.rename_track(context, track_id, name))
    }

    pub fn duplicate_track(
        &self,
        track_id: &[u8],
        new_track_id: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.duplicate_track(context, track_id, new_track_id))
    }

    /// Adds many property tracks with all their keys in one splice and one
    /// validation. A rejection stores the offending track in `failure`.
    pub fn add_property_tracks(
        &self,
        tracks: &[BulkTrack<'_>],
        failure: &Cell<Option<BulkFailure>>,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.add_property_tracks(context, tracks).map_err(|f| {
                failure.set(Some(f));
                f.failure
            })
        })
    }

    pub fn set_track_interpolation(
        &self,
        track_id: &[u8],
        interpolation: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.set_track_interpolation(context, track_id, interpolation)
        })
    }

    pub fn reorder_track(
        &self,
        track_id: &[u8],
        index: usize,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.reorder_track(context, track_id, index))
    }

    pub fn set_cel(
        &self,
        track_id: &[u8],
        tick: u32,
        asset_name: &[u8],
        label: &[u8],
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.set_cel(context, track_id, tick, asset_name, label))
    }

    pub fn set_track_visibility(
        &self,
        track_id: &[u8],
        visible: bool,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.set_track_visibility(context, track_id, visible))
    }

    pub fn remove_cel(&self, track_id: &[u8], tick: u32) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.remove_cel(context, track_id, tick))
    }

    pub fn move_item(
        &self,
        track_id: &[u8],
        from_tick: u32,
        to_tick: u32,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.move_item(context, track_id, from_tick, to_tick))
    }

    pub fn set_loop(&self, mode: LoopMode) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.set_loop(context, mode))
    }

    pub fn set_timing(
        &self,
        duration_ticks: u32,
        ticks_per_second: Option<u32>,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| {
            editor.set_timing(context, duration_ticks, ticks_per_second)
        })
    }

    pub fn set_onion(
        &self,
        before: u32,
        after: u32,
        opacity: f64,
    ) -> Result<Self, ProjectTimelineError> {
        self.transact(|editor, context| editor.set_onion(context, before, after, opacity))
    }

    fn transact(
        &self,
        operation: impl FnOnce(&mut TimelineEdit, &TimelineContext) -> Result<(), EditFailure>,
    ) -> Result<Self, ProjectTimelineError> {
        self.require_editable()?;
        let context = {
            let _phase = phase("edit_context");
            make_context(&self.scene_ids, &self.assets)?
        };
        let mut editor = {
            let _phase = phase("edit_validate");
            // `timeline` was decoded when this candidate was opened or built
            // from a committed (decoded) edit, under this same context.
            TimelineEdit::new_decoded(&self.timeline, self.limits)
                .map_err(ProjectTimelineError::Edit)?
        };
        {
            let _phase = phase("edit_ops");
            operation(&mut editor, &context).map_err(ProjectTimelineError::Edit)?;
        }
        let _phase = phase("edit_copy");
        self.with_timeline(editor.source(), copy_assets(&self.assets)?)
    }

    fn with_timeline(
        &self,
        source: &[u8],
        assets: Vec<OwnedProjectAsset>,
    ) -> Result<Self, ProjectTimelineError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ProjectTimelineError::Limit);
        }
        Ok(Self {
            timeline: copy_bytes(source)?,
            scene_ids: copy_ids(&self.scene_ids)?,
            assets,
            editable: self.editable,
            limits: self.limits,
            lineage: self.lineage,
        })
    }

    fn require_editable(&self) -> Result<(), ProjectTimelineError> {
        if self.editable {
            Ok(())
        } else {
            Err(ProjectTimelineError::UnsupportedTimeline)
        }
    }
}

fn make_context(
    scene_ids: &[Vec<u8>],
    assets: &[OwnedProjectAsset],
) -> Result<TimelineContext, ProjectTimelineError> {
    let mut ids = copy_ids(scene_ids)?;
    ids.sort();
    let mut refs = Vec::new();
    refs.try_reserve_exact(assets.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    for asset in assets {
        refs.push(AssetRef {
            name: copy_bytes(&asset.name)?,
            media_type: copy_bytes(&asset.media_type)?,
        });
    }
    Ok(TimelineContext {
        scene_ids: ids,
        assets: refs,
    })
}

fn copy_ids(source: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, ProjectTimelineError> {
    let mut ids = Vec::new();
    ids.try_reserve_exact(source.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    for id in source {
        ids.push(copy_bytes(id)?);
    }
    Ok(ids)
}

fn copy_assets(
    source: &[OwnedProjectAsset],
) -> Result<Vec<OwnedProjectAsset>, ProjectTimelineError> {
    let mut assets = Vec::new();
    assets
        .try_reserve_exact(source.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    for asset in source {
        assets.push(OwnedProjectAsset {
            name: copy_bytes(&asset.name)?,
            media_type: copy_bytes(&asset.media_type)?,
            source: copy_bytes(&asset.source)?,
        });
    }
    Ok(assets)
}

fn copy_bytes(source: &[u8]) -> Result<Vec<u8>, ProjectTimelineError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(source.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    bytes.extend_from_slice(source);
    Ok(bytes)
}
