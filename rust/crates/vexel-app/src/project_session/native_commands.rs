//! Atomic native-timeline commands for project sessions.

use super::payload::SessionPayload;
use super::svg_asset::OwnedProjectAsset;
use super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};
use super::{ProjectSession, SessionError};
use crate::profile::phase;
use vexel_timeline::{EditLimits, EmptyTimelineSettings};
use vexel_xml::{TokenKind, decode_attribute_value, tokenize_large};

/// Scene-ID inventory limits from the native timeline-v1 contract.
pub const MAX_SCENE_ID_BYTES: usize = 256;
pub const MAX_SCENE_IDS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneIdsError {
    Malformed,
    Duplicate,
    Limit,
    OutOfMemory,
}

/// Failure categories for an atomic native command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeCommandError<E> {
    Session(SessionError),
    Timeline(ProjectTimelineError),
    SceneIds(SceneIdsError),
    CandidateLineageChanged,
    ExistingAssetsChanged,
    Operation(E),
}

/// A candidate decoded for one scene state, payload and edit limits.
#[derive(Debug)]
pub(super) struct CandidateCache {
    state: u64,
    payload: SessionPayload,
    limits: EditLimits,
    candidate: ProjectTimelineCandidate,
}

impl ProjectSession {
    /// Returns a validated view of the current timeline and its owned assets
    /// without publishing a journal transition. Opaque timeline bytes remain
    /// available through the non-editable candidate.
    pub fn timeline_candidate(
        &self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
    ) -> Result<ProjectTimelineCandidate, NativeCommandError<()>> {
        let state = self.document.history_state_id();
        if let Some(cache) = self.candidate.borrow().as_ref() {
            if cache.state == state
                && cache.limits == limits
                && cache.payload.same_payload(&self.payload)
            {
                let _phase = phase("candidate_cached");
                return Ok(cache.candidate.clone());
            }
        }
        let scene_ids = self.cached_scene_ids()?;
        let candidate = self
            .payload
            .timeline_candidate(scene_ids, settings, limits)
            .map_err(NativeCommandError::Timeline)?;
        self.remember_candidate(state, limits, &candidate);
        Ok(candidate)
    }

    fn remember_candidate(
        &self,
        state: u64,
        limits: EditLimits,
        candidate: &ProjectTimelineCandidate,
    ) {
        if !self.payload.has_timeline() || !candidate.is_editable() {
            return;
        }
        *self.candidate.borrow_mut() = Some(CandidateCache {
            state,
            payload: self.payload.clone(),
            limits,
            candidate: candidate.clone(),
        });
    }

    /// Returns the decoded, unique, byte-sorted IDs present in the current SVG.
    pub fn drawing_scene_ids(&self) -> Result<Vec<Vec<u8>>, NativeCommandError<()>> {
        self.cached_scene_ids()
    }

    /// Scene ids of the current source, parsed once per source state.
    fn cached_scene_ids(&self) -> Result<Vec<Vec<u8>>, NativeCommandError<()>> {
        let state = self.document.history_state_id();
        if let Some((cached, ids)) = self.scene_ids.borrow().as_ref() {
            if *cached == state {
                let _phase = phase("scene_copy");
                return copy_scene_ids(ids).map_err(NativeCommandError::Timeline);
            }
        }
        let _phase = phase("scene_scan");
        let source = self
            .document
            .source()
            .map_err(|error| NativeCommandError::Session(SessionError::Document(error)))?;
        let ids = parse_scene_ids(&source).map_err(NativeCommandError::SceneIds)?;
        let copy = copy_scene_ids(&ids).map_err(NativeCommandError::Timeline)?;
        *self.scene_ids.borrow_mut() = Some((state, ids));
        Ok(copy)
    }

    /// Builds and edits a payload candidate, then publishes it as one journal
    /// transition. The closure returns a candidate produced by the owned
    /// `ProjectTimelineCandidate` editing API.
    pub fn edit_timeline<E>(
        &mut self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
        edit: impl FnOnce(&ProjectTimelineCandidate) -> Result<ProjectTimelineCandidate, E>,
    ) -> Result<bool, NativeCommandError<E>> {
        let candidate = {
            let _phase = phase("candidate_open");
            self.timeline_candidate(settings, limits)
        }
        .map_err(map_candidate_error)?;
        let scene_ids = {
            let _phase = phase("scene_copy");
            copy_scene_ids(candidate.scene_ids()).map_err(NativeCommandError::Timeline)?
        };
        if !candidate.is_editable() {
            return Err(NativeCommandError::Timeline(
                ProjectTimelineError::UnsupportedTimeline,
            ));
        }
        let edited = {
            let _phase = phase("edit_apply");
            edit(&candidate)
        }
        .map_err(NativeCommandError::Operation)?;
        // Timeline commands can append imported drawing assets, but cannot
        // remove, reorder or rewrite opaque assets retained by the project.
        if !edited.assets().starts_with(candidate.assets()) {
            return Err(NativeCommandError::ExistingAssetsChanged);
        }
        // Only candidates derived from this input can append assets, and the
        // only append operation is the validated drawing import helper.
        if !edited.same_lineage(&candidate) {
            return Err(NativeCommandError::CandidateLineageChanged);
        }

        // The caller can return a candidate built with another context. Admit
        // only supported data validated against this scene and these limits.
        // A derivative of the opened candidate was already decoded when its
        // edit was committed, so only foreign shapes are re-opened.
        let edited = if edited.is_validated_derivative_of(&candidate, limits) {
            edited
        } else {
            let assets = {
                let _phase = phase("copy_assets");
                copy_assets(edited.assets()).map_err(NativeCommandError::Timeline)?
            };
            let _phase = phase("candidate_reopen");
            ProjectTimelineCandidate::open(
                edited.timeline_bytes(),
                scene_ids,
                assets,
                settings,
                limits,
            )
            .map_err(NativeCommandError::Timeline)?
        };
        if !edited.is_editable() {
            return Err(NativeCommandError::Timeline(
                ProjectTimelineError::UnsupportedTimeline,
            ));
        }

        // Context and edit-budget metadata do not change the saved payload.
        if edited.timeline_bytes() == candidate.timeline_bytes()
            && edited.assets() == candidate.assets()
        {
            return Ok(false);
        }

        let replacement = {
            let _phase = phase("payload_encode");
            self.payload.with_timeline_candidate(&edited)
        }
        .map_err(NativeCommandError::Session)?;
        let admission = {
            let _phase = phase("journal_admit");
            self.journal.prepare_payload(replacement.clone())
        }
        .map_err(|_| NativeCommandError::Session(SessionError::HistoryUnavailable))?;

        // Admission has completed every recoverable allocation/validation.
        self.document.history_boundary();
        self.payload = replacement;
        admission.commit();
        // `edited` was decoded when its last edit was committed and is exactly
        // the new payload, so the next edit need not decode it again.
        self.remember_candidate(self.document.history_state_id(), limits, &edited);
        Ok(true)
    }
}

fn map_candidate_error<E>(error: NativeCommandError<()>) -> NativeCommandError<E> {
    match error {
        NativeCommandError::Session(error) => NativeCommandError::Session(error),
        NativeCommandError::Timeline(error) => NativeCommandError::Timeline(error),
        NativeCommandError::SceneIds(error) => NativeCommandError::SceneIds(error),
        NativeCommandError::CandidateLineageChanged => NativeCommandError::CandidateLineageChanged,
        NativeCommandError::ExistingAssetsChanged => NativeCommandError::ExistingAssetsChanged,
        NativeCommandError::Operation(()) => {
            unreachable!("timeline_candidate has no operation error")
        }
    }
}

fn copy_bytes(source: &[u8]) -> Result<Vec<u8>, ProjectTimelineError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(source.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    bytes.extend_from_slice(source);
    Ok(bytes)
}

fn copy_scene_ids(source: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, ProjectTimelineError> {
    let mut ids = Vec::new();
    ids.try_reserve_exact(source.len())
        .map_err(|_| ProjectTimelineError::OutOfMemory)?;
    for id in source {
        ids.push(copy_bytes(id)?);
    }
    Ok(ids)
}

pub(super) fn copy_assets(
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

fn parse_scene_ids(source: &[u8]) -> Result<Vec<Vec<u8>>, SceneIdsError> {
    if source.len() > vexel_xml::MAX_LARGE_INPUT_BYTES {
        return Err(SceneIdsError::Limit);
    }
    let (tokens, result) = tokenize_large(source);
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return Err(match result.diagnostic.category {
            vexel_xml::XmlError::InputLimit
            | vexel_xml::XmlError::NodeLimit
            | vexel_xml::XmlError::AttributeLimit
            | vexel_xml::XmlError::DepthLimit
            | vexel_xml::XmlError::TextLimit => SceneIdsError::Limit,
            vexel_xml::XmlError::OutputExhausted => SceneIdsError::OutOfMemory,
            _ => SceneIdsError::Malformed,
        });
    }

    let mut ids = Vec::new();
    for token in tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Attribute)
    {
        let name = token.name.slice(source).ok_or(SceneIdsError::Malformed)?;
        if name != b"id" {
            continue;
        }
        if ids.len() >= MAX_SCENE_IDS {
            return Err(SceneIdsError::Limit);
        }
        let raw = token.value.slice(source).ok_or(SceneIdsError::Malformed)?;
        let mut decoded = Vec::new();
        decoded
            .try_reserve_exact(raw.len())
            .map_err(|_| SceneIdsError::OutOfMemory)?;
        decoded.resize(raw.len(), 0);
        let length = decode_attribute_value(source, token.value, &mut decoded)
            .map_err(|_| SceneIdsError::Malformed)?;
        if length == 0 {
            return Err(SceneIdsError::Malformed);
        }
        if length > MAX_SCENE_ID_BYTES {
            return Err(SceneIdsError::Limit);
        }
        decoded.truncate(length);
        ids.try_reserve(1).map_err(|_| SceneIdsError::OutOfMemory)?;
        ids.push(decoded);
    }
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SceneIdsError::Duplicate);
    }
    Ok(ids)
}

#[cfg(test)]
#[path = "native_commands_tests.rs"]
mod tests;
