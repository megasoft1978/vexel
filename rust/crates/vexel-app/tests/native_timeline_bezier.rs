//! Custom cubic-bezier easing in the native timeline: UI intents and the
//! document edit, including clamping, undo and the `custom` picker label.
use vexel_app::native_animation::NativeAnimationController;
use vexel_app::native_timeline::bezier;
use vexel_app::native_timeline::interaction::{InteractionState, Key, Phase};
use vexel_app::native_timeline::layout::{Hit, TimelineLayout, Viewport};
use vexel_app::native_timeline::model::TimelineModel;
use vexel_app::native_timeline::{
    NativeTimelineAction, NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack,
    NativeTrackKind,
};
use vexel_app::session::AppDocument;
use vexel_timeline::sample::Ease;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};
use vexel_ui::Rect;
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

fn key_ease(doc: &AppDocument) -> Option<Ease> {
    let candidate = doc
        .native_timeline_candidate(settings(), EditLimits::default())
        .unwrap();
    let view = candidate.view().unwrap();
    (0..8)
        .filter_map(|i| view.item_at(0, i))
        .find(|item| item.tick == 2)
        .and_then(|item| item.ease)
}

fn doc_with_key() -> AppDocument {
    let mut doc = AppDocument::from_document(V1Document::from_source(SVG).unwrap()).unwrap();
    doc.edit_timeline(settings(), EditLimits::default(), |c| {
        c.add_property_track(
            b"opacity", b"shape", b"opacity", b"scalar", b"linear", b"1.0", b"Opacity",
        )
    })
    .unwrap();
    doc.edit_timeline(settings(), EditLimits::default(), |c| {
        c.set_property_key(b"opacity", 2, b"0.5")
    })
    .unwrap();
    doc
}

fn apply(doc: &mut AppDocument, ease: &str) -> bool {
    let mut controller = NativeAnimationController::default();
    controller
        .handle_action(
            doc,
            NativeTimelineAction::SetPropertyKeyEase {
                track_id: "opacity".into(),
                tick: 2,
                value: "0.5".into(),
                ease: ease.into(),
            },
        )
        .unwrap()
        .changed
}

#[test]
fn applying_custom_bezier_writes_the_curve_and_undo_restores() {
    let mut doc = doc_with_key();
    let before = key_ease(&doc);
    let revision = doc.state_revision();
    assert!(apply(&mut doc, &bezier::encode([0.25, 0.1, 0.25, 1.0])));
    assert_eq!(doc.state_revision(), revision + 1, "one undoable edit");
    assert_eq!(key_ease(&doc), Some(Ease::Bezier([0.25, 0.1, 0.25, 1.0])));
    assert!(doc.undo().unwrap());
    assert_eq!(key_ease(&doc), before);
}

#[test]
fn out_of_range_values_clamp_to_the_validator_limits() {
    let mut doc = doc_with_key();
    assert!(apply(&mut doc, "bezier:-5,9,5,-9"));
    assert_eq!(key_ease(&doc), Some(Ease::Bezier([0.0, 4.0, 1.0, -4.0])));
}

fn snapshot(ease: &str) -> NativeTimelineSnapshot {
    NativeTimelineSnapshot {
        ticks_per_second: 4,
        duration_ticks: 8,
        current_tick: 0,
        tracks: vec![NativeTimelineTrack {
            id: "opacity".into(),
            name: "Opacity".into(),
            target: "shape".into(),
            kind: NativeTrackKind::Property,
            visible: true,
            selected: true,
            items: vec![NativeTimelineItem {
                tick: 2,
                label: "0.5".into(),
                selected: true,
                ease: Some(ease.into()),
            }],
        }],
        selected_target: Some("shape".into()),
        playing: false,
        loop_enabled: false,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
        can_add_drawing_track: true,
        can_import_asset: true,
        can_edit_selected_item: true,
        status: None,
    }
}

fn layout(model: &TimelineModel) -> TimelineLayout {
    TimelineLayout::new(
        Rect {
            x: 0,
            y: 0,
            width: 900,
            height: 260,
        },
        model,
        Viewport::full(model),
    )
}

fn centre(r: Rect) -> (i32, i32) {
    ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32)
}

fn click(
    state: &mut InteractionState,
    l: &TimelineLayout,
    m: &TimelineModel,
    p: (i32, i32),
) -> Vec<NativeTimelineAction> {
    state.pointer(Phase::Down, p, l, m);
    state.pointer(Phase::Up, p, l, m)
}

fn custom_button(l: &TimelineLayout) -> (i32, i32) {
    (
        l.transport.x as i32 + 220 + 5 * 28 + 13,
        l.transport.y as i32 + 16,
    )
}

#[test]
fn picker_labels_bezier_custom_and_presets_by_name() {
    let model = TimelineModel::from_snapshot(&snapshot("bezier:0.25,0.1,0.25,1"));
    let ease = model.selected_item().unwrap().3.ease.as_deref().unwrap();
    assert_eq!(bezier::label(ease), "custom");
    assert_eq!(bezier::label("ease_in"), "ease_in");
    let l = layout(&model);
    assert_eq!(
        l.hit_test(&model, custom_button(&l).0, custom_button(&l).1),
        Hit::EaseCustom
    );
}

#[test]
fn editor_steps_clamp_and_apply_emits_one_edit() {
    let model = TimelineModel::from_snapshot(&snapshot("bezier:0.25,0.1,0.25,1"));
    let l = layout(&model);
    let mut state = InteractionState::default();
    assert!(click(&mut state, &l, &model, custom_button(&l)).is_empty());
    assert!(state.bezier.open);
    assert_eq!(
        state.bezier.values,
        [0.25, 0.1, 0.25, 1.0],
        "starts from the key"
    );

    // x1 minus 10 times clamps at 0; y2 plus 100 times clamps at 4.
    let x1_minus = centre(l.bezier_step_rect(0, false).unwrap());
    let y2_plus = centre(l.bezier_step_rect(3, true).unwrap());
    for _ in 0..10 {
        assert!(click(&mut state, &l, &model, x1_minus).is_empty());
    }
    for _ in 0..100 {
        click(&mut state, &l, &model, y2_plus);
    }
    assert_eq!(state.bezier.values, [0.0, 0.1, 0.25, 4.0]);

    let apply = centre(l.bezier_apply_rect().unwrap());
    let actions = click(&mut state, &l, &model, apply);
    assert_eq!(
        actions,
        vec![NativeTimelineAction::SetPropertyKeyEase {
            track_id: "opacity".into(),
            tick: 2,
            value: "0.5".into(),
            ease: "bezier:0,0.1,0.25,4".into(),
        }]
    );
    assert!(!state.bezier.open);
}

#[test]
fn escape_closes_editor_without_an_edit_and_presets_stay_one_click() {
    let model = TimelineModel::from_snapshot(&snapshot("linear"));
    let l = layout(&model);
    let mut state = InteractionState::default();
    click(&mut state, &l, &model, custom_button(&l));
    assert!(state.bezier.open);
    assert_eq!(state.bezier.values, bezier::DEFAULT);
    assert!(state.key(Key::Escape, &model).is_empty());
    assert!(!state.bezier.open);
    let ease_in = (
        l.transport.x as i32 + 220 + 28 + 13,
        l.transport.y as i32 + 16,
    );
    let actions = click(&mut state, &l, &model, ease_in);
    assert_eq!(actions.len(), 1);
}
