//! The add-key command writes the curve's current value at the playhead.

use vexel_app::native_animation::NativeAnimationController;
use vexel_app::native_timeline::NativeTimelineAction;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode, PropertyValue};
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

/// An opacity track fading 1 -> 0 over ticks 0..4.
fn fading() -> AppDocument {
    let mut doc = AppDocument::from_document(V1Document::from_source(SVG).unwrap()).unwrap();
    doc.edit_timeline(settings(), EditLimits::default(), |candidate| {
        candidate
            .add_property_track(
                b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1.0", b"Opacity",
            )?
            .set_property_key(b"opacity", 4, b"0.0")
    })
    .unwrap();
    doc
}

fn samples(doc: &AppDocument) -> Vec<f64> {
    let candidate = doc
        .native_timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    let view = candidate.view().unwrap();
    (0..8)
        .map(
            |tick| match vexel_timeline::sample_properties(&view, tick).unwrap()[0].value {
                PropertyValue::Scalar(v) => v,
                other => panic!("{other:?}"),
            },
        )
        .collect()
}

fn add(
    controller: &mut NativeAnimationController,
    doc: &mut AppDocument,
    track_id: Option<&str>,
    target_id: Option<&str>,
    tick: u32,
) -> bool {
    controller
        .handle_action(
            doc,
            NativeTimelineAction::AddKey {
                track_id: track_id.map(str::to_owned),
                target_id: target_id.map(str::to_owned),
                tick,
            },
        )
        .unwrap()
        .changed
}

#[test]
fn key_between_keys_splits_the_curve_without_changing_the_animation() {
    let mut doc = fading();
    let before = samples(&doc);
    let revision = doc.state_revision();
    let mut controller = NativeAnimationController::default();
    assert!(add(&mut controller, &mut doc, Some("opacity"), None, 2));
    assert_eq!(doc.state_revision(), revision + 1, "one edit");
    assert_eq!(samples(&doc), before);
    let snapshot = controller.snapshot(&doc).unwrap();
    let track = &snapshot.tracks[0];
    assert_eq!(
        track.items.iter().map(|item| item.tick).collect::<Vec<_>>(),
        [0, 2, 4]
    );
    assert_eq!(track.items[1].label, "0.5");
    assert!(track.items[1].selected);
    assert_eq!(snapshot.current_tick, 2);
    assert!(doc.undo().unwrap());
    assert_eq!(samples(&doc), before);
}

#[test]
fn key_on_an_existing_key_changes_nothing() {
    let mut doc = fading();
    let revision = doc.state_revision();
    let mut controller = NativeAnimationController::default();
    assert!(!add(&mut controller, &mut doc, Some("opacity"), None, 4));
    assert_eq!(doc.state_revision(), revision);
}

#[test]
fn shape_without_a_track_starts_an_opacity_track_and_a_shape_uses_its_first_track() {
    let mut doc = AppDocument::from_document(V1Document::from_source(SVG).unwrap()).unwrap();
    let mut controller = NativeAnimationController::default();
    assert!(add(&mut controller, &mut doc, None, Some("shape"), 3));
    let snapshot = controller.snapshot(&doc).unwrap();
    assert_eq!(snapshot.tracks.len(), 1);
    assert_eq!(snapshot.tracks[0].property, "opacity");
    assert_eq!(snapshot.tracks[0].items.len(), 2);
    assert!(add(&mut controller, &mut doc, None, Some("shape"), 5));
    let snapshot = controller.snapshot(&doc).unwrap();
    assert_eq!(snapshot.tracks.len(), 1, "no second track");
    assert_eq!(snapshot.tracks[0].items.len(), 3);
}

#[test]
fn unknown_track_is_refused() {
    let mut doc = fading();
    let mut controller = NativeAnimationController::default();
    let result = controller.handle_action(
        &mut doc,
        NativeTimelineAction::AddKey {
            track_id: Some("missing".into()),
            target_id: None,
            tick: 2,
        },
    );
    assert!(result.is_err());
}
