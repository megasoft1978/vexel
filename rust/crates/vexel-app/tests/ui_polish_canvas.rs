//! WO-RUST-UI-POLISH-01 acceptance 1 (Fit), 2 (zoom readout) and 3 (well).
use vexel_app::canvas::{FIT_MARGIN, FIT_MAX_ZOOM, MIN_ZOOM, View, controls, percent_label};
use vexel_app::chrome::{AppState, chrome_text};
use vexel_app::compose::DocumentView;
use vexel_app::model::Model;
use vexel_app::render::render;
use vexel_app::session::AppDocument;
use vexel_ui::{Theme, theme_tokens};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as f64 / (1u64 << 31) as f64
    }
}

fn check_fit(stage: (f64, f64), doc: (f64, f64)) {
    let view = View::fit(stage, doc);
    assert!(view.zoom.is_finite() && view.pan.0.is_finite() && view.pan.1.is_finite());
    let want = ((stage.0 - 2.0 * FIT_MARGIN) / doc.0)
        .min((stage.1 - 2.0 * FIT_MARGIN) / doc.1)
        .min(FIT_MAX_ZOOM)
        .max(MIN_ZOOM);
    assert_eq!(view.zoom, want, "{stage:?} {doc:?}");
    if want == MIN_ZOOM && (stage.0 - 2.0 * FIT_MARGIN) / doc.0 < MIN_ZOOM {
        return;
    }
    let (x0, y0, x1, y1) = view.artboard(doc);
    assert!(x0 >= FIT_MARGIN - 1e-6 && y0 >= FIT_MARGIN - 1e-6);
    assert!(x1 <= stage.0 - FIT_MARGIN + 1e-6 && y1 <= stage.1 - FIT_MARGIN + 1e-6);
    assert!((x0 - (stage.0 - x1)).abs() < 1e-6 && (y0 - (stage.1 - y1)).abs() < 1e-6);
}

#[test]
fn fit_is_min_of_both_axes_and_400_percent_and_centred() {
    for doc in [
        (160.0, 120.0),
        (40.0, 40.0),
        (1000.0, 800.0),
        (4000.0, 3000.0),
    ] {
        for stage in [
            (260.0, 140.0),
            (420.0, 300.0),
            (1088.0, 756.0),
            (3000.0, 2000.0),
        ] {
            if stage.0 > 2.0 * FIT_MARGIN + doc.0 * MIN_ZOOM
                && stage.1 > 2.0 * FIT_MARGIN + doc.1 * MIN_ZOOM
            {
                check_fit(stage, doc);
            }
        }
    }
    let v = View::fit((1400.0, 900.0), (160.0, 120.0));
    assert_eq!((v.zoom, v.percent()), (4.0, 400));
}

#[test]
fn five_thousand_seeded_cases_never_panic_or_yield_nan() {
    let mut rng = Lcg(0x5eed);
    for _ in 0..5000 {
        let stage = (260.0 + rng.next() * 2740.0, 140.0 + rng.next() * 1860.0);
        let doc = (1.0 + rng.next() * 3999.0, 1.0 + rng.next() * 2999.0);
        let v = View::fit(stage, doc);
        assert!(v.zoom.is_finite() && v.pan.0.is_finite() && v.pan.1.is_finite());
        assert!((MIN_ZOOM..=FIT_MAX_ZOOM).contains(&v.zoom));
    }
}

#[test]
fn status_zoom_equals_the_cluster_label_in_every_zoom_state() {
    let app = AppDocument::new(160, 120).unwrap();
    let (stage, art) = ((1000.0, 700.0), (160.0, 120.0));
    let mut states = Vec::new();
    let mut view = DocumentView::new();
    view.canvas.set_fit(stage, art);
    states.push(view.canvas.clone());
    view.canvas.set_actual(stage, art);
    states.push(view.canvas.clone());
    view.canvas.zoom_by((100.0, 100.0), 1.37);
    states.push(view.canvas.clone());
    view.canvas.zoom_step((100.0, 100.0), 1);
    states.push(view.canvas.clone());
    view.canvas.zoom_step((100.0, 100.0), -1);
    states.push(view.canvas.clone());
    for canvas in states {
        let mut v = DocumentView::new();
        v.canvas = canvas;
        let state = AppState::from_session(&app, &v);
        let cluster = controls(400, 300, v.canvas.view.percent()).unwrap();
        assert_eq!(percent_label(state.zoom_percent), cluster.label);
        assert!(!cluster.label.is_empty() && cluster.label.ends_with('%'));
    }
}

fn pixel(frame: &vexel_platform_apple::RgbaFrame, x: u32, y: u32) -> u32 {
    let at = ((y * frame.width() + x) * 4) as usize;
    let p = &frame.pixels()[at..at + 4];
    u32::from_be_bytes([p[0], p[1], p[2], 255])
}

fn luma(c: u32) -> i64 {
    (299 * i64::from(c >> 24 & 255)
        + 587 * i64::from(c >> 16 & 255)
        + 114 * i64::from(c >> 8 & 255))
        / 1000
}

fn contrast(a: u32, b: u32) -> f64 {
    let lin = |c: u32| {
        let v = f64::from(c) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = |c: u32| {
        0.2126 * lin(c >> 24 & 255) + 0.7152 * lin(c >> 16 & 255) + 0.0722 * lin(c >> 8 & 255)
    };
    let (x, y) = (l(a), l(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn light_well_differs_from_the_panel_and_both_themes_have_a_border_edge() {
    for dark in [false, true] {
        let mut model = Model::new(dark);
        model.chrome = chrome_text(&AppState::default());
        let l = model.layout(1280, 800).unwrap();
        let frame = render(1280, 800, 1, &model).unwrap();
        let t = theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false);
        let s = l.stage;
        let well = pixel(&frame, s.x + 30, s.y + 30);
        if !dark {
            assert!(
                luma(t.panel) - luma(well) >= 6 || luma(well) - luma(t.panel) >= 6,
                "well {well:08x} vs panel {:08x}",
                t.panel
            );
            assert_eq!(well, t.surface[2]);
        }
        // The panel colour is what the inspector shows.
        let panel = l.inspector.unwrap();
        assert_eq!(pixel(&frame, panel.x + 200, panel.y + 500), t.panel);
        // 1 px `border` edge on all four sides of the stage.
        for x in [s.x, s.x + 5, s.x + s.width - 1] {
            assert_eq!(pixel(&frame, x, s.y), t.border, "top edge at {x}");
            assert_eq!(pixel(&frame, x, s.y + s.height - 1), t.border, "bottom");
        }
        for y in [s.y, s.y + 5, s.y + s.height - 1] {
            assert_eq!(pixel(&frame, s.x, y), t.border, "left edge at {y}");
            assert_eq!(pixel(&frame, s.x + s.width - 1, y), t.border, "right");
        }
        assert_ne!(pixel(&frame, s.x + 1, s.y + 1), t.border, "edge is 1 px");
        // Text over the well keeps its contrast: the empty-state title is drawn
        // with text_primary over the well, which is lighter or equal to it by far.
        assert!(contrast(t.text_primary, well) >= 7.0);
        assert!(contrast(t.text_secondary, well) >= 4.5);
    }
}
