//! WO-RUST-UI-MOTION-01 acceptance 1-4 (headless, injected nanosecond time).
use crate::motion::*;

const MS: u64 = 1_000_000;
// Reference tables: y at x = 0.0, 0.1 .. 1.0, solved independently by 80-step
// bisection of the section 4.5 cubic-bezier control points (Python, f64).
const EASE_OUT: [f32; 11] = [
    0.0, 0.15625, 0.5, 0.688067, 0.802284, 0.877834, 0.929072, 0.963318, 0.984862, 0.996459, 1.0,
];
const STANDARD: [f32; 11] = [
    0.0, 0.046747, 0.222918, 0.47166, 0.666252, 0.797153, 0.884111, 0.940894, 0.975895, 0.994418,
    1.0,
];
const EASE_IN: [f32; 11] = [
    0.0, 0.018373, 0.066151, 0.13594, 0.223167, 0.324815, 0.438821, 0.563785, 0.698843, 0.84375,
    1.0,
];

fn colors() -> ControlColors {
    ControlColors {
        rest_fill: 0x2020_20ff,
        hover_fill: 0x3030_30ff,
        pressed_fill: 0x4040_40ff,
        rest_glyph: 0x8080_80ff,
        hover_glyph: 0xffff_ffff,
        disabled_glyph: 0x5050_50ff,
        ring: 0x4488_ffff,
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

#[test]
fn ac1_easing_endpoints_monotonic_and_reference_table() {
    for (easing, table) in [
        (Easing::EaseOut, EASE_OUT),
        (Easing::Standard, STANDARD),
        (Easing::EaseIn, EASE_IN),
    ] {
        assert!(easing.eval(0.0).abs() < 1e-4 && (easing.eval(1.0) - 1.0).abs() < 1e-4);
        let mut last = -1.0f32;
        for i in 0..=1000 {
            let v = easing.eval(i as f32 / 1000.0);
            assert!(v >= last - 1e-6, "{easing:?} not monotonic at {i}");
            last = v;
        }
        for (i, want) in table.iter().enumerate() {
            let got = easing.eval(i as f32 / 10.0);
            assert!(
                (got - want).abs() < 1e-3,
                "{easing:?} x={} got {got} want {want}",
                i as f32 / 10.0
            );
        }
    }
}

#[test]
fn ac1_solver_curves_match_the_motion_tokens() {
    let m = crate::Motion::STANDARD;
    assert_eq!(Easing::EaseOut.points(), m.ease_out);
    assert_eq!(Easing::Standard.points(), m.standard);
    assert_eq!(Easing::EaseIn.points(), m.ease_in);
}

/// AC2 as amended by the coordinator: "a 120 ms hover-in transition is between
/// 50% and 90% complete at t0+60 ms (the section 4.5 ease_out
/// cubic-bezier(0.2,0,0,1) gives 87.8% there)". Other sub-checks as written.
#[test]
fn ac2_hover_durations_and_reentry_continuity() {
    let t0 = 1_000 * MS;
    let mut r = MotionRegistry::new(false);
    let c = colors();
    let h = |r: &MotionRegistry, t: u64| r.visual(7, t, &c, false).hover;
    r.pointer_move(Some(7), t0);
    assert_eq!(h(&r, t0), 0.0);
    let mid = h(&r, t0 + 60 * MS);
    assert!((0.5..=0.9).contains(&mid), "60 ms progress {mid}");
    assert!((mid - 0.877834).abs() < 1e-3);
    assert_eq!(h(&r, t0 + 120 * MS), 1.0);
    // Leave (90 ms) then re-enter mid-transition: no jump larger than 0.02.
    let t1 = t0 + 200 * MS;
    r.pointer_move(None, t1);
    let t2 = t1 + 40 * MS;
    let before = h(&r, t2);
    assert!(before > 0.0 && before < 1.0);
    r.pointer_move(Some(7), t2);
    assert!((h(&r, t2) - before).abs() <= 0.02);
    let mut prev = h(&r, t2);
    for k in 1..=130u64 {
        let v = h(&r, t2 + k * MS);
        assert!((v - prev).abs() <= 0.02, "jump at {k} ms");
        prev = v;
    }
    assert_eq!(prev, 1.0);
}

#[test]
fn ac2_durations_of_each_state() {
    let c = colors();
    let t0 = 5 * MS;
    let mut r = MotionRegistry::new(false);
    r.pointer_move(Some(1), t0);
    r.pointer_down(Some(1), t0 + 200 * MS);
    let p = |r: &MotionRegistry, t: u64| r.visual(1, t, &c, false).press;
    assert_eq!(p(&r, t0 + 200 * MS), 0.0);
    assert_eq!(p(&r, t0 + 280 * MS), 1.0);
    assert!(p(&r, t0 + 279 * MS) < 1.0);
    r.pointer_up(t0 + 400 * MS);
    assert!(p(&r, t0 + 519 * MS) > 0.0);
    assert_eq!(p(&r, t0 + 520 * MS), 0.0);
    r.focus(Some(1), t0 + 600 * MS);
    assert!(r.visual(1, t0 + 660 * MS, &c, false).focus < 1.0);
    assert_eq!(r.visual(1, t0 + 720 * MS, &c, false).focus, 1.0);
    let mut thumb = ThumbSlide::new(0.0, false);
    thumb.slide_to(0, 100.0);
    assert!(thumb.x(179 * MS) < 100.0 && thumb.x(180 * MS) == 100.0);
    thumb.snap_to(10.0);
    assert_eq!(thumb.x(181 * MS), 10.0);
}

#[test]
fn ac3_random_sequences_never_panic_and_settle() {
    let c = colors();
    let mut rng = Rng(0x5eed);
    for _ in 0..10_000 {
        let mut r = MotionRegistry::new(false);
        let mut now = 0u64;
        for _ in 0..30 {
            now += rng.next(60) * MS;
            let id = Some(1 + rng.next(100) as u32);
            match rng.next(5) {
                0 => r.pointer_move(id, now),
                1 => r.pointer_move(None, now),
                2 => r.pointer_down(id, now),
                3 => r.pointer_up(now),
                _ => r.focus(if rng.next(2) == 0 { id } else { None }, now),
            }
        }
        now += 300 * MS;
        assert_eq!(r.next_deadline(now), None);
        for id in 1..=100u32 {
            let v = r.visual(id, now, &c, false);
            assert_eq!(v.hover, (r.hovered() == Some(id)) as u32 as f32);
            assert_eq!(v.press, (r.pressed() == Some(id)) as u32 as f32);
            assert_eq!(v.focus, (r.focused() == Some(id)) as u32 as f32);
        }
        let mut out = [(0, ControlVisual::EMPTY); MAX_CONTROLS];
        assert!(r.visuals_into(now, &c, &mut out) <= MAX_CONTROLS);
    }
}

#[test]
fn ac3_excess_controls_snap_to_target() {
    let c = colors();
    let mut r = MotionRegistry::new(false);
    for id in 1..=64u32 {
        r.pointer_move(Some(id), id as u64 * MS);
    }
    r.pointer_move(Some(65), 70 * MS);
    // 65 is untracked (all 64 slots busy): immediately at its target.
    assert_eq!(r.visual(65, 70 * MS, &c, false).hover, 1.0);
    r.pointer_move(None, 71 * MS);
    assert_eq!(r.visual(65, 71 * MS, &c, false).hover, 0.0);
}

#[test]
fn ac4_reduced_motion_is_instant_with_no_deadline() {
    let c = colors();
    let mut r = MotionRegistry::new(true);
    r.pointer_move(Some(3), 10);
    assert_eq!(r.visual(3, 10, &c, false).hover, 1.0);
    r.pointer_down(Some(3), 20);
    assert_eq!(r.visual(3, 20, &c, false).press, 1.0);
    r.pointer_up(30);
    r.focus(Some(3), 30);
    assert_eq!(r.visual(3, 30, &c, false).press, 0.0);
    assert_eq!(r.visual(3, 30, &c, false).focus, 1.0);
    assert_eq!(r.next_deadline(30), None);
    let mut thumb = ThumbSlide::new(0.0, true);
    thumb.slide_to(0, 50.0);
    assert_eq!(thumb.x(0), 50.0);
    assert!(!thumb.is_animating(0));
}

#[test]
fn disabled_has_no_hover_and_deadline_tracks_activity() {
    let c = colors();
    let mut r = MotionRegistry::new(false);
    r.pointer_move(Some(2), 0);
    let v = r.visual(2, 60 * MS, &c, true);
    assert!(v.disabled && v.hover == 0.0 && v.glyph == c.disabled_glyph);
    assert_eq!(
        r.next_deadline(60 * MS),
        Some(std::time::Duration::from_nanos(FRAME_NS))
    );
    assert_eq!(
        r.next_deadline(110 * MS),
        Some(std::time::Duration::from_nanos(10 * MS))
    );
    assert_eq!(r.next_deadline(120 * MS), None);
}
