//! Bounded owned copy of the opaque payload retained by a project session.

use std::sync::Arc;

use super::SessionError;
use super::history::PayloadSnapshot;
use super::svg_asset::OwnedProjectAsset;
use super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError as TimelineError};
use vexel_project::{
    ASSET_HEADER_BYTES, Asset, BundleInput, BundleView, HEADER_BYTES, MAX_ASSET_BYTES,
    MAX_ASSET_COUNT, MAX_ASSET_NAME_BYTES, MAX_ASSET_TOTAL_BYTES, MAX_BYTES, MAX_MANIFEST_BYTES,
    MAX_MEDIA_TYPE_BYTES, MAX_SCENE_BYTES, MAX_TIMELINE_BYTES, encoded_size,
};
use vexel_timeline::{EditLimits, EmptyTimelineSettings};

#[derive(Clone, Debug)]
pub(super) struct SessionPayload {
    data: Arc<PayloadData>,
}

#[derive(Debug)]
struct PayloadData {
    manifest_json: Vec<u8>,
    timeline_json: Vec<u8>,
    assets: Vec<OwnedAsset>,
}

#[derive(Debug)]
struct OwnedAsset {
    name: Vec<u8>,
    media_type: Vec<u8>,
    data: Vec<u8>,
}

impl SessionPayload {
    pub(super) fn try_from_view_with(
        view: BundleView<'_>,
        reserve_allowed: &mut impl FnMut() -> bool,
    ) -> Result<Self, ()> {
        let manifest_json = copy_bytes(view.manifest_json, reserve_allowed)?;
        let timeline_json = copy_bytes(view.timeline_json, reserve_allowed)?;
        let mut assets = Vec::new();
        reserve(&mut assets, view.asset_count, reserve_allowed)?;
        for index in 0..view.asset_count {
            let asset = view.asset_at(index).expect("validated bundle asset");
            assets.push(OwnedAsset {
                name: copy_bytes(asset.name, reserve_allowed)?,
                media_type: copy_bytes(asset.media_type, reserve_allowed)?,
                data: copy_bytes(asset.data, reserve_allowed)?,
            });
        }
        Ok(Self {
            data: Arc::new(PayloadData {
                manifest_json,
                timeline_json,
                assets,
            }),
        })
    }

    pub(super) fn defaults(width: u32, height: u32) -> Self {
        Self {
            data: Arc::new(PayloadData {
                manifest_json: format!(
                    "{{\"format\":\"vexel-project\",\"width\":{width},\"height\":{height}}}"
                )
                .into_bytes(),
                // ADR-0020 reserves zero-length bytes as the schema's empty
                // timeline representation; nonempty `{}` remains opaque.
                timeline_json: Vec::new(),
                assets: Vec::new(),
            }),
        }
    }

    /// Both handles share one immutable payload.
    pub(super) fn same_payload(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    /// The payload already holds a timeline (an empty one is initialised from
    /// settings when opened, so it is never a stable candidate).
    pub(super) fn has_timeline(&self) -> bool {
        !self.data.timeline_json.is_empty()
    }

    #[cfg(test)]
    pub(super) fn timeline_bytes(&self) -> &[u8] {
        &self.data.timeline_json
    }

    pub(super) fn timeline_candidate(
        &self,
        scene_ids: Vec<Vec<u8>>,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
    ) -> Result<ProjectTimelineCandidate, TimelineError> {
        let assets = copy_project_assets(&self.data.assets)?;
        ProjectTimelineCandidate::open(
            &self.data.timeline_json,
            scene_ids,
            assets,
            settings,
            limits,
        )
    }

    pub(super) fn with_timeline_candidate(
        &self,
        candidate: &ProjectTimelineCandidate,
    ) -> Result<Self, SessionError> {
        self.with_timeline_candidate_with(candidate, &mut || true)
    }

    fn with_timeline_candidate_with(
        &self,
        candidate: &ProjectTimelineCandidate,
        reserve_allowed: &mut impl FnMut() -> bool,
    ) -> Result<Self, SessionError> {
        validate_payload_bounds(
            &self.data.manifest_json,
            candidate.timeline_bytes(),
            candidate.assets(),
            reserve_allowed,
        )?;
        let timeline_json = copy_bytes(candidate.timeline_bytes(), reserve_allowed)
            .map_err(|()| SessionError::OutOfMemory)?;
        let mut assets = Vec::new();
        reserve(&mut assets, candidate.assets().len(), reserve_allowed)
            .map_err(|()| SessionError::OutOfMemory)?;
        for asset in candidate.assets() {
            assets.push(OwnedAsset {
                name: copy_bytes(&asset.name, reserve_allowed)
                    .map_err(|()| SessionError::OutOfMemory)?,
                media_type: copy_bytes(&asset.media_type, reserve_allowed)
                    .map_err(|()| SessionError::OutOfMemory)?,
                data: copy_bytes(&asset.source, reserve_allowed)
                    .map_err(|()| SessionError::OutOfMemory)?,
            });
        }
        let manifest_json = copy_bytes(&self.data.manifest_json, reserve_allowed)
            .map_err(|()| SessionError::OutOfMemory)?;
        Ok(Self {
            data: Arc::new(PayloadData {
                manifest_json,
                timeline_json,
                assets,
            }),
        })
    }

    pub(super) fn with_bundle_input<R>(
        &self,
        scene_svg: &[u8],
        write: impl FnOnce(&BundleInput<'_>) -> R,
    ) -> R {
        self.try_with_bundle_input(scene_svg, &mut || true, |input| Ok(write(input)))
            .expect("bundle asset vector allocation failed")
    }

    pub(super) fn try_with_bundle_input<R>(
        &self,
        scene_svg: &[u8],
        reserve_allowed: &mut impl FnMut() -> bool,
        write: impl FnOnce(&BundleInput<'_>) -> Result<R, ()>,
    ) -> Result<R, ()> {
        let mut assets = Vec::new();
        reserve(&mut assets, self.data.assets.len(), reserve_allowed)?;
        for asset in &self.data.assets {
            assets.push(Asset {
                name: &asset.name,
                media_type: &asset.media_type,
                data: &asset.data,
            });
        }
        let input = BundleInput {
            manifest_json: &self.data.manifest_json,
            scene_svg,
            timeline_json: &self.data.timeline_json,
            assets: &assets,
        };
        write(&input)
    }
}

impl PayloadSnapshot for SessionPayload {
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.data) as usize
    }

    fn retained_bytes(&self) -> usize {
        let mut total = self.data.manifest_json.len();
        total = total
            .checked_add(self.data.timeline_json.len())
            .expect("bounded payload byte accounting");
        for asset in &self.data.assets {
            total = total
                .checked_add(asset.name.len())
                .and_then(|bytes| bytes.checked_add(asset.media_type.len()))
                .and_then(|bytes| bytes.checked_add(asset.data.len()))
                .expect("bounded payload byte accounting");
        }
        total
    }
}

fn copy_project_assets(source: &[OwnedAsset]) -> Result<Vec<OwnedProjectAsset>, TimelineError> {
    let mut assets = Vec::new();
    assets
        .try_reserve_exact(source.len())
        .map_err(|_| TimelineError::OutOfMemory)?;
    for asset in source {
        assets.push(OwnedProjectAsset {
            name: copy_timeline_bytes(&asset.name)?,
            media_type: copy_timeline_bytes(&asset.media_type)?,
            source: copy_timeline_bytes(&asset.data)?,
        });
    }
    Ok(assets)
}

fn copy_timeline_bytes(source: &[u8]) -> Result<Vec<u8>, TimelineError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(source.len())
        .map_err(|_| TimelineError::OutOfMemory)?;
    bytes.extend_from_slice(source);
    Ok(bytes)
}

fn validate_payload_bounds(
    manifest: &[u8],
    timeline: &[u8],
    assets: &[OwnedProjectAsset],
    reserve_allowed: &mut impl FnMut() -> bool,
) -> Result<(), SessionError> {
    if manifest.is_empty()
        || manifest.len() > MAX_MANIFEST_BYTES
        || timeline.len() > MAX_TIMELINE_BYTES
        || assets.len() > MAX_ASSET_COUNT
    {
        return Err(SessionError::TooLarge);
    }
    let mut bundle_assets = Vec::new();
    reserve(&mut bundle_assets, assets.len(), reserve_allowed)
        .map_err(|()| SessionError::OutOfMemory)?;
    let mut asset_total = 0usize;
    for asset in assets {
        if asset.name.is_empty()
            || asset.name.len() > MAX_ASSET_NAME_BYTES
            || asset.media_type.is_empty()
            || asset.media_type.len() > MAX_MEDIA_TYPE_BYTES
            || asset.source.len() > MAX_ASSET_BYTES
        {
            return Err(SessionError::TooLarge);
        }
        asset_total = asset_total
            .checked_add(asset.source.len())
            .ok_or(SessionError::TooLarge)?;
        if asset_total > MAX_ASSET_TOTAL_BYTES {
            return Err(SessionError::TooLarge);
        }
        bundle_assets.push(Asset {
            name: &asset.name,
            media_type: &asset.media_type,
            data: &asset.source,
        });
    }
    // The checked compile-time bound below includes the largest legal scene,
    // all component maxima and every encoded record header (873,742,392 bytes).
    // It is strictly below MAX_BYTES, so any legal session-owned scene fits;
    // the one-byte placeholder here only supplies encoded_size's nonempty
    // scene requirement while it validates asset names, media and duplicates.
    let input = BundleInput {
        manifest_json: manifest,
        scene_svg: b"x",
        timeline_json: timeline,
        assets: &bundle_assets,
    };
    match encoded_size(&input) {
        Ok(size) if size <= MAX_BYTES => Ok(()),
        Ok(_) => Err(SessionError::TooLarge),
        Err(error) if error.error == vexel_project::Error::OutOfMemory => {
            Err(SessionError::OutOfMemory)
        }
        Err(_) => Err(SessionError::TooLarge),
    }
}

const _: () = {
    let asset_header_and_name = match ASSET_HEADER_BYTES.checked_add(MAX_ASSET_NAME_BYTES) {
        Some(bytes) => bytes,
        None => panic!("project asset name bound overflow"),
    };
    let asset_record = match asset_header_and_name.checked_add(MAX_MEDIA_TYPE_BYTES) {
        Some(bytes) => bytes,
        None => panic!("project asset record bound overflow"),
    };
    let mut total = HEADER_BYTES;
    let components = [
        MAX_MANIFEST_BYTES,
        MAX_SCENE_BYTES,
        MAX_TIMELINE_BYTES,
        MAX_ASSET_TOTAL_BYTES,
        match asset_record.checked_mul(MAX_ASSET_COUNT) {
            Some(bytes) => bytes,
            None => panic!("project asset table bound overflow"),
        },
    ];
    let mut index = 0;
    while index < components.len() {
        total = match total.checked_add(components[index]) {
            Some(bytes) => bytes,
            None => panic!("project encoded bound overflow"),
        };
        index += 1;
    }
    assert!(total < MAX_BYTES);
};

#[cfg(test)]
#[path = "payload_timeline_tests.rs"]
mod payload_timeline_tests;

fn copy_bytes(source: &[u8], reserve_allowed: &mut impl FnMut() -> bool) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    reserve(&mut bytes, source.len(), reserve_allowed)?;
    bytes.extend_from_slice(source);
    Ok(bytes)
}

fn reserve<T>(
    values: &mut Vec<T>,
    additional: usize,
    reserve_allowed: &mut impl FnMut() -> bool,
) -> Result<(), ()> {
    if additional == 0 {
        return Ok(());
    }
    if !reserve_allowed() {
        return Err(());
    }
    values.try_reserve_exact(additional).map_err(|_| ())
}
