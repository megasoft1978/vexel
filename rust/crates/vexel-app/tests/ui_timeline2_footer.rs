//! WO-RUST-UI-TIMELINE-02 acceptance 2 to 5: footer scroll cue and clamps,
//! ruler density, diamond contrast, transport clusters and disabled tooltips.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::layout::{FOOTER_MIN, Layout, STAGE_MIN_H, UiState};
use vexel_app::model::Mode;
use vexel_app::timeline::layout::{Hit, ROW_H, THUMB_W, TimelineLayout};
use vexel_app::timeline::model::{KeyRef, Row};
use vexel_app::timeline::{Phase, Timeline};
use vexel_ui::{Rect, Theme, theme_tokens};

const TWELVE: &[u8] = include_bytes!("../../../fixtures/ui/timeline2/twelve.svg");
const SIZE: (u32, u32) = (1280, 800);

fn lum(c: u32) -> f64 {
    let lin = |shift: u32| {
        let v = f64::from((c >> shift) & 255) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(24) + 0.7152 * lin(16) + 0.0722 * lin(8)
}

fn contrast(a: u32, b: u32) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn rgb_u32(c: [u8; 3]) -> u32 {
    (u32::from(c[0]) << 24) | (u32::from(c[1]) << 16) | (u32::from(c[2]) << 8) | 0xff
}

fn xorshift(seed: u64) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---- 2: footer scroll ---------------------------------------------------------

#[test]
fn twelve_rows_in_six_show_a_proportional_thumb_that_tracks_scroll() {
    let d = driver_with(TWELVE);
    let mut tl = timeline_of(TWELVE, &d);
    assert_eq!(tl.rows().len(), 12);
    let l = TimelineLayout::new(rect(54, 500, 900, 224));
    assert_eq!(l.visible_rows(), 6);
    let area = l.tracks.height - 4;
    let max = l.max_scroll(12);
    assert_eq!(max, 6);
    let mut last_y = 0;
    for scroll in 0..=max {
        let thumb = l.scroll_thumb(12, scroll).expect("overflow shows a thumb");
        assert_eq!(thumb.width, THUMB_W);
        let want_len = area * 6 / 12;
        assert!(thumb.height.abs_diff(want_len) <= 1, "{}", thumb.height);
        assert!(thumb.y >= l.tracks.y && thumb.y + thumb.height <= l.tracks.y + l.tracks.height);
        // Position is linear in scroll: top at 0, bottom at the last row.
        let want_y = l.tracks.y + 2 + (area - thumb.height) * scroll as u32 / max as u32;
        assert_eq!(thumb.y, want_y);
        assert!(scroll == 0 || thumb.y > last_y);
        last_y = thumb.y;
        // It sits in the name column edge.
        assert!(thumb.x + thumb.width <= l.rect.x + l.name_w);
        assert!(thumb.x + 8 >= l.rect.x + l.name_w);
    }
    tl.scroll = max;
    assert!(
        l.scroll_thumb(12, max).unwrap().y + l.scroll_thumb(12, max).unwrap().height
            > l.tracks.y + l.tracks.height - 4
    );
    // No thumb when everything fits.
    assert!(l.scroll_thumb(6, 0).is_none());
    assert!(l.scroll_thumb(3, 0).is_none());
}

#[test]
fn the_thumb_is_painted_in_the_text_tertiary_colour_only_when_rows_overflow() {
    for dark in [true, false] {
        let tk = tokens(dark);
        let f = footer(SIZE);
        let l = TimelineLayout::new(f);
        let d = driver_with(TWELVE);
        let mut tl = timeline_of(TWELVE, &d);
        tl.scroll = 2;
        let shot = frame(SIZE, dark, 1, tl.clone());
        let thumb = l
            .scroll_thumb(12, 2)
            .expect("12 rows overflow the 1280x800 footer");
        let (cx, cy) = (thumb.x + thumb.width / 2, thumb.y + thumb.height / 2);
        // POLISH-02: the thumb is 6 px wide in text.tertiary (was 3 px in control.edge).
        assert_eq!(
            pixel(&shot, 1, cx, cy),
            rgb(tk.text_tertiary),
            "dark={dark}"
        );
        let d2 = driver_with(LOOP);
        let few = timeline_of(LOOP, &d2);
        let shot = frame(SIZE, dark, 1, few);
        assert_ne!(
            pixel(&shot, 1, cx, cy),
            rgb(tk.control_edge),
            "no thumb when rows fit"
        );
    }
}

#[test]
fn wheel_over_the_whole_footer_scrolls_and_clamps_and_never_reaches_the_stage() {
    let d = driver_with(TWELVE);
    let f = rect(44, 576, 936, 200);
    let l = TimelineLayout::new(f);
    let stage = Layout::new(1280, 800, Mode::Animate).unwrap().stage;
    let probes = [
        (l.lane_x0 as i32 + 40, (l.tracks.y + 20) as i32),
        ((f.x + 20) as i32, (l.tracks.y + 20) as i32),
        ((f.x + 300) as i32, (l.transport.y + 4) as i32),
        ((f.x + 300) as i32, (l.ruler.y + 4) as i32),
    ];
    let max = l.max_scroll(12);
    for (x, y) in probes {
        let outside_stage = x < stage.x as i32
            || y < stage.y as i32
            || x >= (stage.x + stage.width) as i32
            || y >= (stage.y + stage.height) as i32;
        assert!(
            outside_stage,
            "footer points are never on the stage ({x},{y})"
        );
        let mut tl = timeline_of(TWELVE, &d);
        // Natural scrolling: negative dy moves the rows up (scroll grows).
        assert_eq!(tl.wheel(f, (x, y), -3.0 * f64::from(ROW_H)), Some(true));
        assert_eq!(tl.scroll, 3);
        assert_eq!(tl.wheel(f, (x, y), -100.0 * f64::from(ROW_H)), Some(true));
        assert_eq!(tl.scroll, max, "clamped at the end");
        assert_eq!(tl.wheel(f, (x, y), -50.0), Some(false));
        assert_eq!(tl.wheel(f, (x, y), 100.0 * f64::from(ROW_H)), Some(true));
        assert_eq!(tl.scroll, 0, "clamped at the start");
    }
    // Over the stage the footer does not take the event.
    let mut tl = timeline_of(TWELVE, &d);
    assert_eq!(tl.wheel(f, (400, 200), -100.0), None);
    assert_eq!(
        tl.wheel(f, (f.x as i32 - 1, (f.y + 10) as i32), -100.0),
        None
    );
}

#[test]
fn footer_drags_keep_the_footer_and_the_stage_inside_their_minimums() {
    let mut next = xorshift(0xDEAD_BEEF_1234_5678);
    for _ in 0..1000 {
        let w = 720 + (next() * 800.0) as u32;
        let h = 480 + (next() * 700.0) as u32;
        let mut ui = UiState::default();
        let layout = Layout::with_state(w, h, Mode::Animate, &ui).unwrap();
        let footer = layout.footer.unwrap();
        let splitter = layout.footer_splitter.unwrap();
        let start = ((splitter.x + 4) as i32, (splitter.y + 1) as i32);
        assert!(ui.begin_drag(&layout, start));
        let _ = footer;
        for _ in 0..4 {
            // Includes absurd extremes in both directions.
            let dy = ((next() - 0.5) * 20000.0) as i32;
            ui.drag_to((start.0, start.1 + dy), (w, h), Mode::Animate);
            let l = Layout::with_state(w, h, Mode::Animate, &ui).unwrap();
            let f = l.footer.unwrap();
            assert!(
                f.height >= FOOTER_MIN,
                "{w}x{h} dy={dy}: footer {}",
                f.height
            );
            assert!(
                l.stage.height >= STAGE_MIN_H,
                "{w}x{h} dy={dy}: stage {}",
                l.stage.height
            );
            assert!(ui.footer_h >= FOOTER_MIN);
        }
        ui.end_drag();
    }
}

// ---- 3: ruler ----------------------------------------------------------------------

#[test]
fn ruler_ticks_are_dense_ordered_and_labelled_without_overlap() {
    for width in (400..=1600).step_by(40) {
        for loop_s in [0.5, 1.0, 2.0, 10.0, 60.0] {
            let l = TimelineLayout::new(rect(0, 0, width, 200));
            let ticks = l.ticks(loop_s);
            assert!(ticks.len() >= 2, "{width} {loop_s}");
            for pair in ticks.windows(2) {
                assert!(pair[1].time > pair[0].time, "{width} {loop_s}: time");
                assert!(pair[1].x > pair[0].x, "{width} {loop_s}: x");
            }
            let cells: Vec<_> = ticks.iter().filter_map(|t| t.label.clone()).collect();
            for pair in cells.windows(2) {
                let (a, b) = (pair[0].1, pair[1].1);
                assert!(a.x + a.width <= b.x, "{width} {loop_s}: labels overlap");
            }
            for (text, cell) in &cells {
                assert!(cell.x + cell.width <= width, "{text} inside the panel");
            }
            if loop_s < 3.0 {
                // 0.25 s ticks, labels at 0.5 s multiples.
                let quarters = (loop_s / 0.25).round() as usize + 1;
                assert_eq!(ticks.len(), quarters, "{width} {loop_s}: 0.25 s ticks");
                for (i, t) in ticks.iter().enumerate() {
                    assert!((t.time - i as f64 * 0.25).abs() < 1e-9);
                    if t.label.is_some() {
                        assert!(t.major && (t.time / 0.5 - (t.time / 0.5).round()).abs() < 1e-9);
                    }
                }
                assert!(
                    ticks.iter().any(|t| t.label.is_some()),
                    "{width} {loop_s}: at least one label"
                );
            }
        }
    }
}

#[test]
fn frame_ticks_appear_only_when_a_frame_is_six_pixels_wide() {
    let wide = TimelineLayout::new(rect(0, 0, 1280, 200));
    let frames = wide.frame_ticks(1.0);
    assert!(!frames.is_empty());
    for pair in frames.windows(2) {
        assert!(pair[1].1 - pair[0].1 >= 6.0 - 1e-9);
    }
    assert!(
        wide.frame_ticks(10.0).is_empty(),
        "frames are under 6 px at 10 s"
    );
    assert!(wide.frame_ticks(60.0).is_empty());
}

#[test]
fn ruler_labels_pass_contrast_and_never_touch_the_playhead_handle() {
    for dark in [true, false] {
        let t = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
        assert!(
            contrast(t.text_secondary, t.panel) >= 4.5,
            "dark={dark}: {}",
            contrast(t.text_secondary, t.panel)
        );
    }
    for width in [400, 640, 936, 1280, 1600] {
        for loop_s in [0.5, 1.0, 2.0, 10.0, 60.0] {
            let l = TimelineLayout::new(rect(0, 0, width, 200));
            let mut times: Vec<f64> = (0..=200).map(|i| loop_s * f64::from(i) / 200.0).collect();
            times.extend(l.ticks(loop_s).iter().map(|t| t.time));
            for time in times {
                let handle = l.playhead_handle(time, loop_s);
                for (text, cell) in l.visible_labels(loop_s, time) {
                    let apart = cell.x >= handle.x + handle.width
                        || cell.x + cell.width <= handle.x
                        || cell.y >= handle.y + handle.height
                        || cell.y + cell.height <= handle.y;
                    assert!(apart, "{width} {loop_s} t={time}: {text} under the handle");
                }
            }
        }
    }
}

// ---- 4: diamonds ------------------------------------------------------------------

#[test]
fn dark_unselected_diamonds_have_three_to_one_against_the_lane() {
    let l = TimelineLayout::new(footer(SIZE));
    let mut d = driver_with(LOOP);
    d.seek(0.8);
    let tl = timeline_of(LOOP, &d);
    let rows = tl.rows();
    for dark in [true, false] {
        let tk = tokens(dark);
        let shot = frame(SIZE, dark, 1, tl.clone());
        for (slot, row) in rows.iter().enumerate() {
            let mid = l.row_rect(slot).y + (ROW_H - 1) / 2;
            // Keys at 0, 0.5, 1.0: the lane (the row's span bar) at 0.25.
            let lane = pixel(&shot, 1, l.time_to_x(0.25, 1.0).round() as u32, mid);
            let diamond = pixel(&shot, 1, l.time_to_x(0.5, 1.0).round() as u32, mid);
            let want = match row {
                Row::Element(_) if !dark => tk.text_tertiary,
                _ => tk.text_secondary,
            };
            assert_eq!(diamond, rgb(want), "dark={dark} row {slot}");
            if dark {
                let ratio = contrast(rgb_u32(diamond), rgb_u32(lane));
                assert!(ratio >= 3.0, "dark row {slot}: {ratio:.2}:1");
            }
        }
    }
}

#[test]
fn selected_diamonds_keep_the_accent() {
    let l = TimelineLayout::new(footer(SIZE));
    let mut d = driver_with(LOOP);
    d.seek(0.8);
    let mut tl = timeline_of(LOOP, &d);
    tl.selection = Some(KeyRef {
        element: 0,
        prop: Some(0),
        index: 1,
    });
    for dark in [true, false] {
        let shot = frame(SIZE, dark, 1, tl.clone());
        let mid = l.row_rect(1).y + (ROW_H - 1) / 2;
        assert_eq!(
            pixel(&shot, 1, l.time_to_x(0.5, 1.0).round() as u32, mid),
            rgb(tokens(dark).accent)
        );
    }
}

// ---- 5: transport -------------------------------------------------------------------

#[test]
fn transport_buttons_are_ordered_in_two_clusters_without_overlap() {
    for width in (400..=1600).step_by(23) {
        let l = TimelineLayout::new(rect(0, 500, width, 200));
        let slots: Vec<(&str, Option<Rect>)> = vec![
            ("step-first", l.skip_start),
            ("step-back", l.step_back),
            ("play", l.play),
            ("step-next", l.step_forward),
            ("step-last", l.skip_end),
            ("loop", l.loop_btn),
            ("readout", l.readout),
            ("speed", l.speed),
            ("group", l.group),
        ];
        // Present controls form a prefix: nothing appears without its predecessors.
        let present: Vec<_> = slots
            .iter()
            .filter_map(|(n, r)| r.map(|r| (*n, r)))
            .collect();
        let first_missing = slots
            .iter()
            .position(|(_, r)| r.is_none())
            .unwrap_or(slots.len());
        assert!(
            slots[first_missing..]
                .iter()
                .all(|(n, r)| r.is_none() || *n == "speed" || *n == "group"),
            "{width}: gap in the left cluster"
        );
        for pair in present.windows(2) {
            let (a, b) = (pair[0].1, pair[1].1);
            assert!(
                a.x + a.width <= b.x,
                "{width}: {} overlaps {}",
                pair[0].0,
                pair[1].0
            );
        }
        if let (Some(g), Some(e), Some(k)) = (l.group, l.ease, l.add_key) {
            assert!(
                e.x >= g.x && k.x + k.width <= g.x + g.width,
                "{width}: inside the container"
            );
            assert!(e.x + e.width <= k.x);
            assert!(g.x + g.width <= width);
        } else {
            assert!(l.ease.is_none() && l.add_key.is_none() && l.group.is_none());
        }
        // Order of the hit-testable buttons.
        let order: Vec<_> = l.buttons().into_iter().map(|(h, _)| h).collect();
        let all = [
            Hit::SkipStart,
            Hit::StepBack,
            Hit::Play,
            Hit::StepForward,
            Hit::SkipEnd,
            Hit::Loop,
        ];
        assert_eq!(order, all[..order.len()], "{width}");
        // Disabled controls are never hit.
        let model = TimelineLayout::new(rect(0, 500, width, 200));
        for (hit, rc) in model.disabled_controls() {
            let centre = ((rc.x + rc.width / 2) as i32, (rc.y + rc.height / 2) as i32);
            assert_eq!(
                model.disabled_at(centre.0, centre.1).map(|(h, _)| h),
                Some(hit)
            );
            let tl = Timeline::default();
            assert_eq!(
                model.hit_test(&tl.model, &[], 0, centre.0, centre.1),
                Hit::Panel,
                "{width}: {hit:?} must not be hit-testable"
            );
        }
    }
    let l = TimelineLayout::new(rect(0, 500, 1280, 200));
    assert!(
        l.group.is_some() && l.speed.is_some(),
        "full width shows both right clusters"
    );
    assert!(l.speed.unwrap().x + l.speed.unwrap().width <= l.group.unwrap().x);
}

fn diff_box(a: &(u32, u32, Vec<u8>), b: &(u32, u32, Vec<u8>)) -> Option<(u32, u32, u32, u32)> {
    let w = a.0;
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for (i, (p, q)) in a.2.chunks_exact(4).zip(b.2.chunks_exact(4)).enumerate() {
        if p != q {
            let (x, y) = (i as u32 % w, i as u32 / w);
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bbox
}

#[test]
fn hovering_a_disabled_control_for_500ms_shows_its_hint_inside_the_window() {
    let f = footer(SIZE);
    let l = TimelineLayout::new(f);
    for (hit, cell) in l.disabled_controls() {
        let mut d = driver_with(LOOP);
        let mut tl = timeline_of(LOOP, &d);
        let centre = (
            (cell.x + cell.width / 2) as i32,
            (cell.y + cell.height / 2) as i32,
        );
        let before = frame(SIZE, true, 1, tl.clone());
        let out = tl.pointer(Phase::Move, centre, &l, &mut d);
        assert!(out.redraw || tl.model.tip.is_some());
        assert_eq!(tl.hover, None, "disabled controls are not hover targets");
        assert_eq!(tl.model.tip.map(|t| t.0), Some(hit));
        tl.model.sync_playback(&d);
        assert_eq!(tl.model.tip_ready(), None, "no tooltip at once");
        d.playback().clock().advance(0.49);
        assert!(!tl.model.sync_playback(&d));
        assert_eq!(tl.model.tip_ready(), None, "not before the polish delay");
        d.playback().clock().advance(0.01);
        assert!(tl.model.sync_playback(&d), "the tooltip appearing redraws");
        assert_eq!(tl.model.tip_ready(), Some(hit));
        let shown = frame(SIZE, true, 1, tl.clone());
        let (x0, y0, x1, y1) = diff_box(&before, &shown).expect("a tooltip is painted");
        assert!(x1 < SIZE.0 && y1 < SIZE.1, "inside the window");
        assert!(
            y0 > cell.y + cell.height,
            "below the control, over the footer"
        );
        assert!(
            x0 >= f.x && x1 < f.x + f.width,
            "inside the footer: {x0}..{x1}"
        );
        let hint = hit.disabled_hint().unwrap();
        assert!(
            x1 - x0 + 1 >= (hint.len() as u32 * 12 * 56 / 100) / 2,
            "wide enough for {hint}"
        );
        // Leaving the control hides it again.
        tl.pointer(
            Phase::Move,
            (f.x as i32 + 300, (f.y + 100) as i32),
            &l,
            &mut d,
        );
        assert_eq!(tl.model.tip, None);
        assert_eq!(tl.model.tip_ready(), None);
        // Hovering an enabled button never raises one.
        let step = l.step_back.unwrap();
        tl.pointer(
            Phase::Move,
            ((step.x + 4) as i32, (step.y + 4) as i32),
            &l,
            &mut d,
        );
        d.playback().clock().advance(2.0);
        tl.model.sync_playback(&d);
        assert_eq!(tl.model.tip_ready(), None);
    }
}
