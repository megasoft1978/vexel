//! WO-RUST-UI-CHROME-01 acceptance 1 to 6: pure layout geometry, resize,
//! hit-testing, segmented control, section collapse and status/MCP.

use super::*;
use crate::model::{McpState, Model};

const MODES: [Mode; 2] = [Mode::Draw, Mode::Animate];

fn inside(a: Rect, w: u32, h: u32) -> bool {
    a.x + a.width <= w && a.y + a.height <= h
}

fn disjoint(a: Rect, b: Rect) -> bool {
    a.width == 0
        || b.width == 0
        || a.height == 0
        || b.height == 0
        || a.x + a.width <= b.x
        || b.x + b.width <= a.x
        || a.y + a.height <= b.y
        || b.y + b.height <= a.y
}

fn check_geometry(l: &Layout, w: u32, h: u32, mode: Mode) {
    let mut panels = vec![l.top, l.rail, l.stage, l.status];
    panels.extend(l.inspector);
    panels.extend(l.inspector_splitter);
    panels.extend(l.footer);
    panels.extend(l.footer_splitter);
    for p in &panels {
        assert!(inside(*p, w, h), "{w}x{h} {mode:?}: {p:?} outside");
    }
    for (i, a) in panels.iter().enumerate() {
        for b in &panels[i + 1..] {
            assert!(disjoint(*a, *b), "{w}x{h} {mode:?}: {a:?} overlaps {b:?}");
        }
    }
    assert_eq!(l.top.height, 44);
    assert_eq!(l.rail.width, 44);
    assert_eq!(l.status.height, 24);
    assert!(l.stage.width >= STAGE_MIN_W && l.stage.height >= STAGE_MIN_H);
    match l.inspector {
        Some(i) => {
            assert!(w >= INSPECTOR_MIN_WINDOW);
            assert!(
                (INSPECTOR_MIN..=INSPECTOR_MAX).contains(&i.width),
                "{w}: {}",
                i.width
            );
        }
        None => assert!(w < INSPECTOR_MIN_WINDOW),
    }
    let content = h - 44 - 24;
    match (mode, l.footer) {
        (Mode::Animate, Some(f)) => {
            assert!(
                f.height >= FOOTER_MIN && f.height <= content / 2,
                "{h}: {}",
                f.height
            );
        }
        (Mode::Draw, None) => {}
        other => panic!("footer/mode mismatch: {other:?}"),
    }
}

#[test]
fn geometry_holds_from_the_minimum_to_4096() {
    for w in [720, 721, 800, 899, 900, 901, 1000, 1280, 2048, 4095, 4096] {
        for h in [480, 481, 600, 820, 1500, 4096] {
            for mode in MODES {
                let l = Layout::new(w, h, mode).unwrap();
                check_geometry(&l, w, h, mode);
            }
        }
    }
}

#[test]
fn sizes_below_the_minimum_or_above_the_maximum_are_rejected() {
    for mode in MODES {
        for (w, h) in [
            (719, 480),
            (720, 479),
            (0, 0),
            (440, 300),
            (4097, 600),
            (900, 4097),
        ] {
            assert!(Layout::new(w, h, mode).is_none(), "{w}x{h}");
        }
    }
}

#[test]
fn five_thousand_seeded_sizes_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = |m: u32| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % u64::from(m)) as u32
    };
    for _ in 0..5000 {
        let (w, h) = (next(5000), next(5000));
        let mut ui = UiState::default();
        ui.set_inspector_width(next(1000));
        ui.set_footer_height(next(1000), 400);
        for mode in MODES {
            let got = Layout::with_state(w, h, mode, &ui);
            let valid = (MIN_W..=MAX_AXIS).contains(&w) && (MIN_H..=MAX_AXIS).contains(&h);
            assert_eq!(got.is_some(), valid, "{w}x{h}");
            if let Some(l) = got {
                check_geometry(&l, w, h, mode);
            }
        }
    }
}

#[test]
fn splitter_drags_clamp_and_never_shrink_the_stage_below_260() {
    for (w, h, mode) in [
        (900, 480, Mode::Animate),
        (1280, 800, Mode::Animate),
        (4096, 4096, Mode::Draw),
    ] {
        for delta in (-5000..5000).step_by(53) {
            let mut ui = UiState::default();
            let l = Layout::with_state(w, h, mode, &ui).unwrap();
            let s = l.inspector_splitter.unwrap();
            assert!(ui.begin_drag(&l, (s.x as i32 + 1, s.y as i32 + 5)));
            ui.drag_to((s.x as i32 + 1 + delta, 0), (w, h), mode);
            ui.end_drag();
            let after = Layout::with_state(w, h, mode, &ui).unwrap();
            check_geometry(&after, w, h, mode);
            if mode == Mode::Animate {
                let l = Layout::with_state(w, h, mode, &ui).unwrap();
                let f = l.footer_splitter.unwrap();
                assert!(ui.begin_drag(&l, (f.x as i32 + 9, f.y as i32 + 1)));
                ui.drag_to((0, f.y as i32 + delta), (w, h), mode);
                ui.end_drag();
                check_geometry(&Layout::with_state(w, h, mode, &ui).unwrap(), w, h, mode);
            }
        }
    }
}

#[test]
fn layout_round_trips_a_set_size() {
    let mut ui = UiState::default();
    for want in [240, 296, 333, 420] {
        ui.set_inspector_width(want);
        let l = Layout::with_state(1600, 900, Mode::Draw, &ui).unwrap();
        assert_eq!(l.inspector.unwrap().width, want);
    }
    ui.set_inspector_width(10);
    assert_eq!(ui.inspector_w, INSPECTOR_MIN);
    ui.set_inspector_width(9999);
    assert_eq!(ui.inspector_w, INSPECTOR_MAX);
    for want in [120, 200, 350] {
        ui.set_footer_height(want, 900 - 68);
        let l = Layout::with_state(1600, 900, Mode::Animate, &ui).unwrap();
        assert_eq!(l.footer.unwrap().height, want);
    }
}

fn targets(l: &Layout) -> Vec<(Target, Rect)> {
    let mut v = vec![
        (Target::DrawMode, l.draw_mode),
        (Target::AnimateMode, l.animate_mode),
        (Target::Button(Button::Mcp), l.mcp),
    ];
    let buttons = [
        Button::Undo,
        Button::Redo,
        Button::Save,
        Button::Export,
        Button::ToggleInspector,
    ];
    v.extend(
        buttons
            .iter()
            .zip(l.buttons)
            .map(|(b, r)| (Target::Button(*b), r)),
    );
    let tools = [
        Tool::Select,
        Tool::Pen,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::Line,
    ];
    v.extend(
        tools
            .iter()
            .zip(l.tools)
            .map(|(t, r)| (Target::Tool(*t), r)),
    );
    for (i, r) in l.section_headers.iter().enumerate() {
        if r.width > 0 {
            v.push((Target::SectionHeader(i), *r));
        }
    }
    v.extend(
        l.inspector_splitter
            .map(|r| (Target::Splitter(Splitter::Inspector), r)),
    );
    v.extend(
        l.footer_splitter
            .map(|r| (Target::Splitter(Splitter::Footer), r)),
    );
    v
}

#[test]
fn every_target_hit_tests_to_itself_and_targets_are_big_enough() {
    for (w, h) in [
        (720, 480),
        (900, 480),
        (1280, 800),
        (960, 640),
        (2000, 1200),
    ] {
        for mode in MODES {
            let l = Layout::new(w, h, mode).unwrap();
            let all = targets(&l);
            for (i, (t, r)) in all.iter().enumerate() {
                let min = match t {
                    Target::Tool(_) => 32,
                    Target::Splitter(_) => 4,
                    _ => 28,
                };
                assert!(r.width >= min && r.height >= min, "{w}x{h} {t:?} {r:?}");
                if matches!(t, Target::Splitter(_)) {
                    assert!(r.width.max(r.height) >= 28);
                }
                for (u, o) in &all[i + 1..] {
                    assert!(disjoint(*r, *o), "{w}x{h}: {t:?} overlaps {u:?}");
                }
                let centre = (r.x + r.width / 2, r.y + r.height / 2);
                assert_eq!(l.hit_test(centre), Some(*t), "{w}x{h} {t:?}");
            }
            assert_eq!(l.hit_test((l.stage.x + 10, l.stage.y + 10)), None);
        }
    }
}

#[test]
fn selecting_animate_moves_the_thumb_target_and_keys_work_when_focused() {
    let mut model = Model::new(true);
    let l = Layout::new(1280, 800, Mode::Draw).unwrap();
    assert_eq!(l.thumb_target(Mode::Draw), l.draw_mode);
    assert!(model.pointer(&l, (l.animate_mode.x + 3, l.animate_mode.y + 3)));
    assert_eq!(model.mode, Mode::Animate);
    let a = model.layout(1280, 800).unwrap();
    assert_eq!(a.thumb_target(model.mode), a.animate_mode);
    let segs = segment_rects(a.segmented, 2);
    assert_eq!(segs, vec![a.draw_mode, a.animate_mode]);
    // Left/Right only act once the control has focus.
    assert!(!model.focus_key(123, false, true));
    assert!(model.focus_key(48, false, true));
    assert_eq!(model.ui.focus, Some(Focus::Segmented));
    assert!(model.focus_key(123, false, true));
    assert_eq!(model.mode, Mode::Draw);
    assert!(model.focus_key(124, false, true));
    assert_eq!(model.mode, Mode::Animate);
}

#[test]
fn collapsing_a_section_removes_its_rows_and_moves_the_next_header_up() {
    let open = UiState::default();
    let base = Layout::with_state(1280, 800, Mode::Draw, &open).unwrap();
    for i in 0..SECTIONS - 1 {
        let mut ui = open;
        ui.collapsed[i] = !open.collapsed[i];
        let flipped = Layout::with_state(1280, 800, Mode::Draw, &ui).unwrap();
        let body = SECTION_BODY_H[i];
        let (a, b) = if open.collapsed[i] {
            (&base, &flipped)
        } else {
            (&flipped, &base)
        };
        // `a` has section i collapsed, `b` expanded.
        assert_eq!(a.section_bodies[i].height, 0);
        assert!(b.section_bodies[i].height > 0);
        for j in i + 1..SECTIONS {
            if a.section_headers[j].width > 0 && b.section_headers[j].width > 0 {
                assert_eq!(
                    b.section_headers[j].y - a.section_headers[j].y,
                    body,
                    "{i}->{j}"
                );
            }
        }
        // Restoring the state restores the exact geometry.
        assert_eq!(
            Layout::with_state(1280, 800, Mode::Draw, &open).unwrap(),
            base
        );
    }
}

#[test]
fn focus_order_follows_reading_order() {
    let mut ui = UiState::default();
    let mut seen = Vec::new();
    for _ in 0..=SECTIONS + 1 {
        ui.tab(true, true);
        seen.extend(ui.focus);
    }
    let want: Vec<Focus> = std::iter::once(Focus::Segmented)
        .chain((0..SECTIONS).map(Focus::Section))
        .collect();
    assert_eq!(seen, want);
    assert_eq!(
        ui.focus, None,
        "past the last stop focus returns to the page"
    );
    let mut none = UiState::default();
    none.tab(true, false);
    none.tab(true, false);
    assert_eq!(none.focus, None, "no inspector, no section stops");
}

#[test]
fn space_and_return_toggle_a_focused_section_header() {
    let mut model = Model::new(true);
    model.ui.focus = Some(Focus::Section(1));
    assert!(model.focus_key(49, false, true));
    assert!(model.ui.collapsed[1]);
    assert!(model.focus_key(36, false, true));
    assert!(!model.ui.collapsed[1]);
}

#[test]
fn the_activity_band_is_gone_and_chrome_is_44_plus_24() {
    for mode in MODES {
        for (w, h) in [(720, 480), (1280, 800), (1000, 1000)] {
            let l = Layout::new(w, h, mode).unwrap();
            assert_eq!(l.rail.height + l.top.height + l.status.height, h);
            assert_eq!(l.rail.y + l.rail.height, l.status.y);
        }
    }
}

#[test]
fn mcp_state_is_conveyed_by_word_and_the_popover_keeps_the_old_text() {
    let labels: Vec<&str> = [McpState::Unavailable, McpState::Ready, McpState::Active]
        .iter()
        .map(|s| s.label())
        .collect();
    for (i, l) in labels.iter().enumerate() {
        assert!(l.starts_with("MCP ") && l.len() > 4);
        assert!(labels.iter().enumerate().all(|(j, o)| i == j || o != l));
    }
    assert_eq!(McpState::Unavailable.detail(), "Unavailable in this shell");
    let mut model = Model::new(true);
    let l = model.layout(1280, 800).unwrap();
    let click = |m: &mut Model, r: Rect| m.pointer(&l, (r.x + 4, r.y + 4));
    assert!(click(&mut model, l.mcp) && model.ui.mcp_open);
    // Connect is live only while the popover is open and stays inert (as the
    // old disabled button did): it dispatches no command.
    let c = l.mcp_connect;
    assert_eq!(
        l.hit_test_with((c.x + 2, c.y + 2), true),
        Some(Target::Button(Button::Connect))
    );
    assert!(!click(&mut model, c) && model.ui.mcp_open);
    assert!(click(&mut model, l.stage) && !model.ui.mcp_open);
}

#[test]
fn empty_state_is_shown_in_the_selection_section_only() {
    use crate::chrome::{Inspector, ShapeProps};
    use crate::inspector::section_empty_text;
    let shape = ShapeProps {
        index: 1,
        fill: None,
        stroke: None,
        stroke_width: 0.0,
        transform: vexel_geometry::affine_identity(),
    };
    for i in 0..SECTIONS {
        let text = section_empty_text(i, &Inspector::NoSelection);
        assert_eq!(text.is_some(), i == 0, "section {i}");
        assert!(section_empty_text(i, &Inspector::Shape(shape)).is_none());
    }
    // Rows are 28 px tall with 8 px padding: Stroke and Transform hold two.
    assert!(SECTION_BODY_H[2] >= 8 + 28 + 8 + 28);
    assert!(SECTION_BODY_H[3] >= 8 + 28 + 8 + 28);
    assert!(SECTION_BODY_H[1] >= 8 + 28);
}

#[test]
fn tooltips_name_every_tool_and_toolbar_button() {
    let l = Layout::new(1280, 800, Mode::Draw).unwrap();
    for (t, r) in targets(&l) {
        if matches!(t, Target::Tool(_) | Target::Button(_)) {
            let (name, _) = tooltip(t).unwrap_or_else(|| panic!("{t:?}"));
            assert!(!name.is_empty());
            let _ = r;
        }
    }
}
