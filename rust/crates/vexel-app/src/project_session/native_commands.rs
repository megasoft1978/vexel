//! Atomic native-timeline commands for project sessions.

use super::svg_asset::OwnedProjectAsset;
use super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};
use super::{ProjectSession, SessionError};
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

impl ProjectSession {
    /// Returns a validated view of the current timeline and its owned assets
    /// without publishing a journal transition. Opaque timeline bytes remain
    /// available through the non-editable candidate.
    pub fn timeline_candidate(
        &self,
        settings: EmptyTimelineSettings,
        limits: EditLimits,
    ) -> Result<ProjectTimelineCandidate, NativeCommandError<()>> {
        let source = self
            .document
            .source()
            .map_err(|error| NativeCommandError::Session(SessionError::Document(error)))?;
        let scene_ids = parse_scene_ids(&source).map_err(NativeCommandError::SceneIds)?;
        self.payload
            .timeline_candidate(scene_ids, settings, limits)
            .map_err(NativeCommandError::Timeline)
    }

    /// Returns the decoded, unique, byte-sorted IDs present in the current SVG.
    pub fn drawing_scene_ids(&self) -> Result<Vec<Vec<u8>>, NativeCommandError<()>> {
        let source = self
            .document
            .source()
            .map_err(|error| NativeCommandError::Session(SessionError::Document(error)))?;
        parse_scene_ids(&source).map_err(NativeCommandError::SceneIds)
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
        let candidate = self
            .timeline_candidate(settings, limits)
            .map_err(map_candidate_error)?;
        let scene_ids =
            copy_scene_ids(candidate.scene_ids()).map_err(NativeCommandError::Timeline)?;
        if !candidate.is_editable() {
            return Err(NativeCommandError::Timeline(
                ProjectTimelineError::UnsupportedTimeline,
            ));
        }
        let edited = edit(&candidate).map_err(NativeCommandError::Operation)?;
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
        let edited = ProjectTimelineCandidate::open(
            edited.timeline_bytes(),
            scene_ids,
            copy_assets(edited.assets()).map_err(NativeCommandError::Timeline)?,
            settings,
            limits,
        )
        .map_err(NativeCommandError::Timeline)?;
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

        let replacement = self
            .payload
            .with_timeline_candidate(&edited)
            .map_err(NativeCommandError::Session)?;
        let admission = self
            .journal
            .prepare_payload(replacement.clone())
            .map_err(|_| NativeCommandError::Session(SessionError::HistoryUnavailable))?;

        // Admission has completed every recoverable allocation/validation.
        self.document.history_boundary();
        self.payload = replacement;
        admission.commit();
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
