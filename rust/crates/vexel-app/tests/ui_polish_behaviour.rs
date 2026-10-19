//! WO-RUST-UI-POLISH-01 acceptance 4 (inspector), 5 (motion coverage),
//! 6 (tooltips), 7 (status) and 8 (history).
use std::cell::Cell;
use std::rc::Rc;
use vexel_app::chrome::{AppState, Inspector, ShapeProps, chrome_text};
use vexel_app::history::{self, Availability};
use vexel_app::layout::{
    Button, EMPTY_CARD_BODY_H, Layout, SECTION_BODY_H, SECTIONS, Splitter, UiState, status_hint,
    tooltip, tooltip_rect,
};
use vexel_app::model::{Mode, Model, Target, Tool};
use vexel_app::motion::{MotionAdapter, REGISTERED, colors_for, control_id};
use vexel_app::overlay::overlay_tooltip;
use vexel_app::render::render;
use vexel_app::session::{AppDocument, EditAction};
use vexel_platform_apple::PointerKind;
use vexel_ui::motion::Clock;
use vexel_ui::{Rect, Theme, theme_tokens};

#[derive(Clone)]
struct TestClock(Rc<Cell<u64>>);
impl Clock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.get()
    }
}
const MS: u64 = 1_000_000;

fn shape() -> Inspector {
    Inspector::Shape(ShapeProps {
        index: 1,
        fill: Some([1, 2, 3, 255]),
        stroke: None,
        stroke_width: 0.0,
        transform: vexel_geometry::affine_identity(),
    })
}

fn open_model(mode: Mode, selected: bool) -> Model {
    let mut model = Model::new(false);
    model.mode = mode;
    model.chrome = chrome_text(&AppState {
        open: true,
        dirty: true,
        name: Some("a.svg".into()),
        size: (160, 120),
        zoom_percent: 400,
        selection: selected.then(|| match shape() {
            Inspector::Shape(p) => p,
            Inspector::NoSelection => unreachable!(),
        }),
        ..AppState::default()
    });
    model
}

fn pixel(f: &vexel_platform_apple::RgbaFrame, x: u32, y: u32) -> u32 {
    let at = ((y * f.width() + x) * 4) as usize;
    let p = &f.pixels()[at..at + 4];
    u32::from_be_bytes([p[0], p[1], p[2], 255])
}

fn inside(r: Rect, p: (u32, u32)) -> bool {
    p.0 >= r.x && p.1 >= r.y && p.0 < r.x + r.width && p.1 < r.y + r.height
}

fn centre(r: Rect) -> (i32, i32) {
    ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32)
}

// ---- 4: inspector ----

#[test]
fn empty_selection_lays_out_one_card_and_selection_restores_the_rows() {
    let empty = open_model(Mode::Draw, false).layout(1280, 820).unwrap();
    assert_eq!(empty.section_bodies[0].height, EMPTY_CARD_BODY_H);
    for i in 1..=3 {
        assert_eq!(empty.section_headers[i].width, 0, "section {i} header");
        assert_eq!(empty.section_bodies[i].height, 0, "section {i} body");
    }
    assert!(empty.section_headers[4].width > 0, "Animation stays");
    let selected = open_model(Mode::Draw, true).layout(1280, 820).unwrap();
    assert_eq!(selected, Layout::new(1280, 820, Mode::Draw).unwrap());
    for (i, h) in [(0, 52), (1, 44), (2, 76)] {
        assert_eq!(selected.section_bodies[i].height, h);
        assert_eq!(SECTION_BODY_H[i], h);
    }
    // Header hit rects keep their size and the hit target.
    for i in [0, 4] {
        let r = empty.section_headers[i];
        assert_eq!(r.height, 32);
        let (x, y) = centre(r);
        assert_eq!(
            empty.hit_test((x as u32, y as u32)),
            Some(Target::SectionHeader(i))
        );
    }
    assert_eq!(SECTIONS, 5);
    // Layout built from a default UiState is the selected geometry.
    assert_eq!(
        Layout::with_state(1280, 820, Mode::Draw, &UiState::default()).unwrap(),
        selected
    );
}

#[test]
fn headers_have_no_band_at_rest() {
    for dark in [false, true] {
        for selected in [false, true] {
            let mut model = open_model(Mode::Draw, selected);
            model.dark = dark;
            let l = model.layout(1280, 820).unwrap();
            let t = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
            let f = render(1280, 820, 1, &model).unwrap();
            for head in l.section_headers.iter().filter(|h| h.width > 0) {
                assert_eq!(pixel(&f, head.x + 200, head.y + 16), t.panel, "{dark}");
            }
        }
    }
}

// ---- 5: motion coverage ----

fn all_targets() -> Vec<Target> {
    let mut v = vec![Target::DrawMode, Target::AnimateMode];
    v.extend(
        [
            Tool::Select,
            Tool::Pen,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Line,
        ]
        .map(Target::Tool),
    );
    v.extend(
        [
            Button::Undo,
            Button::Redo,
            Button::Save,
            Button::Export,
            Button::ToggleInspector,
            Button::Mcp,
            Button::Connect,
        ]
        .map(Target::Button),
    );
    // Pin updated (WO-RUST-UI-WIRING-01): the Export popover rows are registered too.
    v.extend(vexel_app::model::ExportFormat::ALL.map(Target::MenuItem));
    v.extend((0..8).map(Target::SectionHeader));
    v.extend([Splitter::Inspector, Splitter::Footer].map(Target::Splitter));
    v
}

#[test]
fn every_target_has_a_unique_registered_id() {
    let ids: Vec<u32> = all_targets().into_iter().map(control_id).collect();
    for id in &ids {
        assert!(REGISTERED.contains(id), "{id} not registered");
    }
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "ids are unique");
    assert_eq!(sorted.len(), REGISTERED.len(), "no unused registry slot");
}

fn adapter() -> (MotionAdapter<TestClock>, Rc<Cell<u64>>) {
    let time = Rc::new(Cell::new(1_000_000_000));
    (
        MotionAdapter::with_clock(TestClock(time.clone()), false),
        time,
    )
}

#[test]
fn hovering_every_chrome_control_changes_its_fill_except_disabled_buttons() {
    let model = open_model(Mode::Animate, true);
    let l = model.layout(1280, 820).unwrap();
    let t = theme_tokens(Theme::Light, false);
    let mut seen = 0;
    for target in all_targets() {
        let Some(rect) = l.target_rect(target) else {
            continue;
        };
        if rect.width == 0 || matches!(target, Target::Button(Button::Connect)) {
            continue;
        }
        let (mut m, time) = adapter();
        let (x, y) = centre(rect);
        let (hit, _) = m.pointer_in(&l, PointerKind::Move, (x, y), true, &model);
        assert_eq!(hit, l.hit_test((x as u32, y as u32)));
        time.set(time.get() + 130 * MS);
        let id = control_id(target);
        let visual = m.frame(false, Mode::Animate).get(id);
        let selected = target == Target::AnimateMode;
        let rest = colors_for(&t, id, selected).rest_fill;
        let enabled = model.target_enabled(&l, target);
        if hit == Some(target) {
            seen += 1;
            if enabled && !selected {
                assert_ne!(visual.fill, rest, "{target:?} hover fill");
            }
            if !enabled {
                assert_eq!(visual.hover, 0.0, "{target:?} disabled never hovers");
            }
        }
    }
    assert!(seen >= 20, "{seen}");
    assert!(!model.target_enabled(&l, Target::Button(Button::Redo)));
}

#[test]
fn a_point_under_the_open_popover_hits_the_popover_not_the_chrome_beneath() {
    let mut found = 0;
    for (w, h) in [(900, 520), (1000, 560), (1280, 600)] {
        let mut model = open_model(Mode::Draw, true);
        let l = model.layout(w, h).unwrap();
        model.ui.mcp_open = true;
        let p = l.mcp_popover;
        for y in (p.y..p.y + p.height).step_by(3) {
            for x in (p.x..p.x + p.width).step_by(3) {
                if l.hit_test((x, y)).is_some() && !inside(l.mcp_connect, (x, y)) {
                    found += 1;
                    assert_eq!(l.hit_test_with((x, y), true), None);
                    let (mut m, time) = adapter();
                    let (hit, _) =
                        m.pointer_in(&l, PointerKind::Move, (x as i32, y as i32), true, &model);
                    time.set(time.get() + 130 * MS);
                    assert_eq!(hit, None);
                    assert_eq!(m.registry().hovered(), None);
                }
            }
        }
    }
    assert!(found > 0, "the popover must overlap some chrome");
}

// ---- 6: tooltips ----

#[test]
fn tooltip_appears_after_500_ms_and_goes_on_press_or_leave() {
    let model = open_model(Mode::Draw, true);
    let l = model.layout(1280, 800).unwrap();
    let (mut m, time) = adapter();
    let p = centre(l.tools[1]);
    m.pointer_in(&l, PointerKind::Move, p, true, &model);
    assert_eq!(m.tooltip(), None);
    time.set(time.get() + 499 * MS);
    assert_eq!(m.tooltip(), None);
    assert!(!m.poll_tooltip());
    time.set(time.get() + MS);
    let target = m.tooltip().expect("shown at 500 ms");
    assert_eq!(target, Target::Tool(Tool::Pen));
    assert_eq!(tooltip(target), Some(("Pen", "P")));
    assert_eq!(m.frame(false, Mode::Draw).tooltip, Some(target));
    assert!(m.poll_tooltip());
    // Press dismisses it, and it stays gone while the pointer rests.
    m.pointer_in(&l, PointerKind::Down, p, true, &model);
    assert_eq!(m.tooltip(), None);
    time.set(time.get() + 900 * MS);
    assert_eq!(m.tooltip(), None);
    assert!(m.poll_tooltip());
    // Leaving also clears it; a new hover waits again.
    m.pointer_in(&l, PointerKind::Up, p, true, &model);
    m.pointer_in(&l, PointerKind::Move, centre(l.tools[2]), true, &model);
    time.set(time.get() + 600 * MS);
    assert_eq!(m.tooltip(), Some(Target::Tool(Tool::Rectangle)));
    m.pointer_in(&l, PointerKind::Move, (700, 400), true, &model);
    assert_eq!(m.tooltip(), None);
}

#[test]
fn reduced_motion_still_delays_the_tooltip() {
    let model = open_model(Mode::Draw, true);
    let l = model.layout(1280, 800).unwrap();
    let time = Rc::new(Cell::new(5 * 1_000_000_000));
    let mut m = MotionAdapter::with_clock(TestClock(time.clone()), true);
    m.pointer_in(&l, PointerKind::Move, centre(l.buttons[2]), true, &model);
    assert_eq!(m.tooltip(), None);
    time.set(time.get() + 500 * MS);
    assert_eq!(m.tooltip(), Some(Target::Button(Button::Save)));
}

#[test]
fn tooltip_rects_stay_inside_the_window_for_every_target() {
    for (w, h) in [(720, 480), (1280, 800)] {
        for mode in [Mode::Draw, Mode::Animate] {
            let l = Layout::new(w, h, mode).unwrap();
            let mut n = 0;
            for target in all_targets() {
                let Some(r) = tooltip_rect(&l, target) else {
                    assert!(tooltip(target).is_none() || l.target_rect(target).is_none());
                    continue;
                };
                n += 1;
                assert!(
                    r.x + r.width <= w && r.y + r.height <= h,
                    "{target:?} {r:?}"
                );
            }
            assert_eq!(n, 11, "5 tools + 6 buttons");
        }
    }
}

#[test]
fn tooltip_frame_differs_only_inside_the_tooltip_rect() {
    for scale in [1u32, 2] {
        for dark in [false, true] {
            let mut model = open_model(Mode::Draw, true);
            model.dark = dark;
            let l = model.layout(1280, 800).unwrap();
            for target in [Target::Tool(Tool::Ellipse), Target::Button(Button::Export)] {
                let rect = tooltip_rect(&l, target).unwrap();
                let base = render(1280, 800, scale, &model).unwrap();
                let before = base.pixels().to_vec();
                let width = base.width();
                let after = overlay_tooltip(base, &model, &l, Some(target));
                let mut changed = 0;
                for (i, (a, b)) in before
                    .chunks_exact(4)
                    .zip(after.pixels().chunks_exact(4))
                    .enumerate()
                {
                    if a != b {
                        changed += 1;
                        let (x, y) = (i as u32 % width / scale, i as u32 / width / scale);
                        assert!(
                            x >= rect.x
                                && y >= rect.y
                                && x < rect.x + rect.width
                                && y < rect.y + rect.height,
                            "{target:?} pixel {x},{y} outside {rect:?}"
                        );
                    }
                }
                assert!(changed > 50, "{target:?}: {changed}");
            }
        }
    }
}

// ---- 7: status ----

#[test]
fn status_readouts_share_one_baseline_and_hints_follow_the_context() {
    for (w, h) in [(1000, 600), (1280, 800), (1920, 1080)] {
        let l = Layout::new(w, h, Mode::Draw).unwrap();
        let rects = l.status_texts();
        let mid = |r: Rect| i64::from(r.y) * 2 + i64::from(r.height);
        for r in rects {
            assert!(r.width > 0);
            assert!(
                (mid(r) - mid(rects[3])).abs() <= 2,
                "{r:?} vs {:?}",
                rects[3]
            );
            assert!(r.y >= l.status.y && r.y + r.height <= l.status.y + l.status.height);
        }
        let hint = l.status_row(l.status_hint);
        assert!((mid(hint) - mid(rects[3])).abs() <= 2);
    }
    let none = Model::new(false);
    assert_eq!(status_hint(&none), "Open an SVG to begin (Cmd+O)");
    let mut m = open_model(Mode::Draw, false);
    let want = [
        (Tool::Select, "Click a shape to select it"),
        (
            Tool::Pen,
            "Click to add points, double-click or Return to finish",
        ),
        (Tool::Rectangle, "Drag on the canvas to draw a rectangle"),
        (Tool::Ellipse, "Drag on the canvas to draw an ellipse"),
        (Tool::Line, "Drag on the canvas to draw a line"),
    ];
    for (tool, text) in want {
        m.tool = tool;
        assert_eq!(status_hint(&m), text);
    }
    m.mode = Mode::Animate;
    assert_eq!(status_hint(&m), "Space plays, drag the ruler to scrub");
}

// ---- 8: history ----

fn edit(app: &mut AppDocument, id: &str, x: f64) {
    app.edit(|d| d.add_rectangle_with_id(x, 10.0, x + 20.0, 40.0, "#ff0000", id))
        .unwrap();
}

#[test]
fn redo_is_enabled_only_after_an_undo_and_a_load_resets_it() {
    // Pin updated (WO-RUST-UI-WIRING-01): the shell keeps no redo counter any
    // more; availability is read from the core's history cursor.
    let mut app = AppDocument::new(200, 100).unwrap();
    let mut model = open_model(Mode::Draw, false);
    let l = model.layout(1280, 800).unwrap();
    let redo = |m: &Model| m.target_enabled(&l, Target::Button(Button::Redo));
    let cursor = |a: &AppDocument| (a.document().history_cursor(), a.document().history_count());
    edit(&mut app, "a", 10.0);
    edit(&mut app, "b", 50.0);
    model.sync_history(&app);
    assert!(!redo(&model));
    assert!(history::apply(&mut app, EditAction::Undo));
    model.sync_history(&app);
    assert!(redo(&model));
    assert!(history::apply(&mut app, EditAction::Undo));
    assert_eq!(cursor(&app), (0, 2));
    assert_eq!(
        app.document().history_count() - app.document().history_cursor(),
        2
    );
    assert!(history::apply(&mut app, EditAction::Redo));
    model.sync_history(&app);
    assert!(redo(&model));
    assert_eq!(cursor(&app), (1, 2));
    edit(&mut app, "c", 90.0);
    model.sync_history(&app);
    assert!(!redo(&model));
    // Redo while disabled is a no-op: the document does not change.
    let (rev, src) = (app.revision(), app.source_bytes().unwrap());
    assert!(!history::apply(&mut app, EditAction::Redo));
    assert_eq!((app.revision(), app.source_bytes().unwrap()), (rev, src));
    // Load resets.
    history::apply(&mut app, EditAction::Undo);
    assert!(Availability::of(&app).redo);
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/v1/compose/compose-v1.svg");
    app.open_path(&fixture).unwrap();
    assert_eq!(Availability::of(&app), Availability::default());
    assert!(model.sync_history(&app));
    assert!(!redo(&model));
}
