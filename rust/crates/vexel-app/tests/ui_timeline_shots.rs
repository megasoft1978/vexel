//! Headless renders of the Animate shell (inspector plus native timeline) as
//! PNGs for visual review. Writes to `VEXEL_SHOTS_DIR` (default
//! `/private/tmp/vexel-tl-shots`); the assertions only pin geometry facts.
use vexel_app::chrome::{Inspector, ShapeExt, ShapeProps};
use vexel_app::model::{Mode, Model};
use vexel_app::native_timeline::layout::Hit;
use vexel_app::native_timeline::presentation::TimelinePresentation;
use vexel_app::native_timeline::{
    NativeTimelineItem, NativeTimelineSnapshot, NativeTimelineTrack, NativeTrackKind,
};
use vexel_app::render::render;
use vexel_app::session::AppDocument;
use vexel_platform_apple::RgbaFrame;
use vexel_png::{Compression, encode_rgba8};
use vexel_ui::Rect;
use vexel_v1_document::V1Document;

const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="24" viewBox="0 0 32 24"><rect id="hero" x="2" y="2" width="12" height="10" fill="#e53935" stroke="#1a237e" stroke-width="2"/></svg>"##;

fn key(tick: u32, v: &str, ease: Option<&str>, selected: bool) -> NativeTimelineItem {
    NativeTimelineItem {
        tick,
        label: v.into(),
        selected,
        ease: ease.map(str::to_owned),
    }
}

fn prop(
    id: &str,
    name: &str,
    target: &str,
    prop: &str,
    items: Vec<NativeTimelineItem>,
) -> NativeTimelineTrack {
    NativeTimelineTrack {
        id: id.into(),
        name: name.into(),
        target: target.into(),
        property: prop.into(),
        kind: NativeTrackKind::Property,
        visible: !id.ends_with('6'),
        selected: items.iter().any(|i| i.selected),
        items,
    }
}

fn snapshot(select: bool) -> NativeTimelineSnapshot {
    let props = [
        "translate x",
        "translate y",
        "rotate",
        "scale x",
        "scale y",
        "opacity",
        "fill",
        "stroke width",
    ];
    let mut tracks: Vec<NativeTimelineTrack> = props
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let n = i as u32;
            let items = vec![
                key(0, "0", Some("linear"), false),
                key(8 + n, "1", Some("ease_in_out"), n == 2 && false),
                key(20 + n * 2, "2", Some("ease_out"), i == 2),
                key(40, "3", Some("step"), false),
            ];
            prop(
                &format!("t{}", i + 1),
                &format!("t{}", i + 1),
                if i < 5 { "hero" } else { "logo" },
                p,
                items,
            )
        })
        .collect();
    tracks[2].selected = select;
    tracks[2].items[2].selected = select;
    tracks[2].items[2].ease = Some("bezier:0.25,0.1,0.25,1".into());
    NativeTimelineSnapshot {
        ticks_per_second: 24,
        duration_ticks: 48,
        current_tick: 14,
        tracks,
        selected_target: Some("hero".into()),
        playing: false,
        loop_enabled: true,
        onion_before: 1,
        onion_after: 0,
        onion_opacity: 0.25,
        can_add_drawing_track: true,
        can_import_asset: true,
        can_edit_selected_item: true,
        status: None,
    }
}

fn paint_timeline(
    frame: RgbaFrame,
    footer: Rect,
    snap: &NativeTimelineSnapshot,
    pres: &TimelinePresentation,
) -> RgbaFrame {
    vexel_app::overlay::paint_over(frame, true, |c| pres.paint(c, footer, snap))
}

fn write(name: &str, frame: &RgbaFrame, crop: Option<Rect>) {
    let dir = std::env::var("VEXEL_SHOTS_DIR").unwrap_or("/private/tmp/vexel-tl-shots".into());
    std::fs::create_dir_all(&dir).unwrap();
    let (w, h, px) = (frame.width(), frame.height(), frame.pixels());
    let s = frame.backing_scale() as u32;
    let (cx, cy, cw, ch) = crop.map_or((0, 0, w, h), |r| {
        (r.x * s, r.y * s, r.width * s, r.height * s)
    });
    let mut out = Vec::with_capacity((cw * ch * 4) as usize);
    for y in cy..cy + ch {
        let a = ((y * w + cx) * 4) as usize;
        out.extend_from_slice(&px[a..a + (cw * 4) as usize]);
    }
    let png = encode_rgba8(cw, ch, &out, Compression::Fixed).unwrap();
    std::fs::write(format!("{dir}/{name}.png"), png).unwrap();
}

fn window(bezier: bool, hover: Option<Hit>, select: bool, footer_h: u32, name: &str) {
    let size = (1280, 800);
    let doc = AppDocument::from_document(V1Document::from_source(SVG).unwrap()).unwrap();
    let mut model = Model::new(true);
    model.mode = Mode::Animate;
    model.chrome.state.ext = ShapeExt::read(doc.document(), "hero");
    model.chrome.inspector = Inspector::Shape(ShapeProps {
        index: 0,
        fill: Some([229, 57, 53, 255]),
        stroke: Some([26, 35, 126, 255]),
        stroke_width: 2.0,
        transform: vexel_geometry::affine_identity(),
    });
    model.ui.collapsed = [false; 5];
    model.ui.set_footer_height(footer_h, 600);
    let layout = model.layout(size.0, size.1).unwrap();
    let footer = layout.footer.unwrap();
    let raw = render(size.0, size.1, 1, &model).unwrap();
    let mut pres = TimelinePresentation::default();
    if name.contains("smil") {
        pres.mode = vexel_app::native_timeline::presentation::TimelineMode::SvgSmil;
    }
    pres.interaction.bezier.open = bezier;
    pres.interaction.hover = hover;
    let snap = snapshot(select);
    let frame = paint_timeline(raw, footer, &snap, &pres);
    write(&format!("{name}-window"), &frame, None);
    write(&format!("{name}-timeline"), &frame, Some(footer));
}

#[test]
fn shots() {
    let tag = std::env::var("VEXEL_SHOT_TAG").unwrap_or("after".into());
    window(false, None, true, 340, &format!("{tag}-key"));
    window(
        true,
        Some(Hit::BezierStep { field: 1, up: true }),
        true,
        340,
        &format!("{tag}-bezier"),
    );
    window(
        false,
        Some(Hit::Visibility(1)),
        false,
        340,
        &format!("{tag}-nokey"),
    );
    window(false, None, true, 200, &format!("{tag}-default"));
    window(false, None, false, 200, &format!("{tag}-smil"));
}

fn model_of(select: bool) -> vexel_app::native_timeline::model::TimelineModel {
    vexel_app::native_timeline::model::TimelineModel::from_snapshot(&snapshot(select))
}

fn panel(h: u32) -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 900,
        height: h,
    }
}

fn disjoint(a: Rect, b: Rect) -> bool {
    a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y
}

#[test]
fn key_bar_appears_only_for_a_property_key_and_never_moves_rows() {
    use vexel_app::native_timeline::layout::{TimelineLayout, Viewport};
    let (none, some) = (model_of(false), model_of(true));
    let a = TimelineLayout::new(panel(340), &none, Viewport::full(&none));
    let b = TimelineLayout::new(panel(340), &some, Viewport::full(&some));
    assert_eq!(a.keybar.height, 0);
    assert_eq!(b.keybar.height, 36);
    assert_eq!(a.rows.y, b.rows.y, "rows keep their top edge");
    assert_eq!(a.row_rect(0), b.row_rect(0));
    assert_eq!(
        b.keybar.y + b.keybar.height,
        340,
        "bar sits at the bottom edge"
    );
}

#[test]
fn easing_cells_are_labelled_slots_inside_the_bar_and_hit_their_presets() {
    use vexel_app::native_timeline::layout::{TimelineLayout, Viewport};
    let m = model_of(true);
    let l = TimelineLayout::new(panel(340), &m, Viewport::full(&m));
    let cells = l.ease_cells();
    for (i, (hit, r)) in cells.iter().enumerate() {
        assert!(r.x >= l.keybar.x && r.x + r.width <= l.keybar.x + l.keybar.width);
        assert!(r.y >= l.keybar.y && r.y + r.height <= l.keybar.y + l.keybar.height);
        assert_eq!(
            l.hit_test(&m, (r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32),
            *hit
        );
        for (_, o) in cells.iter().skip(i + 1) {
            assert!(disjoint(*r, *o));
        }
    }
    assert_eq!(cells[0].0, Hit::Ease("linear"));
    assert_eq!(cells[5].0, Hit::EaseCustom);
}

#[test]
fn bezier_editor_fits_above_the_bar_and_cancel_is_hit_testable() {
    use vexel_app::native_timeline::layout::{TimelineLayout, Viewport};
    let m = model_of(true);
    for h in [200, 260, 340] {
        let l = TimelineLayout::new(panel(h), &m, Viewport::full(&m));
        let e = l.bezier_editor().expect("editor fits");
        assert!(e.y >= l.panel.y && e.y + e.height <= l.keybar.y, "h={h}");
        assert!(e.x >= l.panel.x && e.x + e.width <= l.panel.x + l.panel.width);
        let cancel = l.bezier_cancel_rect().unwrap();
        let apply = l.bezier_apply_rect().unwrap();
        assert!(disjoint(cancel, apply));
        let mid = |r: Rect| ((r.x + r.width / 2) as i32, (r.y + r.height / 2) as i32);
        assert_eq!(
            l.bezier_hit(mid(cancel).0, mid(cancel).1),
            Some(Hit::BezierCancel)
        );
        assert_eq!(
            l.bezier_hit(mid(apply).0, mid(apply).1),
            Some(Hit::BezierApply)
        );
        for field in 0..4u8 {
            let cell = l.bezier_cell_rect(field).unwrap();
            for up in [false, true] {
                let s = l.bezier_step_rect(field, up).unwrap();
                assert!(s.x >= cell.x && s.x + s.width <= cell.x + cell.width);
                assert_eq!(
                    l.bezier_hit(mid(s).0, mid(s).1),
                    Some(Hit::BezierStep { field, up })
                );
            }
        }
    }
    assert!(
        TimelineLayout::new(panel(150), &m, Viewport::full(&m))
            .bezier_editor()
            .is_none()
    );
}

#[test]
fn keys_are_centred_on_their_tick_and_win_over_the_name_column_at_tick_zero() {
    use vexel_app::native_timeline::layout::{TimelineLayout, Viewport};
    let m = model_of(false);
    let l = TimelineLayout::new(panel(340), &m, Viewport::full(&m));
    let r = l.item_rect(&m, 0, 1).unwrap();
    assert_eq!(
        r.x + r.width / 2,
        l.lane_x(m.tracks[0].items[1].tick).round() as u32
    );
    let first = l.item_rect(&m, 0, 0).unwrap();
    let probe = ((first.x + 2) as i32, (first.y + first.height / 2) as i32);
    assert_eq!(
        l.hit_test(&m, probe.0, probe.1),
        Hit::Item { track: 0, item: 0 }
    );
    assert_eq!(
        l.hit_test(&m, 16, (l.row_rect(0).y + 10) as i32),
        Hit::Visibility(0)
    );
    assert_eq!(
        l.hit_test(&m, 100, (l.row_rect(0).y + 10) as i32),
        Hit::Track(0)
    );
}
