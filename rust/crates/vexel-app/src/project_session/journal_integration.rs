//! Connects the bounded journal to the source-owning V1 editor.

use super::edit_error::SessionEditError;
use super::history::{GroupToken, Journal, JournalError};
use super::payload::SessionPayload;
use super::{ProjectSession, SessionError};
use vexel_v1_document::V1Document;

pub(super) fn new_journal(
    document: &V1Document,
    payload: SessionPayload,
) -> Result<Journal<SessionPayload>, SessionError> {
    Journal::try_new(document.history_state_id(), payload).map_err(map_journal_error)
}

pub(super) fn edit<R, E>(
    session: &mut ProjectSession,
    edit: impl FnOnce(&mut V1Document) -> Result<R, E>,
) -> Result<R, SessionEditError<E>> {
    let admission = session
        .journal
        .prepare_scene()
        .map_err(|error| SessionEditError::History(map_journal_error(error)))?;
    let result = edit(&mut session.document);
    let final_scene_id = session.document.history_state_id();
    admission.finish(final_scene_id, |scene_id| {
        is_scene_reachable(&session.document, scene_id)
    });
    result.map_err(SessionEditError::Operation)
}

pub(super) fn begin_group(session: &mut ProjectSession) -> Result<GroupToken, SessionError> {
    session.journal.begin_group().map_err(map_journal_error)
}

pub(super) fn end_group(
    session: &mut ProjectSession,
    token: GroupToken,
) -> Result<(), SessionError> {
    session.journal.end_group(token).map_err(map_journal_error)
}

/// Restores the group-start scene and payload, then drops the group's states.
pub(super) fn rollback_group(
    session: &mut ProjectSession,
    token: GroupToken,
) -> Result<(), SessionError> {
    let target = (0..=session.document.history_count())
        .filter_map(|cursor| session.document.history_state_id_at(cursor))
        .find_map(|scene_id| {
            session
                .journal
                .rollback_target(token, std::slice::from_ref(&scene_id))
                .map(|t| (t.scene_state_id(), t.payload().clone()))
        });
    let Some((scene_state_id, payload)) = target else {
        session.journal.abandon_group();
        return Err(SessionError::HistoryUnavailable);
    };
    session
        .document
        .history_navigate(scene_state_id)
        .map_err(SessionError::Document)?;
    session.document.history_boundary();
    session.payload = payload;
    session
        .journal
        .commit_rollback(token)
        .map_err(map_journal_error)
}

pub(super) fn undo(session: &mut ProjectSession) -> Result<bool, SessionError> {
    if session.journal.group_open() {
        return Ok(false);
    }
    let Some((app_state_id, scene_state_id, payload)) = undo_target(session) else {
        return Ok(false);
    };
    session
        .document
        .history_navigate(scene_state_id)
        .map_err(SessionError::Document)?;
    session.document.history_boundary();
    session.payload = payload;
    session
        .journal
        .commit_undo(app_state_id)
        .expect("journal cursor cannot change during an exclusive undo");
    Ok(true)
}

pub(super) fn redo(session: &mut ProjectSession) -> Result<bool, SessionError> {
    if session.journal.group_open() {
        return Ok(false);
    }
    let Some((app_state_id, scene_state_id, payload)) = redo_target(session) else {
        return Ok(false);
    };
    session
        .document
        .history_navigate(scene_state_id)
        .map_err(SessionError::Document)?;
    session.document.history_boundary();
    session.payload = payload;
    session
        .journal
        .commit_redo(app_state_id)
        .expect("journal cursor cannot change during an exclusive redo");
    Ok(true)
}

pub(super) fn can_undo(session: &ProjectSession) -> bool {
    undo_target(session).is_some()
}

pub(super) fn can_redo(session: &ProjectSession) -> bool {
    redo_target(session).is_some()
}

fn undo_target(session: &ProjectSession) -> Option<(u64, u64, SessionPayload)> {
    for cursor in 0..=session.document.history_count() {
        let Some(scene_id) = session.document.history_state_id_at(cursor) else {
            continue;
        };
        if let Some(target) = session.journal.undo_target(std::slice::from_ref(&scene_id)) {
            return Some((
                target.app_state_id(),
                target.scene_state_id(),
                target.payload().clone(),
            ));
        }
    }
    None
}

fn redo_target(session: &ProjectSession) -> Option<(u64, u64, SessionPayload)> {
    for cursor in 0..=session.document.history_count() {
        let Some(scene_id) = session.document.history_state_id_at(cursor) else {
            continue;
        };
        if let Some(target) = session.journal.redo_target(std::slice::from_ref(&scene_id)) {
            return Some((
                target.app_state_id(),
                target.scene_state_id(),
                target.payload().clone(),
            ));
        }
    }
    None
}

fn is_scene_reachable(document: &V1Document, scene_id: u64) -> bool {
    (0..=document.history_count())
        .any(|cursor| document.history_state_id_at(cursor) == Some(scene_id))
}

fn map_journal_error(_: JournalError) -> SessionError {
    SessionError::HistoryUnavailable
}
