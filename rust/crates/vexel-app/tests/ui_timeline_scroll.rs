//! Vertical scrolling of the native timeline track list: wheel, thumb,
//! reveal of a canvas-selected track and hit-testing at an offset. Writes a
//! screenshot to `VEXEL_SHOTS_DIR` (default `/private/tmp/vexel-w8-shots`).
use vexel_app::model::{Mode, Model};
use vexel_app::native_timeline::interaction::{InteractionState, Phase};
use vexel_app::native_timeline::layout::{Hit, TRACK_HEIGHT, TimelineLayout};
use vexel_app::native_timeline::model::TimelineModel;
use vexel_app::native_timeline::presentation::TimelinePresentation;
use vexel_app::native_timeline::{
    NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind,
};
use vexel_app::render::render;
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::Rect;

fn snapshot(count: usize, selected: Option<usize>) -> NativeTimelineSnapshot {
    let tracks = (0..count)
        .map(|i| NativeTimelineTrack {
            id: format!("track-{i}"),
            name: format!("Joint {i}"),
            target: format!("part-{i}"),
            property: "rotate".into(),
            kind: NativeTrackKind::Property,
            visible: true,
            selected: selected == Some(i),
            items: vec![
                NativeTimelineItem {
                    tick: 0,
                    label: "0".into(),
                    selected: false,
                    ease: Some("linear".into()),
                },
                NativeTimelineItem {
                    tick: 10 + (i as u32 % 20),
                    label: "1".into(),
                    selected: false,
                    ease: Some("ease_in_out".into()),
                },
            ],
        })
        .collect();
    NativeTimelineSnapshot {
        ticks_per_second: 24,
        duration_ticks: 48,
        current_tick: 6,
        tracks,
        selected_target: None,
        playing: false,
        loop_enabled: false,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
        can_add_drawing_track: true,
        can_import_asset: true,
        can_edit_selected_item: false,
        status: None,
    }
}

fn panel() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 900,
        height: 220,
    }
}

fn setup(count: usize, selected: Option<usize>) -> (TimelineModel, InteractionState) {
    let snap = snapshot(count, selected);
    let mut state = InteractionState::default();
    state.note_selection(&snap.tracks);
    (TimelineModel::from_snapshot(&snap), state)
}

fn name_x() -> i32 {
    100
}

fn row_y(layout: &TimelineLayout, slot: u32) -> i32 {
    (layout.rows.y + slot * TRACK_HEIGHT + 10) as i32
}

#[test]
fn wheel_scrolls_whole_rows_clamped_and_hit_testing_follows_the_offset() {
    let (model, mut state) = setup(136, None);
    let layout = state.layout(panel(), &model);
    assert_eq!(layout.first_row, 0);
    let visible = layout.visible_rows();
    assert!((3..30).contains(&visible), "{visible}");
    assert_eq!(
        layout.hit_test(&model, name_x(), row_y(&layout, 0)),
        Hit::Track(0)
    );
    // Negative points move to later tracks, positive ones back (as in the SMIL panel).
    assert!(state.scroll_points(-f64::from(TRACK_HEIGHT) * 5.0, &layout, 136));
    let layout = state.layout(panel(), &model);
    assert_eq!(layout.first_row, 5);
    assert_eq!(
        layout.hit_test(&model, name_x(), row_y(&layout, 0)),
        Hit::Track(5)
    );
    assert_eq!(
        layout.hit_test(&model, name_x(), row_y(&layout, 2)),
        Hit::Track(7)
    );
    // A key of track 5 is hit in the first drawn row; its old row 0 is gone.
    let item = layout.item_rect(&model, 5, 1).unwrap();
    assert_eq!(
        layout.hit_test(
            &model,
            (item.x + item.width / 2) as i32,
            (item.y + item.height / 2) as i32
        ),
        Hit::Item { track: 5, item: 1 }
    );
    assert_eq!(layout.row_rect(0).height, 0, "scrolled out of the band");
    // Far past the end clamps to the last full page; and back to the top.
    assert!(state.scroll_points(-1.0e6, &layout, 136));
    let end = state.layout(panel(), &model);
    assert_eq!(end.first_row, 136 - visible);
    assert!(
        !state.scroll_points(-100.0, &end, 136),
        "already at the end"
    );
    assert!(state.scroll_points(1.0e6, &end, 136));
    assert_eq!(state.layout(panel(), &model).first_row, 0);
}

#[test]
fn thumb_exists_only_when_tracks_overflow_and_tracks_the_offset() {
    let (few, state) = setup(3, None);
    assert!(state.layout(panel(), &few).scroll_thumb(3).is_none());
    let (model, mut state) = setup(136, None);
    let top = state.layout(panel(), &model);
    let thumb = top.scroll_thumb(136).expect("overflowing list has a thumb");
    assert!(thumb.y >= top.rows.y && thumb.y + thumb.height <= top.rows.y + top.rows.height);
    assert_eq!(thumb.x + thumb.width + 2, 900);
    state.scroll_points(-1.0e6, &top, 136);
    let bottom = state.layout(panel(), &model);
    let low = bottom.scroll_thumb(136).unwrap();
    assert!(low.y > thumb.y);
    assert_eq!(low.height, thumb.height);
    assert!(low.y + low.height <= bottom.rows.y + bottom.rows.height);
}

#[test]
fn a_track_selected_from_the_canvas_is_scrolled_into_view_once() {
    let (model, mut state) = setup(136, Some(100));
    let layout = state.layout(panel(), &model);
    let visible = layout.visible_rows();
    assert_eq!(layout.first_row, 100 + 1 - visible, "bottom edge shows it");
    assert!(layout.row_rect(100).height > 0);
    // A pointer move commits the scroll, so a later wheel starts from it.
    state.pointer(Phase::Move, (500, 5), &layout, &model);
    assert!(state.reveal.is_none());
    assert!(state.scroll_points(f64::from(TRACK_HEIGHT) * 4.0, &layout, 136));
    let up = state.layout(panel(), &model);
    assert_eq!(up.first_row, layout.first_row - 4);
    // The same selection arriving again does not pull the list back.
    state.note_selection(&snapshot(136, Some(100)).tracks);
    assert_eq!(state.layout(panel(), &model).first_row, up.first_row);
    // A different selection above the window scrolls up just enough.
    state.note_selection(&snapshot(136, Some(2)).tracks);
    let model = TimelineModel::from_snapshot(&snapshot(136, Some(2)));
    assert_eq!(state.layout(panel(), &model).first_row, 2);
}

#[test]
fn shot_of_a_long_walk_cycle_list() {
    let size = (1280, 800);
    let mut model = Model::new(true);
    model.mode = Mode::Animate;
    model.ui.set_footer_height(300, 600);
    let footer = model.layout(size.0, size.1).unwrap().footer.unwrap();
    let raw = render(size.0, size.1, 1, &model).unwrap();
    let snap = snapshot(136, Some(70));
    let mut pres = TimelinePresentation::default();
    pres.interaction.note_selection(&snap.tracks);
    let frame = vexel_app::overlay::paint_over(raw, true, |c| pres.paint(c, footer, &snap));
    let tm = TimelineModel::from_snapshot(&snap);
    let layout = pres.interaction.layout(footer, &tm);
    assert!(layout.row_rect(70).height > 0, "selected track is visible");
    let dir = std::env::var("VEXEL_SHOTS_DIR").unwrap_or("/private/tmp/vexel-w8-shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let (w, px) = (frame.width(), frame.pixels());
    let mut out = Vec::new();
    for y in footer.y..footer.y + footer.height {
        let a = ((y * w + footer.x) * 4) as usize;
        out.extend_from_slice(&px[a..a + (footer.width * 4) as usize]);
    }
    let png = encode_rgba8(footer.width, footer.height, &out, Compression::Fixed).unwrap();
    std::fs::write(format!("{dir}/timeline-scrolled.png"), png).unwrap();
}
