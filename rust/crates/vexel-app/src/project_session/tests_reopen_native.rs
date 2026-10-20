//! Save/reopen coverage for native timelines mixed with drawing edits.

use super::ProjectSession;
use crate::animation::native_drawing::{SVG_MEDIA_TYPE, SvgGeometry};
use std::{
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
use vexel_v1_document::V1Document;

const ANIM: &[u8] = b"<animate attributeName=\"x\" values=\"10;20\" dur=\"1s\"/>";
const DRAWING: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><rect width="4" height="4"/></svg>"#;

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-reopen-native-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

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

fn geometry() -> SvgGeometry {
    SvgGeometry {
        width: 16.0,
        height: 16.0,
        view_box: [0.0, 0.0, 16.0, 16.0],
    }
}

fn fresh_session() -> ProjectSession {
    let mut doc = V1Document::new(200, 100).unwrap();
    doc.add_rectangle_with_id(10.0, 10.0, 60.0, 50.0, "#ff0000", "r1")
        .unwrap();
    ProjectSession::new(doc).unwrap()
}

/// Owned copy of the observable native timeline: exact payload bytes plus
/// the decoded track and key structure.
#[derive(Debug, PartialEq, Eq)]
struct TimelineSnapshot {
    bytes: Vec<u8>,
    tracks: Vec<(Vec<u8>, Vec<u8>, usize)>,
    keys: Vec<(u32, Option<Vec<u8>>)>,
    asset_names: Vec<Vec<u8>>,
    asset_sources: Vec<Vec<u8>>,
}

fn timeline_snapshot(session: &ProjectSession) -> TimelineSnapshot {
    let candidate = session
        .timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    let view = candidate.view().unwrap();
    let mut tracks = Vec::new();
    let mut keys = Vec::new();
    for track_index in 0..view.track_count() {
        let track = view.track_at(track_index).unwrap();
        tracks.push((track.id.to_vec(), track.target.to_vec(), track.item_count));
        for item_index in 0..track.item_count {
            let item = view.item_at(track_index, item_index).unwrap();
            keys.push((item.tick, item.value_json.map(<[u8]>::to_vec)));
        }
    }
    TimelineSnapshot {
        bytes: candidate.timeline_bytes().to_vec(),
        tracks,
        keys,
        asset_names: candidate.assets().iter().map(|a| a.name.clone()).collect(),
        asset_sources: candidate
            .assets()
            .iter()
            .map(|a| a.source.clone())
            .collect(),
    }
}

/// Whole-document state: scene bytes plus timeline snapshot.
#[derive(Debug, PartialEq, Eq)]
struct State {
    scene: Vec<u8>,
    timeline: TimelineSnapshot,
}

fn state(session: &ProjectSession) -> State {
    State {
        scene: session.document().source().unwrap(),
        timeline: timeline_snapshot(session),
    }
}

#[test]
fn reopen_native_timeline_preserves_tracks_keys_and_drawing_asset() {
    let dir = Dir::new();
    let path = dir.file("native.vexel");
    let mut session = fresh_session();

    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.add_property_track(
                b"opacity", b"r1", b"opacity", b"scalar", b"linear", b"1", b"Opacity",
            )
        })
        .unwrap();
    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.set_property_key(b"opacity", 12, b"0.5")
        })
        .unwrap();
    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.set_property_key(b"opacity", 48, b"0.25")
        })
        .unwrap();
    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.import_drawing_track(
                b"drawing.svg",
                SVG_MEDIA_TYPE,
                DRAWING,
                geometry(),
                b"draw",
                b"r1",
                b"Drawing",
                0,
                b"first",
            )
        })
        .unwrap();
    session
        .edit(|doc| doc.add_rectangle_with_id(70.0, 10.0, 20.0, 20.0, "#00ff00", "r2"))
        .unwrap();

    let before = state(&session);
    assert_eq!(before.timeline.tracks.len(), 2);
    assert_eq!(before.timeline.asset_names, vec![b"drawing.svg".to_vec()]);
    assert_eq!(before.timeline.asset_sources, vec![DRAWING.to_vec()]);
    session.save_as(bytes(&path)).unwrap();
    assert!(!session.is_dirty());

    let reopened = ProjectSession::open(bytes(&path)).unwrap();
    assert!(!reopened.is_dirty());
    assert_eq!(reopened.document().source().unwrap(), before.scene);
    let after = timeline_snapshot(&reopened);
    assert_eq!(after.bytes, before.timeline.bytes);
    assert_eq!(after.tracks, before.timeline.tracks);
    assert_eq!(after.keys, before.timeline.keys);
    assert_eq!(after.asset_names, before.timeline.asset_names);
    assert_eq!(after.asset_sources, before.timeline.asset_sources);
    assert_eq!(state(&reopened), before);
}

#[test]
fn reopen_native_timeline_undo_order_is_preserved_across_draw_and_animation_edits() {
    let dir = Dir::new();
    let path = dir.file("order.vexel");
    let mut session = fresh_session();

    // Alternating scene and payload edits: each is a separate undo step.
    let mut history = vec![state(&session)];
    session
        .edit(|doc| doc.add_rectangle_with_id(70.0, 10.0, 20.0, 20.0, "#00ff00", "r2"))
        .unwrap();
    history.push(state(&session));
    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.add_property_track(
                b"opacity", b"r1", b"opacity", b"scalar", b"linear", b"1", b"Opacity",
            )
        })
        .unwrap();
    history.push(state(&session));
    session
        .edit(|doc| doc.add_animation_fragment("r1", ANIM))
        .unwrap();
    history.push(state(&session));
    session
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.set_property_key(b"opacity", 12, b"0.5")
        })
        .unwrap();
    history.push(state(&session));
    session
        .edit(|doc| doc.add_ellipse_with_id(90.0, 20.0, 150.0, 90.0, "#0000ff", "e1"))
        .unwrap();
    let latest = state(&session);

    // Undo walks the exact reverse chronology.
    for expected in history.iter().rev() {
        assert!(session.undo().unwrap());
        assert_eq!(&state(&session), expected);
    }
    assert!(!session.can_undo());

    // Redo replays forward to the latest state.
    for _ in 0..history.len() {
        assert!(session.redo().unwrap());
    }
    assert_eq!(state(&session), latest);
    assert!(!session.can_redo());

    // Save and reopen: the v1 bundle has no history section, so the reopened
    // session starts with empty history while keeping the saved state.
    session.save_as(bytes(&path)).unwrap();
    let mut reopened = ProjectSession::open(bytes(&path)).unwrap();
    assert!(!reopened.can_undo());
    assert!(!reopened.can_redo());
    assert_eq!(reopened.undo(), Ok(false));
    assert_eq!(state(&reopened), latest);

    // Undo order is still correct for edits made after reopening.
    let reopened_base = state(&reopened);
    reopened
        .edit(|doc| doc.add_rectangle_with_id(100.0, 40.0, 10.0, 10.0, "#123456", "r3"))
        .unwrap();
    let post_draw = state(&reopened);
    reopened
        .edit_timeline(settings(), EditLimits::default(), |c| {
            c.set_property_key(b"opacity", 24, b"0.75")
        })
        .unwrap();
    assert!(reopened.undo().unwrap());
    assert_eq!(state(&reopened), post_draw);
    assert!(reopened.undo().unwrap());
    assert_eq!(state(&reopened), reopened_base);
    assert_eq!(
        reopened.undo(),
        Ok(false),
        "post-reopen history must not reach pre-save edits"
    );
    assert_eq!(state(&reopened), reopened_base);
}
