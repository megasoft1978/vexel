//! Headless renders for review: the transport bar with the Add key button
//! (enabled, hovered, disabled) and a shape rotated twice. Writes PNGs to
//! `VEXEL_SHOTS_DIR` (default `/private/tmp/vexel-uifix-shots`).
use vexel_app::compose::compose_document;
use vexel_app::model::{Mode, Model};
use vexel_app::native_timeline::layout::{Hit, TimelineLayout, Viewport};
use vexel_app::native_timeline::model::TimelineModel;
use vexel_app::native_timeline::presentation::TimelinePresentation;
use vexel_app::native_timeline::{
    NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind,
};
use vexel_app::render::render;
use vexel_app::tools::{PointerInput, ScaleMap, ToolController};
use vexel_platform_apple::RgbaFrame;
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::Rect;
use vexel_v1_document::V1Document;

fn dir() -> String {
    let dir = std::env::var("VEXEL_SHOTS_DIR").unwrap_or("/private/tmp/vexel-uifix-shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_rgba(name: &str, w: u32, h: u32, px: &[u8]) {
    let png = encode_rgba8(w, h, px, Compression::Fixed).unwrap();
    std::fs::write(format!("{}/{name}.png", dir()), png).unwrap();
}

fn crop(name: &str, frame: &RgbaFrame, r: Rect) {
    let w = frame.width();
    let mut out = Vec::new();
    for y in r.y..r.y + r.height {
        let a = ((y * w + r.x) * 4) as usize;
        out.extend_from_slice(&frame.pixels()[a..a + (r.width * 4) as usize]);
    }
    write_rgba(name, r.width, r.height, &out);
}

fn snapshot(with_selection: bool) -> NativeTimelineSnapshot {
    let key = |tick, label: &str| NativeTimelineItem {
        tick,
        label: label.into(),
        selected: false,
        ease: Some("linear".into()),
    };
    NativeTimelineSnapshot {
        ticks_per_second: 24,
        duration_ticks: 48,
        current_tick: 12,
        tracks: if with_selection {
            vec![NativeTimelineTrack {
                id: "opacity_0".into(),
                name: "Opacity".into(),
                target: "hero".into(),
                property: "opacity".into(),
                kind: NativeTrackKind::Property,
                visible: true,
                selected: true,
                items: vec![key(0, "1"), key(12, "0.75"), key(36, "0")],
            }]
        } else {
            vec![]
        },
        selected_target: with_selection.then(|| "hero".to_owned()),
        playing: false,
        loop_enabled: true,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
        can_add_drawing_track: with_selection,
        can_import_asset: with_selection,
        can_edit_selected_item: false,
        status: None,
    }
}

fn transport(name: &str, with_selection: bool, hover: Option<Hit>) -> (RgbaFrame, Rect) {
    let mut model = Model::new(true);
    model.mode = Mode::Animate;
    let layout = model.layout(1280, 800).unwrap();
    let footer = layout.footer.unwrap();
    let raw = render(1280, 800, 1, &model).unwrap();
    let mut pres = TimelinePresentation::default();
    pres.interaction.hover = hover;
    let snap = snapshot(with_selection);
    let frame = vexel_app::overlay::paint_over(raw, true, |c| pres.paint(c, footer, &snap));
    let bar = Rect {
        x: footer.x,
        y: footer.y,
        width: footer.width.min(760),
        height: 40,
    };
    crop(name, &frame, bar);
    (frame, footer)
}

#[test]
fn add_key_button_shots_and_states() {
    let (on, footer) = transport("addkey-enabled", true, None);
    transport("addkey-hover", true, Some(Hit::AddKey));
    let (off, _) = transport("addkey-disabled", false, None);
    let model = TimelineModel::from_snapshot(&snapshot(true));
    let layout = TimelineLayout::new(footer, &model, Viewport::full(&model));
    let (_, r) = layout
        .control_rects()
        .into_iter()
        .find(|(hit, _)| *hit == Hit::AddKey)
        .unwrap();
    let cell = |frame: &RgbaFrame| {
        let mut out = Vec::new();
        for y in r.y..r.y + r.height {
            let a = ((y * frame.width() + r.x) * 4) as usize;
            out.extend_from_slice(&frame.pixels()[a..a + (r.width * 4) as usize]);
        }
        out
    };
    assert_ne!(cell(&on), cell(&off), "disabled state is visibly different");
}

#[test]
fn rotated_twice_shape_shot() {
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"150\" viewBox=\"0 0 200 150\"><rect id=\"r\" x=\"70\" y=\"55\" width=\"60\" height=\"40\" fill=\"#e53935\" stroke=\"#1a237e\" stroke-width=\"3\"/></svg>";
    let mut doc = V1Document::from_source(svg.as_bytes()).unwrap();
    let mut c = ToolController::new();
    let map = ScaleMap::identity(200.0, 150.0).unwrap();
    for delta in [30.0_f64, 25.0] {
        c.select_id(&doc, "r").unwrap();
        let b = vexel_app_frame_bounds(&doc);
        let (cx, cy) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
        let down = (b.0 - 9.0, b.1 - 9.0);
        let a0 = (down.1 - cy).atan2(down.0 - cx);
        let radius = (down.0 - cx).hypot(down.1 - cy);
        let end = (
            cx + radius * (a0 + delta.to_radians()).cos(),
            cy + radius * (a0 + delta.to_radians()).sin(),
        );
        for input in [
            PointerInput::Down {
                x: down.0,
                y: down.1,
            },
            PointerInput::Move { x: end.0, y: end.1 },
            PointerInput::Up { x: end.0, y: end.1 },
        ] {
            c.pointer(input, &mut doc, &map).unwrap();
        }
    }
    let source = doc.source().unwrap();
    let composed = compose_document(&source, 200, 150, &|| false).unwrap();
    let f = &composed.frame;
    write_rgba("rotated-twice", f.width, f.height, &f.rgba);
    let m = doc.get_transform_matrix("r").unwrap();
    let angle = m[1].atan2(m[0]).to_degrees();
    assert!((angle - 55.0).abs() < 1.0, "30 + 25 degrees, got {angle}");
}

/// Document-space bounds of the shape's selection frame.
fn vexel_app_frame_bounds(doc: &V1Document) -> (f64, f64, f64, f64) {
    if let Ok(b) = doc.geometry_bounds("r") {
        return b;
    }
    // Rotated: the axis-aligned box of the 60x40 rect around its centre.
    let m = doc.get_transform_matrix("r").unwrap();
    let pts = [(70.0, 55.0), (130.0, 55.0), (130.0, 95.0), (70.0, 95.0)]
        .map(|(x, y)| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]));
    let fold = |f: fn(f64, f64) -> f64, i: usize, init: f64| {
        pts.iter()
            .map(|p| if i == 0 { p.0 } else { p.1 })
            .fold(init, f)
    };
    (
        fold(f64::min, 0, f64::INFINITY),
        fold(f64::min, 1, f64::INFINITY),
        fold(f64::max, 0, f64::NEG_INFINITY),
        fold(f64::max, 1, f64::NEG_INFINITY),
    )
}
