use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use vexel_app::animation::native_driver::NativeDriver;
use vexel_app::animation::native_stage::{NativeStage, OnionSettings, StageUpdate};
use vexel_app::animation::playback::ManualClock;
use vexel_app::compose::{svg_viewport, Frame};
use vexel_app::native_animation::{
    import, NativeAnimationController, NativeAnimationHost, NativeFrameResult, NativeFrameTag,
    NativeRenderRequest, NativeTimelineEffect, TimelineSurface,
};
use vexel_app::native_timeline::interaction::Key;
use vexel_app::native_timeline::{NativeTimelineAction, NativeTrackKind};
use vexel_app::session::AppDocument;
use vexel_platform_apple::{
    PanelBackend, PanelResult, RecoveryChoice, RecoveryText, UnsavedChoice, UnsavedText,
};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
use vexel_v1_document::V1Document;

const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="24" viewBox="0 0 32 24"><rect id="shape" x="2" y="2" width="12" height="10" fill="#f00"/></svg>"##;

fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: 4,
        duration_ticks: 8,
        fps_numerator: 4,
        fps_denominator: 1,
        loop_mode: LoopMode::Repeat,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

fn document() -> AppDocument {
    AppDocument::from_document(V1Document::from_source(SVG).unwrap()).unwrap()
}

fn action(
    controller: &mut NativeAnimationController,
    doc: &mut AppDocument,
    action: NativeTimelineAction,
) -> vexel_app::native_animation::NativeActionOutcome {
    controller.handle_action(doc, action).unwrap()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-native-integration-{}-{}",
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

struct Panel(PanelResult);

impl PanelBackend for Panel {
    fn open_panel(&self, _: &[&str]) -> PanelResult {
        self.0.clone()
    }

    fn save_panel(&self, _: &str) -> PanelResult {
        PanelResult::Cancelled
    }

    fn confirm_unsaved(&self, _: &UnsavedText<'_>) -> UnsavedChoice {
        UnsavedChoice::Cancel
    }

    fn confirm_recovery(&self, _: &RecoveryText<'_>) -> RecoveryChoice {
        RecoveryChoice::Cancel
    }

    fn show_error(&self, _: &str, _: &str) {}
}

#[test]
fn atomic_native_action_changes_payload_revision_once_and_noop_does_not() {
    let mut doc = document();
    let mut controller = NativeAnimationController::default();
    controller.set_surface(TimelineSurface::Native);
    let source_revision = doc.revision();
    let state_revision = doc.state_revision();

    let outcome = action(
        &mut controller,
        &mut doc,
        NativeTimelineAction::SetLoop { enabled: false },
    );
    assert!(outcome.changed);
    assert_eq!(doc.revision(), source_revision);
    assert_eq!(doc.state_revision(), state_revision + 1);
    assert!(doc.is_dirty());

    let candidate = doc
        .native_timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    assert_eq!(candidate.view().unwrap().loop_mode(), LoopMode::Once);
    let changed = doc.state_revision();
    assert!(!doc
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::Once)
        })
        .unwrap());
    assert_eq!(doc.state_revision(), changed);
}

#[test]
fn mixed_scene_and_payload_undo_redo_follow_one_chronology() {
    let temp = TempDir::new();
    let path = temp.file("mixed-history.vexel");
    let mut doc = document();
    doc.save_as(path.as_os_str().as_bytes()).unwrap();
    let original = doc.source_bytes().unwrap();
    doc.edit(|scene| scene.add_rectangle_with_id(1.0, 1.0, 4.0, 4.0, "#123456", "second"))
        .unwrap();
    let scene_edit = doc.source_bytes().unwrap();
    let mut controller = NativeAnimationController::default();
    controller
        .handle_action(&mut doc, NativeTimelineAction::SetLoop { enabled: false })
        .unwrap();

    assert!(doc.undo().unwrap());
    assert_eq!(doc.source_bytes().unwrap(), scene_edit);
    assert_eq!(
        doc.native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .loop_mode(),
        LoopMode::Repeat
    );
    assert!(doc.undo().unwrap());
    assert_eq!(doc.source_bytes().unwrap(), original);
    assert!(doc.redo().unwrap());
    assert_eq!(doc.source_bytes().unwrap(), scene_edit);
    assert!(doc.redo().unwrap());
    assert_eq!(
        doc.native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .loop_mode(),
        LoopMode::Once
    );
}

#[test]
fn payload_only_edit_is_seen_by_recovery_and_save_reopen() {
    let temp = TempDir::new();
    let path = temp.file("timeline.vexel");
    let mut doc = document();
    doc.save_as(path.as_os_str().as_bytes()).unwrap();
    let backend = Panel(PanelResult::Cancelled);
    let mut files = vexel_app::file_ui::FileUi::with_size(backend, (32, 24)).unwrap();
    files.with_lifecycle(|document, _| {
        document.open_path(&path).unwrap();
        document
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                candidate.set_loop(LoopMode::Once)
            })
            .unwrap();
    });
    let doc = files.document();
    assert_eq!(doc.revision(), 1);
    assert_eq!(doc.state_revision(), 2);
    assert_eq!(
        files.tick_recovery(Instant::now()),
        Some(vexel_app::file_ui::RecoveryStatusMessage::NewerEditsPending)
    );

    let doc = files.document_mut();
    doc.save().unwrap();
    let mut reopened = AppDocument::new(1, 1).unwrap();
    reopened.open_path(&path).unwrap();
    assert_eq!(
        reopened
            .native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .loop_mode(),
        LoopMode::Once
    );
}

#[test]
fn import_cancel_and_viewport_mismatch_leave_journal_unchanged() {
    let temp = TempDir::new();
    let doc = document();
    let source_revision = doc.revision();
    let state_revision = doc.state_revision();
    let effect = NativeTimelineEffect::ImportSvg {
        target_id: "shape".into(),
        tick: 0,
    };

    let mut cancelled =
        vexel_app::file_ui::FileUi::with_size(Panel(PanelResult::Cancelled), (32, 24)).unwrap();
    assert_eq!(
        import::handle(&mut cancelled, &effect, settings(), EditLimits::default()),
        import::ImportResult::Cancelled
    );
    assert_eq!(cancelled.document().state_revision(), 0);

    let bad_svg = temp.file("wrong-geometry.svg");
    fs::write(
        &bad_svg,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20"/>"#,
    )
    .unwrap();
    let mut mismatch = vexel_app::file_ui::FileUi::with_size(
        Panel(PanelResult::Path(bad_svg.as_os_str().as_bytes().to_vec())),
        (32, 24),
    )
    .unwrap();
    assert_eq!(
        import::handle(&mut mismatch, &effect, settings(), EditLimits::default()),
        import::ImportResult::Failed
    );
    assert_eq!(mismatch.document().state_revision(), 0);
    assert_eq!(doc.revision(), source_revision);
    assert_eq!(doc.state_revision(), state_revision);
}

#[test]
fn unsupported_timeline_payload_stays_byte_exact_and_authoring_is_disabled() {
    let temp = TempDir::new();
    let input = temp.file("opaque.vexel");
    let output = temp.file("opaque-copy.vexel");
    let scene = V1Document::from_source(SVG).unwrap();
    let scene_svg = scene.source().unwrap();
    let manifest = br#"{"format":"vexel-project","extension":{"keep":true}}"#;
    let timeline = br#"{ "schema":"vexel.animation", "version":2, "vendor":[1, {"x":"keep"}] }"#;
    let assets = [vexel_project::Asset {
        name: b"opaque.bin",
        media_type: b"application/octet-stream",
        data: b"\0\xffopaque",
    }];
    let encoded = vexel_project::encode(&vexel_project::BundleInput {
        manifest_json: manifest,
        scene_svg: &scene_svg,
        timeline_json: timeline,
        assets: &assets,
    })
    .unwrap();
    fs::write(&input, encoded).unwrap();

    let mut doc = AppDocument::new(1, 1).unwrap();
    doc.open_path(&input).unwrap();
    let original_revision = doc.state_revision();
    let mut controller = NativeAnimationController::default();
    assert!(controller.snapshot(&doc).unwrap().tracks.is_empty());
    assert!(controller
        .handle_action(&mut doc, NativeTimelineAction::SetLoop { enabled: false },)
        .is_err());
    assert_eq!(doc.state_revision(), original_revision);
    doc.save_as(output.as_os_str().as_bytes()).unwrap();

    let loaded = vexel_project_io::project_file_load(output.as_os_str().as_bytes()).unwrap();
    let view = loaded.view();
    assert_eq!(view.manifest_json, manifest);
    assert_eq!(view.timeline_json, timeline);
    assert_eq!(view.asset_count, assets.len());
    assert_eq!(view.asset_at(0), Some(assets[0]));
}

#[test]
fn escaped_timeline_strings_are_decoded_for_snapshot_and_commands() {
    let temp = TempDir::new();
    let input = temp.file("escaped-timeline.vexel");
    let scene = V1Document::from_source(SVG).unwrap();
    let scene_svg = scene.source().unwrap();
    let timeline = br#"{"schema":"vexel.animation","version":1,"ticks_per_second":4,"duration_ticks":8,"fps":{"numerator":4,"denominator":1},"loop":"loop","onion":{"before":0,"after":0,"opacity":0.25},"tracks":[{"id":"draw\u005fone","kind":"draw\u0069ng","target":"sh\u0061pe","name":"Dr\u0061w","visible":true,"cels":[{"tick":0,"asset":"a\u002esvg","label":"c\u0065l"}]},{"id":"opacity\u005f0","kind":"propert\u0079","target":"sh\u0061pe","name":"Op\u0061city","visible":true,"property":"opacity","value_type":"scalar","interpolation":"linear","keys":[{"tick":0,"value":1.0}]}]}"#;
    let assets = [vexel_project::Asset {
        name: b"a.svg",
        media_type: b"image/svg+xml",
        data: b"<svg/>",
    }];
    let encoded = vexel_project::encode(&vexel_project::BundleInput {
        manifest_json: b"{}",
        scene_svg: &scene_svg,
        timeline_json: timeline,
        assets: &assets,
    })
    .unwrap();
    fs::write(&input, encoded).unwrap();

    let mut doc = AppDocument::new(1, 1).unwrap();
    doc.open_path(&input).unwrap();
    let mut controller = NativeAnimationController::default();
    let snapshot = controller.snapshot(&doc).unwrap();
    assert_eq!(snapshot.tracks.len(), 2);
    assert_eq!(snapshot.tracks[0].id, "draw_one");
    assert_eq!(snapshot.tracks[0].name, "Draw");
    assert_eq!(snapshot.tracks[0].target, "shape");
    assert_eq!(snapshot.tracks[0].kind, NativeTrackKind::Drawing);
    assert_eq!(snapshot.tracks[0].items[0].label, "a.svg");
    assert_eq!(snapshot.tracks[1].id, "opacity_0");
    assert_eq!(snapshot.tracks[1].kind, NativeTrackKind::Property);

    let outcome = action(
        &mut controller,
        &mut doc,
        NativeTimelineAction::SetTrackVisibility {
            track_id: "draw_one".into(),
            visible: false,
        },
    );
    assert!(outcome.changed);
    assert!(!controller.snapshot(&doc).unwrap().tracks[0].visible);

    let opacity = action(
        &mut controller,
        &mut doc,
        NativeTimelineAction::AddOpacityKey {
            target_id: "shape".into(),
            tick: 1,
            value: 0.5,
        },
    );
    assert!(opacity.changed);
    let snapshot = controller.snapshot(&doc).unwrap();
    assert_eq!(snapshot.tracks[2].id, "opacity_1");
    assert_eq!(snapshot.tracks[2].kind, NativeTrackKind::Property);
}

#[test]
fn property_key_edits_and_track_ops_round_trip_through_document_history() {
    let mut doc = document();
    doc.edit_timeline(settings(), EditLimits::default(), |candidate| {
        candidate.add_property_track(
            b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1.0", b"Opacity",
        )
    })
    .unwrap();
    let mut controller = NativeAnimationController::default();
    let initial_candidate = doc
        .native_timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    let initial = initial_candidate.view().unwrap();
    let start_revision = doc.state_revision();
    assert!(
        action(
            &mut controller,
            &mut doc,
            NativeTimelineAction::MovePropertyKey {
                track_id: "opacity".into(),
                from_tick: 0,
                to_tick: 2
            }
        )
        .changed
            == false
    );
    assert_eq!(
        doc.state_revision(),
        start_revision,
        "tick zero cannot move"
    );
    assert!(
        action(
            &mut controller,
            &mut doc,
            NativeTimelineAction::SetPropertyKeyEase {
                track_id: "opacity".into(),
                tick: 0,
                value: "1.0".into(),
                ease: "step".into()
            }
        )
        .changed
            == false
    );
    let before = doc.state_revision();
    assert!(
        action(
            &mut controller,
            &mut doc,
            NativeTimelineAction::DuplicateTrack {
                track_id: "opacity".into()
            }
        )
        .changed
    );
    assert_eq!(doc.state_revision(), before + 1);
    assert!(doc.undo().unwrap());
    assert_eq!(
        doc.native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .track_count(),
        initial.track_count()
    );
    assert!(doc.redo().unwrap());
    assert_eq!(
        doc.native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .track_count(),
        initial.track_count() + 1
    );
    let before = doc.state_revision();
    assert!(
        action(
            &mut controller,
            &mut doc,
            NativeTimelineAction::RemoveTrack {
                track_id: "opacity_copy1".into()
            }
        )
        .changed
    );
    assert_eq!(doc.state_revision(), before + 1);
    assert!(doc.undo().unwrap());
    assert_eq!(
        doc.native_timeline_candidate(settings(), EditLimits::default())
            .unwrap()
            .view()
            .unwrap()
            .track_count(),
        initial.track_count() + 1
    );
}

#[test]
fn stale_timeline_target_disables_snapshot_but_keeps_mode_switch_available() {
    let mut files =
        vexel_app::file_ui::FileUi::with_size(Panel(PanelResult::Cancelled), (32, 24)).unwrap();
    let document = files.document_mut();
    document
        .edit(|scene| scene.add_rectangle_with_id(2.0, 2.0, 12.0, 10.0, "#ff0000", "shape"))
        .unwrap();
    document
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.add_property_track(
                b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1.0", b"Opacity",
            )
        })
        .unwrap();
    document.edit(|scene| scene.delete("shape")).unwrap();
    let revision = files.document().state_revision();

    let mut host = NativeAnimationHost::new(ManualClock::default());
    let snapshot = host
        .snapshot(files.document(), Some("shape".into()))
        .unwrap();
    assert!(snapshot.tracks.is_empty());
    assert!(!snapshot.can_edit_selected_item);
    assert!(snapshot
        .status
        .as_deref()
        .is_some_and(|status| status.contains("authoring is disabled")));
    assert_eq!(files.document().state_revision(), revision);

    assert!(host
        .key(Key::Tab, &mut files, Some("shape".into()))
        .unwrap());
    assert!(host
        .key(Key::Right, &mut files, Some("shape".into()))
        .unwrap());
    assert_eq!(host.surface(), TimelineSurface::Smil);
    assert_eq!(files.document().state_revision(), revision);
}

fn request(doc: &AppDocument, tick: u32) -> NativeRenderRequest {
    let source_svg = Arc::new(doc.source_bytes().unwrap());
    let viewport = svg_viewport(source_svg.clone(), &|| false).unwrap();
    NativeRenderRequest {
        state_revision: doc.state_revision(),
        tick,
        output_width: 32,
        output_height: 24,
        source_svg,
        timeline: doc
            .native_timeline_candidate(settings(), EditLimits::default())
            .unwrap(),
        viewport,
    }
}

#[test]
fn native_driver_samples_integer_tick_boundaries_and_rejects_stale_frames() {
    let doc = document();
    let mut driver = NativeDriver::new(ManualClock::default());
    let mut stage = NativeStage::new();
    let onion = OnionSettings {
        before: 0,
        after: 0,
        opacity: 0.25,
    };

    let first = driver.tick(request(&doc, 0)).unwrap();
    assert_eq!(first.tag.tick, 0);
    stage.expect(first.tag, onion);
    assert_eq!(stage.accept(first), StageUpdate::Published);
    assert!(driver.play());
    driver.playback().clock().advance(0.249);
    assert!(driver.tick(request(&doc, 0)).is_none());
    driver.playback().clock().advance(0.001);
    assert_eq!(driver.tick(request(&doc, 0)).unwrap().tag.tick, 1);

    let stale = NativeFrameResult {
        tag: NativeFrameTag {
            state_revision: 0,
            tick: 0,
            width: 32,
            height: 24,
        },
        result: Ok(Frame {
            width: 32,
            height: 24,
            rgba: vec![0; 32 * 24 * 4],
            disclosures: Vec::new(),
        }),
    };
    stage.expect(
        NativeFrameTag {
            state_revision: 1,
            tick: 1,
            width: 32,
            height: 24,
        },
        onion,
    );
    assert_eq!(stage.accept(stale), StageUpdate::Stale);
    assert!(stage.frame().is_some());
}
