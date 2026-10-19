use super::{MAX_SCENE_ID_BYTES, NativeCommandError, SceneIdsError, parse_scene_ids};
use crate::animation::native_drawing::SvgGeometry;
use crate::project_session::history::PayloadSnapshot;
use crate::project_session::payload::SessionPayload;
use crate::project_session::{ProjectSession, journal_integration};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
use vexel_v1_document::V1Document;

const SCENE: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" viewBox=\"0 0 16 16\"><rect id=\"shape\" width=\"4\" height=\"4\"/></svg>";
const ASSET: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\" viewBox=\"0 0 16 16\"><rect width=\"4\" height=\"4\"/></svg>";

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

fn session() -> ProjectSession {
    ProjectSession::new(V1Document::from_source(SCENE).unwrap()).unwrap()
}

fn geometry() -> SvgGeometry {
    SvgGeometry {
        width: 16.0,
        height: 16.0,
        view_box: [0.0, 0.0, 16.0, 16.0],
    }
}

#[test]
fn scene_ids_are_decoded_sorted_unique_and_bounded() {
    assert_eq!(
        parse_scene_ids(b"<svg><path id='z'/><path id='a&amp;b'/></svg>").unwrap(),
        vec![b"a&b".to_vec(), b"z".to_vec()]
    );
    assert_eq!(
        parse_scene_ids(b"<svg><path id='x'/><path id='x'/></svg>"),
        Err(SceneIdsError::Duplicate)
    );
    assert_eq!(
        parse_scene_ids(b"<svg><path id='x'></svg>"),
        Err(SceneIdsError::Malformed)
    );
    let mut oversized = vec![b'a'; MAX_SCENE_ID_BYTES + 1];
    let source = {
        let mut source = b"<svg><path id='".to_vec();
        source.append(&mut oversized);
        source.extend_from_slice(b"'/></svg>");
        source
    };
    assert_eq!(parse_scene_ids(&source), Err(SceneIdsError::Limit));
}

#[test]
fn session_exposes_current_sorted_ids() {
    let session = ProjectSession::new(
        V1Document::from_source(
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><path id='z'/><path id='a&amp;b'/></svg>",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        session.drawing_scene_ids().unwrap(),
        vec![b"a&b".to_vec(), b"z".to_vec()]
    );
}

#[test]
fn timeline_noop_and_operation_rejection_preserve_payload_and_journal() {
    let mut session = session();
    let identity = session.payload.identity();
    let app_id = session.journal.current().app_state_id();
    let scene_id = session.document.history_state_id();

    assert!(
        !session
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                Ok::<_, ()>(candidate.clone())
            })
            .unwrap()
    );
    assert_eq!(session.payload.identity(), identity);
    assert_eq!(session.journal.current().app_state_id(), app_id);
    assert_eq!(session.document.history_state_id(), scene_id);

    assert_eq!(
        session.edit_timeline(settings(), EditLimits::default(), |_| Err("rejected")),
        Err(NativeCommandError::Operation("rejected"))
    );
    assert_eq!(session.payload.identity(), identity);
    assert_eq!(session.journal.current().app_state_id(), app_id);
    assert_eq!(session.document.history_state_id(), scene_id);
}

#[test]
fn timeline_change_is_one_undoable_payload_transition() {
    let mut session = session();
    let original_identity = session.payload.identity();
    assert!(
        session
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                candidate.set_loop(LoopMode::Repeat).map_err(|error| error)
            })
            .unwrap()
    );
    assert_ne!(session.payload.identity(), original_identity);
    assert_eq!(session.journal.state_count(), 2);
    assert!(session.undo().unwrap());
    assert_eq!(session.payload.identity(), original_identity);
    assert!(session.can_redo());
    assert!(session.redo().unwrap());
    assert_ne!(session.payload.identity(), original_identity);
}

#[test]
fn drawing_import_and_first_track_reference_commit_together() {
    let mut session = session();
    let original_identity = session.payload.identity();
    assert!(
        session
            .edit_timeline(settings(), EditLimits::default(), |candidate| {
                candidate
                    .import_drawing_track(
                        b"drawing.svg",
                        b"image/svg+xml",
                        ASSET,
                        geometry(),
                        b"draw-track",
                        b"shape",
                        b"Drawing",
                        0,
                        b"first",
                    )
                    .map_err(|error| error)
            })
            .unwrap()
    );
    session.payload.with_bundle_input(SCENE, |bundle| {
        assert_eq!(bundle.assets.len(), 1);
        assert_eq!(bundle.assets[0].name, b"drawing.svg");
        assert!(
            bundle
                .timeline_json
                .windows(11)
                .any(|part| part == b"drawing.svg")
        );
        assert!(bundle.timeline_json.windows(5).any(|part| part == b"first"));
    });
    assert_ne!(session.payload.identity(), original_identity);
    assert_eq!(session.journal.state_count(), 2);
    assert!(session.undo().unwrap());
    assert_eq!(session.payload.identity(), original_identity);
}

#[test]
fn unsupported_payload_edit_is_rejected_without_publication() {
    let mut session = session();
    let opaque_timeline = br#"{"schema":"vexel.animation","version":2}"#;
    let input = vexel_project::BundleInput {
        manifest_json: b"{\"format\":\"vexel-project\",\"width\":16,\"height\":16}",
        scene_svg: SCENE,
        timeline_json: opaque_timeline,
        assets: &[],
    };
    let encoded = vexel_project::encode(&input).unwrap();
    let view = vexel_project::decode(&encoded).unwrap();
    session.payload = SessionPayload::try_from_view_with(view, &mut || true).unwrap();
    session.journal =
        journal_integration::new_journal(&session.document, session.payload.clone()).unwrap();
    let identity = session.payload.identity();
    let result = session.edit_timeline::<()>(settings(), EditLimits::default(), |_| {
        panic!("unsupported timeline must fail before operation")
    });
    assert_eq!(
        result,
        Err(NativeCommandError::Timeline(
            super::ProjectTimelineError::UnsupportedTimeline
        ))
    );
    assert_eq!(session.payload.identity(), identity);
    assert_eq!(session.journal.state_count(), 1);
}

// Include journal cursor, next identity, checkpoint and redo branch in every
// rejected-publication comparison, along with the exact editor source.
fn state(session: &ProjectSession) -> (usize, Vec<u8>, u64, usize, usize, String) {
    (
        session.payload.identity(),
        session.document.source().unwrap(),
        session.document.history_state_id(),
        session.document.history_count(),
        session.document.history_cursor(),
        format!("{:?}", session.journal),
    )
}

fn seed_redo(session: &mut ProjectSession) -> u64 {
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::Repeat)
        })
        .unwrap();
    session.journal.checkpoint();
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::PingPong)
        })
        .unwrap();
    let redo = session.journal.current().app_state_id();
    assert!(session.undo().unwrap());
    assert!(!session.journal.is_dirty());
    assert!(session.can_redo());
    redo
}

#[test]
fn returned_candidate_and_edit_budget_rejections_preserve_journal_state() {
    let mut session = session();
    let redo = seed_redo(&mut session);
    let before = state(&session);
    let forged = super::ProjectTimelineCandidate::open(
        b"",
        vec![b"absent".to_vec()],
        vec![],
        settings(),
        EditLimits::default(),
    )
    .unwrap()
    .add_property_track(
        b"foreign", b"absent", b"opacity", b"scalar", b"linear", b"1", b"Foreign",
    )
    .unwrap();
    let result = session.edit_timeline(settings(), EditLimits::default(), |_| Ok::<_, ()>(forged));
    assert_eq!(result, Err(NativeCommandError::CandidateLineageChanged));
    assert_eq!(state(&session), before);

    // The existing payload fits exactly; changing its loop mode exceeds the
    // caller's byte budget during the candidate operation.
    let limits = EditLimits {
        commands: 1,
        bytes: session.payload.timeline_bytes().len(),
    };
    let result = session.edit_timeline(settings(), limits, |candidate| {
        candidate.set_loop(LoopMode::PingPong)
    });
    assert!(matches!(result, Err(NativeCommandError::Operation(_))));
    assert_eq!(state(&session), before);
    assert!(session.redo().unwrap());
    assert_eq!(session.journal.current().app_state_id(), redo);
}

#[test]
fn separately_opened_unsupported_candidate_is_rejected_before_publication() {
    let mut session = session();
    seed_redo(&mut session);
    let before = state(&session);
    for opaque in [
        &b"{}"[..],
        &br#"{"schema":"vexel.animation","version":2}"#[..],
    ] {
        let candidate = super::ProjectTimelineCandidate::open(
            opaque,
            vec![],
            vec![],
            settings(),
            EditLimits::default(),
        )
        .unwrap();
        assert_eq!(
            session.edit_timeline(settings(), EditLimits::default(), |_| Ok::<_, ()>(
                candidate
            )),
            Err(NativeCommandError::CandidateLineageChanged),
        );
        assert_eq!(state(&session), before);
    }
}

#[test]
fn returning_the_unchanged_candidate_is_a_semantic_noop() {
    let mut session = session();
    seed_redo(&mut session);
    let before = state(&session);
    let result = session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            Ok::<_, ()>(candidate.clone())
        })
        .unwrap();
    assert!(!result);
    assert_eq!(state(&session), before);
}

#[test]
fn rejected_operation_and_payload_admission_preserve_redo_and_checkpoint() {
    let mut session = session();
    let redo = seed_redo(&mut session);
    let before = state(&session);
    assert_eq!(
        session.edit_timeline(settings(), EditLimits::default(), |_| Err("rejected")),
        Err(NativeCommandError::Operation("rejected")),
    );
    assert_eq!(state(&session), before);
    crate::project_session::history::fail_next_reservation_for_test();
    assert_eq!(
        session.edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.import_drawing_track(
                b"drawing.svg",
                b"image/svg+xml",
                ASSET,
                geometry(),
                b"draw",
                b"shape",
                b"Drawing",
                0,
                b"first",
            )
        }),
        Err(NativeCommandError::Session(
            crate::project_session::SessionError::HistoryUnavailable
        )),
    );
    assert_eq!(state(&session), before);
    assert!(session.redo().unwrap());
    assert_eq!(session.journal.current().app_state_id(), redo);
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::Once)
        })
        .unwrap();
    assert_eq!(session.journal.current().app_state_id(), redo + 1);
}

#[test]
fn payload_commit_preserves_separate_editor_undo_steps() {
    let mut session = session();
    session
        .edit(|document| document.set_fill("shape", "#112233"))
        .unwrap();
    assert_eq!(session.document.history_count(), 1);
    let before_payload = session.document.source().unwrap();
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.set_loop(LoopMode::Repeat)
        })
        .unwrap();
    let payload = session.payload.identity();
    session
        .edit(|document| document.set_fill("shape", "#334455"))
        .unwrap();
    assert_eq!(session.document.history_count(), 2);
    assert_eq!(session.journal.state_count(), 4);
    assert!(session.undo().unwrap());
    assert_eq!(session.document.source().unwrap(), before_payload);
    assert_eq!(session.payload.identity(), payload);
    assert!(session.undo().unwrap());
    assert_eq!(session.document.source().unwrap(), before_payload);
    assert_ne!(session.payload.identity(), payload);
    assert!(session.undo().unwrap());
    assert_eq!(session.document.source().unwrap(), SCENE);
}

#[test]
fn drawing_import_preserves_manifest_and_all_existing_asset_bytes() {
    let mut session = session();
    let manifest = br#"{ "format":"vexel-project", "width":16, "height":16, "future":[1,2] }"#;
    let assets = [
        vexel_project::Asset {
            name: b"opaque.bin",
            media_type: b"application/octet-stream",
            data: b"\0\xffopaque",
        },
        vexel_project::Asset {
            name: b"existing.svg",
            media_type: b"image/svg+xml",
            data: ASSET,
        },
    ];
    let encoded = vexel_project::encode(&vexel_project::BundleInput {
        manifest_json: manifest,
        scene_svg: SCENE,
        timeline_json: b"",
        assets: &assets,
    })
    .unwrap();
    session.payload =
        SessionPayload::try_from_view_with(vexel_project::decode(&encoded).unwrap(), &mut || true)
            .unwrap();
    session.journal =
        journal_integration::new_journal(&session.document, session.payload.clone()).unwrap();
    session
        .edit_timeline(settings(), EditLimits::default(), |candidate| {
            candidate.import_drawing_track(
                b"drawing.svg",
                b"image/svg+xml",
                ASSET,
                geometry(),
                b"draw",
                b"shape",
                b"Drawing",
                0,
                b"first",
            )
        })
        .unwrap();
    session.payload.with_bundle_input(SCENE, |bundle| {
        assert_eq!(bundle.manifest_json, manifest);
        assert_eq!(bundle.scene_svg, SCENE);
        assert_eq!(bundle.assets.len(), 3);
        assert_eq!(&bundle.assets[..2], &assets);
        assert_eq!(
            bundle.assets[2],
            vexel_project::Asset {
                name: b"drawing.svg",
                media_type: b"image/svg+xml",
                data: ASSET,
            }
        );
    });
}

#[test]
fn loaded_empty_object_stays_opaque_and_byte_exact() {
    let mut session = session();
    let encoded = vexel_project::encode(&vexel_project::BundleInput {
        manifest_json: br#"{"format":"vexel-project","width":16,"height":16}"#,
        scene_svg: SCENE,
        timeline_json: b"{}",
        assets: &[],
    })
    .unwrap();
    session.payload =
        SessionPayload::try_from_view_with(vexel_project::decode(&encoded).unwrap(), &mut || true)
            .unwrap();
    session.journal =
        journal_integration::new_journal(&session.document, session.payload.clone()).unwrap();
    let before = state(&session);
    assert_eq!(
        session.edit_timeline::<()>(settings(), EditLimits::default(), |_| panic!(
            "opaque payload"
        )),
        Err(NativeCommandError::Timeline(
            super::ProjectTimelineError::UnsupportedTimeline
        )),
    );
    assert_eq!(session.payload.timeline_bytes(), b"{}");
    assert_eq!(state(&session), before);
}

#[test]
fn returned_candidate_cannot_drop_or_modify_existing_opaque_assets() {
    let mut session = session();
    let assets = [
        vexel_project::Asset {
            name: b"opaque.bin",
            media_type: b"application/octet-stream",
            data: b"\0\xffopaque",
        },
        vexel_project::Asset {
            name: b"second.bin",
            media_type: b"application/octet-stream",
            data: b"second opaque asset",
        },
    ];
    let encoded = vexel_project::encode(&vexel_project::BundleInput {
        manifest_json: br#"{"format":"vexel-project","width":16,"height":16}"#,
        scene_svg: SCENE,
        timeline_json: b"",
        assets: &assets,
    })
    .unwrap();
    session.payload =
        SessionPayload::try_from_view_with(vexel_project::decode(&encoded).unwrap(), &mut || true)
            .unwrap();
    session.journal =
        journal_integration::new_journal(&session.document, session.payload.clone()).unwrap();
    seed_redo(&mut session);
    let before = state(&session);
    for modification in 0..5 {
        let result = session.edit_timeline(settings(), EditLimits::default(), |candidate| {
            let mut assets = candidate.assets().to_vec();
            match modification {
                0 => assets.clear(),
                1 => assets[0].source.push(b'!'),
                2 => assets[0].media_type = b"application/x-changed".to_vec(),
                3 => assets[0].name = b"renamed.bin".to_vec(),
                _ => assets.swap(0, 1),
            }
            super::ProjectTimelineCandidate::open(
                candidate.timeline_bytes(),
                vec![b"shape".to_vec()],
                assets,
                settings(),
                EditLimits::default(),
            )
        });
        assert_eq!(result, Err(NativeCommandError::ExistingAssetsChanged));
        assert_eq!(state(&session), before);
    }
}
