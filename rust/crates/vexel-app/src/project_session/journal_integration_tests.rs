use super::*;
use std::{
    fs,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 24,
        duration_ticks: 240,
        fps_numerator: 24,
        fps_denominator: 1,
        loop_mode: LoopMode::Once,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.5,
    }
}

fn edit_loop(session: &mut ProjectSession, mode: LoopMode) -> bool {
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(mode)
        })
        .unwrap()
}

fn journal_snapshot(session: &ProjectSession) -> String {
    format!("{:?}", session.journal)
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-journal-integration-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bytes(path: &std::path::Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

#[test]
fn scene_admission_failure_does_not_run_edit_or_change_combined_history() {
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    assert!(edit_loop(&mut session, LoopMode::Repeat));
    session.undo().unwrap();
    let source = session.document().source().unwrap();
    let payload = session.payload.timeline_bytes().to_vec();
    let before = journal_snapshot(&session);
    let mut called = false;

    history::fail_next_reservation_for_test();
    let result = session.edit(|document| {
        called = true;
        document.add_rectangle(1.0, 1.0, 10.0, 10.0, "#123456")
    });

    assert!(matches!(
        result,
        Err(SessionEditError::History(SessionError::HistoryUnavailable))
    ));
    assert!(
        !called,
        "the editor closure must not run after admission fails"
    );
    assert_eq!(session.document().source().unwrap(), source);
    assert_eq!(session.payload.timeline_bytes(), payload);
    assert_eq!(journal_snapshot(&session), before);
    assert!(session.can_redo());
}

#[test]
fn partial_scene_error_is_recorded_and_remains_undoable() {
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    let before = session.document().source().unwrap();
    let result = session.edit(|document| {
        document
            .add_rectangle(1.0, 1.0, 10.0, 10.0, "#123456")
            .unwrap();
        Err::<(), _>("failed after mutation")
    });

    assert!(matches!(
        result,
        Err(SessionEditError::Operation("failed after mutation"))
    ));
    assert_ne!(session.document().source().unwrap(), before);
    assert!(session.can_undo());
    session.undo().unwrap();
    assert_eq!(session.document().source().unwrap(), before);
    assert!(session.can_redo());
}

#[test]
fn scene_and_payload_states_undo_and_redo_in_chronological_order() {
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    let initial_scene = session.document().source().unwrap();
    let initial_payload = session.payload.timeline_bytes().to_vec();

    assert!(edit_loop(&mut session, LoopMode::Repeat));
    let repeat_payload = session.payload.timeline_bytes().to_vec();
    session
        .edit(|document| document.add_rectangle(1.0, 1.0, 10.0, 10.0, "#123456"))
        .unwrap();
    let edited_scene = session.document().source().unwrap();
    assert!(edit_loop(&mut session, LoopMode::Once));
    let final_payload = session.payload.timeline_bytes().to_vec();

    assert!(session.undo().unwrap());
    assert_eq!(session.document().source().unwrap(), edited_scene);
    assert_eq!(session.payload.timeline_bytes(), repeat_payload);
    assert!(session.undo().unwrap());
    assert_eq!(session.document().source().unwrap(), initial_scene);
    assert_eq!(session.payload.timeline_bytes(), repeat_payload);
    assert!(session.undo().unwrap());
    assert_eq!(session.payload.timeline_bytes(), initial_payload);

    assert!(session.redo().unwrap());
    assert_eq!(session.payload.timeline_bytes(), repeat_payload);
    assert!(session.redo().unwrap());
    assert_eq!(session.document().source().unwrap(), edited_scene);
    assert_eq!(session.payload.timeline_bytes(), repeat_payload);
    assert!(session.redo().unwrap());
    assert_eq!(session.payload.timeline_bytes(), final_payload);
    assert!(!session.can_redo());
}

#[test]
fn payload_noop_preserves_redo_and_payload_edit_branches_history() {
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    assert!(edit_loop(&mut session, LoopMode::Repeat));
    let repeat_payload = session.payload.timeline_bytes().to_vec();
    session.undo().unwrap();
    let before_noop = journal_snapshot(&session);

    assert!(!edit_loop(&mut session, LoopMode::Once));
    assert_eq!(journal_snapshot(&session), before_noop);
    assert!(session.can_redo());
    assert!(session.redo().unwrap());
    assert_eq!(session.payload.timeline_bytes(), repeat_payload);

    session.undo().unwrap();
    assert!(edit_loop(&mut session, LoopMode::PingPong));
    let branch = session.payload.timeline_bytes().to_vec();
    assert_ne!(branch, repeat_payload);
    assert!(!session.can_redo());
}

#[test]
fn save_checkpoints_combined_scene_and_payload_state() {
    let dir = TempDir::new();
    let path = dir.file("combined.vexel");
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    session.save_as(bytes(&path)).unwrap();
    assert!(!session.is_dirty());

    assert!(edit_loop(&mut session, LoopMode::Repeat));
    session
        .edit(|document| document.add_rectangle(1.0, 1.0, 10.0, 10.0, "#123456"))
        .unwrap();
    assert!(session.is_dirty());
    session.save().unwrap();
    assert!(!session.is_dirty());

    assert!(edit_loop(&mut session, LoopMode::Once));
    assert!(session.is_dirty());
    session.undo().unwrap();
    assert!(
        !session.is_dirty(),
        "undo returns to the combined saved state"
    );
    session.undo().unwrap();
    assert!(
        session.is_dirty(),
        "undo past the checkpoint changes the scene"
    );
}

#[test]
fn failed_open_and_recovery_replacement_preserve_live_history() {
    let dir = TempDir::new();
    let missing = dir.file("missing.vexel");
    let mut session = ProjectSession::new(V1Document::new(80, 60).unwrap()).unwrap();
    session
        .edit(|document| document.add_rectangle(1.0, 1.0, 10.0, 10.0, "#123456"))
        .unwrap();
    let source = session.document().source().unwrap();
    let before = journal_snapshot(&session);

    assert!(session.open_replacing(bytes(&missing)).is_err());
    assert_eq!(session.document().source().unwrap(), source);
    assert_eq!(journal_snapshot(&session), before);

    let home = dir.file("recovery-home");
    fs::create_dir_all(&home).unwrap();
    let project_path = dir.file("recovery.vexel");
    with_recovery_test_home(&home, || {
        session.save_as(bytes(&project_path)).unwrap();
        session
            .edit(|document| document.add_ellipse(20.0, 20.0, 30.0, 30.0, "#abcdef"))
            .unwrap();
        session.capture_recovery(10).unwrap();
        session
            .edit(|document| document.add_rectangle(40.0, 40.0, 50.0, 50.0, "#fedcba"))
            .unwrap();
        let live_source = session.document().source().unwrap();
        let live_journal = journal_snapshot(&session);

        history::fail_next_reservation_for_test();
        assert_eq!(
            session.restore_recovery(),
            Err(SessionError::HistoryUnavailable)
        );
        assert_eq!(session.document().source().unwrap(), live_source);
        assert_eq!(journal_snapshot(&session), live_journal);
    });
}
