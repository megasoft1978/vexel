use super::timeline_edit::{ProjectTimelineCandidate, ProjectTimelineError};
use crate::animation::native_drawing::{SVG_MEDIA_TYPE, SvgGeometry};
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const GEOMETRY: SvgGeometry = SvgGeometry {
    width: 1.0,
    height: 1.0,
    view_box: [0.0, 0.0, 1.0, 1.0],
};
const DRAWING: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#;

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

#[test]
fn empty_payload_is_initialized_from_explicit_settings() {
    let candidate = ProjectTimelineCandidate::open(
        b"",
        vec![b"shape".to_vec()],
        Vec::new(),
        settings(),
        EditLimits::default(),
    )
    .unwrap();
    assert!(candidate.is_editable());
    assert!(!candidate.timeline_bytes().is_empty());
    let duration = b"\"duration_ticks\":240";
    assert!(
        candidate
            .timeline_bytes()
            .windows(duration.len())
            .any(|window| window == duration)
    );
}

#[test]
fn unsupported_nonempty_payload_is_preserved_and_cannot_be_edited() {
    let future = br#"{ "schema" : "vexel.animation", "version" : 2, "future": true }"#;
    let candidate = ProjectTimelineCandidate::open(
        future,
        Vec::new(),
        Vec::new(),
        settings(),
        EditLimits::default(),
    )
    .unwrap();
    assert!(!candidate.is_editable());
    assert_eq!(candidate.timeline_bytes(), future);
    assert_eq!(
        candidate.set_loop(LoopMode::Repeat),
        Err(ProjectTimelineError::UnsupportedTimeline)
    );
}

#[test]
fn edits_return_new_candidate_and_invalid_edit_does_not_publish_partial_state() {
    let initial = ProjectTimelineCandidate::open(
        b"",
        vec![b"shape".to_vec()],
        Vec::new(),
        settings(),
        EditLimits::default(),
    )
    .unwrap();
    let edited = initial
        .add_property_track(
            b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1", b"Opacity",
        )
        .unwrap();
    assert_ne!(initial.timeline_bytes(), edited.timeline_bytes());
    assert_eq!(initial.assets().len(), 0);

    let failed = initial.import_drawing_track(
        b"drawing.svg",
        SVG_MEDIA_TYPE,
        DRAWING,
        GEOMETRY,
        b"draw",
        b"shape",
        b"Drawing",
        240,
        b"late",
    );
    assert!(failed.is_err());
    assert!(initial.assets().is_empty());
    assert!(
        !initial
            .timeline_bytes()
            .windows(5)
            .any(|window| window == b"draw\"")
    );
}

#[test]
fn drawing_asset_and_initial_timeline_reference_publish_together() {
    let initial = ProjectTimelineCandidate::open(
        b"",
        vec![b"shape".to_vec()],
        Vec::new(),
        settings(),
        EditLimits::default(),
    )
    .unwrap();
    let committed = initial
        .import_drawing_track(
            b"drawing.svg",
            SVG_MEDIA_TYPE,
            DRAWING,
            GEOMETRY,
            b"draw",
            b"shape",
            b"Drawing",
            0,
            b"first",
        )
        .unwrap();
    assert_eq!(initial.assets().len(), 0);
    assert_eq!(committed.assets()[0].source, DRAWING);
    assert!(
        committed
            .timeline_bytes()
            .windows(b"drawing.svg".len())
            .any(|window| window == b"drawing.svg")
    );
}

#[test]
fn candidate_visibility_edit_is_source_preserving_and_decodable() {
    let initial = ProjectTimelineCandidate::open(
        b"",
        vec![b"shape".to_vec()],
        Vec::new(),
        settings(),
        EditLimits::default(),
    )
    .unwrap()
    .add_property_track(
        b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1", b"Opacity",
    )
    .unwrap();
    let track = b"\"visible\":true";
    assert!(
        initial
            .timeline_bytes()
            .windows(track.len())
            .any(|part| part == track)
    );

    let hidden = initial.set_track_visibility(b"opacity", false).unwrap();
    assert!(
        hidden
            .timeline_bytes()
            .windows(b"\"visible\":false".len())
            .any(|part| part == b"\"visible\":false")
    );
    assert!(initial.view().is_ok());
    assert!(hidden.view().is_ok());
    assert!(
        initial
            .timeline_bytes()
            .windows(track.len())
            .any(|part| part == track)
    );
}
