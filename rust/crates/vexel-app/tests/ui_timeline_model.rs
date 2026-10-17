//! WO-RUST-UI-TIMELINE-01 acceptance 1 to 3: model, layout and time mapping.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::chrome::{NO_ANIMATION, NO_DOCUMENT_TO_ANIMATE};
use vexel_app::timeline::Timeline;
use vexel_app::timeline::layout::{NAME_W, ROW_H, TimelineLayout, label_text};

#[test]
fn animated_fixtures_have_tracks_with_sorted_keys_inside_the_loop() {
    for src in [LOOP, LOOP2, LONG] {
        let d = driver_with(src);
        let tl = timeline_of(src, &d);
        let loop_s = tl.model.loop_seconds;
        assert!(loop_s > 0.0);
        assert!(!tl.model.elements.is_empty());
        assert_eq!(tl.model.empty_state(), None);
        for element in &tl.model.elements {
            assert!(!element.props.is_empty());
            for prop in &element.props {
                assert!(!prop.keys.is_empty(), "{}", prop.name);
                assert!(prop.keys.windows(2).all(|w| w[0] <= w[1]));
                assert!(prop.keys.iter().all(|k| (0.0..=loop_s).contains(k)));
            }
        }
    }
}

#[test]
fn element_and_property_names_come_from_the_document() {
    let d = driver_with(LOOP);
    let tl = timeline_of(LOOP, &d);
    let names: Vec<_> = tl
        .model
        .elements
        .iter()
        .map(|e| (e.name.as_str(), e.props[0].name.as_str()))
        .collect();
    assert_eq!(names, [("box", "x"), ("dot", "opacity")]);
    assert_eq!(tl.model.elements[0].props[0].keys, [0.0, 0.5, 1.0]);
}

#[test]
fn static_and_missing_documents_show_the_existing_empty_states() {
    let d = driver_with(STATIC);
    let tl = timeline_of(STATIC, &d);
    assert!(tl.model.elements.is_empty());
    assert_eq!(tl.model.empty_state().as_deref(), Some(NO_ANIMATION));
    assert_eq!(
        Timeline::default().model.empty_state().as_deref(),
        Some(NO_DOCUMENT_TO_ANIMATE)
    );
}

#[test]
fn layout_bands_never_overlap_and_stay_inside_for_all_sizes() {
    let mut widths: Vec<u32> = (260..=3840).step_by(37).collect();
    widths.push(3840);
    for width in widths {
        for height in (120..=600).step_by(23).chain([600]) {
            let rect = rect(7, 11, width, height);
            let l = TimelineLayout::new(rect);
            let bottom = rect.y + rect.height;
            let bands = [l.transport, l.ruler, l.tracks];
            for b in bands {
                assert!(b.x >= rect.x && b.x + b.width <= rect.x + rect.width);
                assert!(
                    b.y >= rect.y && b.y + b.height <= bottom,
                    "{width}x{height}"
                );
            }
            assert!(l.transport.y + l.transport.height <= l.ruler.y);
            assert!(l.ruler.y + l.ruler.height <= l.tracks.y);
            assert!(l.name_w <= width * 2 / 5 && l.name_w <= NAME_W);
            assert!(l.lane_x1 > l.lane_x0);
            for (_, b) in l.buttons() {
                assert!(b.width >= 28 && b.height >= 28, "button >= 28 px");
                assert!(b.x >= rect.x && b.x + b.width <= rect.x + rect.width);
                assert!(
                    b.y >= l.transport.y && b.y + b.height <= l.transport.y + l.transport.height
                );
            }
            for slot in 0..l.visible_rows() {
                let row = l.row_rect(slot);
                assert!(row.y + row.height <= l.tracks.y + l.tracks.height);
                assert_eq!(row.height, ROW_H);
            }
        }
    }
}

#[test]
fn time_x_mapping_round_trips_and_the_playhead_clamps_to_the_ruler() {
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for width in [260, 800, 1280, 3840] {
        let l = TimelineLayout::new(rect(0, 0, width, 200));
        for _ in 0..1000 {
            let loop_s = 0.5 + next() * 9.5;
            let t = next() * loop_s;
            let back = l.x_to_time(l.time_to_x(t, loop_s), loop_s);
            assert!((back - t).abs() < 1.0 / 48.0, "{t} -> {back}");
        }
        assert_eq!(l.playhead_x(-5.0, 2.0), l.lane_x0);
        assert_eq!(l.playhead_x(99.0, 2.0), l.lane_x1);
    }
}

#[test]
fn ruler_labels_never_overlap_and_fit_in_the_panel() {
    for width in (260..=3840).step_by(53) {
        for loop_s in [0.5, 1.0, 2.0, 4.0, 10.0, 90.0, 600.0] {
            let rect = rect(0, 0, width, 200);
            let l = TimelineLayout::new(rect);
            let cells: Vec<_> = l
                .ticks(loop_s)
                .into_iter()
                .filter_map(|t| t.label)
                .collect();
            for (text, cell) in &cells {
                assert!(cell.x + cell.width <= rect.x + rect.width, "{text}");
                assert!(cell.y + cell.height <= l.ruler.y + l.ruler.height);
            }
            for pair in cells.windows(2) {
                let (a, b) = (pair[0].1, pair[1].1);
                assert!(a.x + a.width <= b.x, "{width} {loop_s}: labels overlap");
            }
        }
    }
    assert_eq!(label_text(2.0), "2s");
    assert_eq!(label_text(75.0), "1:15");
}

#[test]
fn major_ticks_are_one_second_apart_when_there_is_room() {
    let l = TimelineLayout::new(rect(0, 0, 1280, 200));
    let majors: Vec<_> = l
        .ticks(4.0)
        .into_iter()
        .filter(|t| t.major)
        .map(|t| t.time)
        .collect();
    assert_eq!(majors, [0.0, 1.0, 2.0, 3.0, 4.0]);
    let minors = l.ticks(4.0).into_iter().filter(|t| !t.major).count();
    assert_eq!(minors, 12);
}
