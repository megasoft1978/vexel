//! WO-RUST-UI-TIMELINE-01 acceptance 6: the painted footer for loop.svg at
//! t = 0.8 s (1x and 2x, dark and light): playhead column in accent at the
//! mapped x, one diamond per model key at the mapped position, ruler ticks at
//! the major spacing, and nothing drawn outside the footer rect.
#[path = "ui_timeline_support.rs"]
mod support;

use support::*;
use vexel_app::model::{Mode, Model};
use vexel_app::render::render;
use vexel_app::timeline::layout::{ROW_H, TimelineLayout};
use vexel_app::timeline::model::Row;

const SIZE: (u32, u32) = (1280, 800);

fn loaded(t: f64) -> vexel_app::timeline::Timeline {
    let mut d = driver_with(LOOP);
    d.seek(t);
    timeline_of(LOOP, &d)
}

#[test]
fn playhead_diamonds_and_ticks_are_painted_at_their_mapped_positions() {
    for dark in [true, false] {
        for scale in [1, 2] {
            let tk = tokens(dark);
            let tl = loaded(0.8);
            let l = TimelineLayout::new(footer(SIZE));
            let rows = tl.rows();
            let shot = frame(SIZE, dark, scale, tl.clone());
            let ctx = format!("dark={dark} scale={scale}");

            // Playhead column below the last row, where nothing else is drawn.
            let px = l.playhead_x(0.8, 1.0).round() as u32;
            let y = l.tracks.y + rows.len() as u32 * ROW_H + 8;
            assert!(y < l.tracks.y + l.tracks.height);
            assert_eq!(
                pixel(&shot, scale, px, y),
                rgb(tk.accent),
                "{ctx}: playhead"
            );
            // Its handle over the ruler.
            assert_eq!(
                pixel(&shot, scale, px, l.ruler.y + 8),
                rgb(tk.accent),
                "{ctx}: handle"
            );
            // Away from the playhead the same spot is not accent.
            assert_ne!(pixel(&shot, scale, px + 40, y), rgb(tk.accent));

            // One diamond per key, centre pixel in the key colour.
            let mut count = 0;
            for (slot, row) in rows.iter().enumerate() {
                let mid = l.row_rect(slot).y + (ROW_H - 1) / 2;
                let want = match row {
                    Row::Element(_) => tk.text_tertiary,
                    Row::Prop(..) => tk.text_secondary,
                };
                for time in tl.model.row_keys(*row) {
                    let x = l.time_to_x(time, 1.0).round() as u32;
                    assert_eq!(
                        pixel(&shot, scale, x, mid),
                        rgb(want),
                        "{ctx}: key {time} row {slot}"
                    );
                    count += 1;
                }
            }
            let expected: usize = rows.iter().map(|r| tl.model.row_keys(*r).len()).sum();
            assert_eq!(count, expected);
            assert_eq!(count, 12, "2 elements x (3 summary + 3 property) keys");

            // Major ticks at one-second spacing: the tick pixel is drawn in
            // text_tertiary just above the ruler's bottom edge.
            let ticks = l.ticks(1.0);
            let majors: Vec<_> = ticks.iter().filter(|t| t.major).collect();
            assert_eq!(majors.len(), 2);
            let base = l.ruler.y + l.ruler.height - 1;
            for tick in majors {
                let x = tick.x.round() as u32;
                assert_eq!(
                    pixel(&shot, scale, x, base - 4),
                    rgb(tk.text_tertiary),
                    "{ctx}: tick"
                );
            }
            let spacing = majors_spacing(&l);
            assert!((spacing - (l.lane_x1 - l.lane_x0)).abs() < 1e-9);
        }
    }
}

fn majors_spacing(l: &TimelineLayout) -> f64 {
    let t = l.ticks(1.0);
    let m: Vec<_> = t.iter().filter(|t| t.major).collect();
    m[1].x - m[0].x
}

#[test]
fn the_timeline_draws_nothing_outside_its_rect() {
    let f = footer(SIZE);
    let mut base = Model::new(true);
    base.mode = Mode::Animate;
    let mut with = Model::new(true);
    with.mode = Mode::Animate;
    with.timeline = loaded(0.8);
    let a = render(SIZE.0, SIZE.1, 1, &base).unwrap();
    let b = render(SIZE.0, SIZE.1, 1, &with).unwrap();
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            let outside = x < f.x || x >= f.x + f.width || y < f.y || y >= f.y + f.height;
            if outside {
                let at = ((y * SIZE.0 + x) * 4) as usize;
                assert_eq!(a.pixels()[at..at + 4], b.pixels()[at..at + 4], "({x},{y})");
            }
        }
    }
}

#[test]
fn hover_and_pressed_states_paint_visibly_different_buttons() {
    use vexel_app::timeline::layout::Hit;
    let l = TimelineLayout::new(footer(SIZE));
    let skip = l.skip_start.unwrap();
    let probe = (skip.x + 2, skip.y + 2);
    let rest = frame(SIZE, true, 1, loaded(0.0));
    let mut hover = loaded(0.0);
    hover.hover = Some(Hit::SkipStart);
    let hovered = frame(SIZE, true, 1, hover);
    assert_ne!(
        pixel(&rest, 1, probe.0, probe.1),
        pixel(&hovered, 1, probe.0, probe.1)
    );
    let mut press = loaded(0.0);
    press.hover = Some(Hit::SkipStart);
    press.pressed = Some(Hit::SkipStart);
    let pressed = frame(SIZE, true, 1, press);
    assert_ne!(
        pixel(&hovered, 1, probe.0, probe.1),
        pixel(&pressed, 1, probe.0, probe.1)
    );
}

#[test]
fn empty_states_paint_without_tracks() {
    let d = driver_with(STATIC);
    let stat = timeline_of(STATIC, &d);
    let none = vexel_app::timeline::Timeline::default();
    for tl in [stat, none] {
        let tk = tokens(true);
        let l = TimelineLayout::new(footer(SIZE));
        let shot = frame(SIZE, true, 1, tl);
        // No playhead: the accent never appears in the track band.
        let mut accent = 0;
        for y in l.tracks.y..l.tracks.y + l.tracks.height {
            for x in l.tracks.x..l.tracks.x + l.tracks.width {
                if pixel(&shot, 1, x, y) == rgb(tk.accent) {
                    accent += 1;
                }
            }
        }
        assert_eq!(accent, 0);
    }
}
