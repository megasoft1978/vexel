//! Inspector: collapsible grouped sections (Selection, Fill, Stroke,
//! Transform, Animation), read-only for now. Geometry comes from the layout
//! module; this file paints rows into the section rects.

use crate::animation::strip;
use crate::chrome::{FillKind, Inspector, ShapeProps, number};
use crate::layout::paint::{centred, mono, paint_focus, text, text_width};
use crate::layout::{EMPTY_CARD_BODY_H, Focus, Layout, SECTION_BODY_H, SECTION_NAMES, SECTIONS};
use crate::model::Model;
use crate::motion::{self, MotionFrame};
use vexel_ui::{FontWeight, IconKind, Rect, WidgetCanvas};

#[path = "inspector_ext.rs"]
mod ext;

pub const CARD_TITLE: &str = "Select a shape";
pub const CARD_HINT: &str = "Click one on the canvas to edit it.";

/// Empty-state line of a section: only Selection has one.
pub fn section_empty_text(section: usize, content: &Inspector) -> Option<&'static str> {
    (section == 0 && matches!(content, Inspector::NoSelection)).then_some(CARD_TITLE)
}

fn pack(c: [u8; 4]) -> u32 {
    u32::from_be_bytes(c)
}

fn inset(r: Rect, x: u32, y: u32, h: u32) -> Rect {
    Rect {
        x: r.x + x,
        y: r.y + y,
        width: r.width.saturating_sub(2 * x),
        height: h,
    }
}

pub(crate) fn draw_inspector(
    c: &mut WidgetCanvas<'_>,
    l: &Layout,
    model: &Model,
    frame: &MotionFrame,
) {
    let (Some(panel), content) = (l.inspector, &model.chrome.inspector) else {
        return;
    };
    let t = c.tokens;
    c.paint_rect(panel, t.panel);
    let empty = matches!(content, Inspector::NoSelection);
    for i in 0..SECTIONS {
        let head = l.section_headers[i];
        if head.width == 0 {
            continue;
        }
        let open = l.section_bodies[i].height > 0;
        // No band at rest: transparent over the panel, a hover fill and a
        // 1 px divider above (except the first header).
        let hover = frame.get(motion::control_id(crate::model::Target::SectionHeader(i)));
        if hover.fill != t.panel {
            c.paint_rect(head, hover.fill);
        }
        if i > 0 {
            c.paint_rect(Rect { height: 1, ..head }, t.separator);
        }
        let chevron = if open {
            IconKind::CHEVRON_DOWN
        } else {
            IconKind::CHEVRON_RIGHT
        };
        let g = Rect {
            x: head.x + 10,
            y: head.y + (head.height - 16) / 2,
            width: 16,
            height: 16,
        };
        c.paint_icon_color(chevron, g, t.text_secondary);
        let label = Rect {
            x: head.x + 32,
            y: head.y,
            width: head.width - 40,
            height: head.height,
        };
        text(
            c,
            SECTION_NAMES[i],
            label,
            t.text_primary,
            13,
            FontWeight::Semibold,
        );
        if model.ui.focus == Some(Focus::Section(i)) {
            paint_focus(
                c,
                Rect {
                    x: head.x + 2,
                    y: head.y + 2,
                    width: head.width - 4,
                    height: head.height - 4,
                },
                4,
            );
        }
        let body = l.section_bodies[i];
        let full = if i == 0 && empty {
            EMPTY_CARD_BODY_H
        } else {
            SECTION_BODY_H[i]
        };
        if body.height == full {
            draw_body(c, i, body, l, content, model);
        }
    }
}

fn draw_body(
    c: &mut WidgetCanvas<'_>,
    section: usize,
    body: Rect,
    l: &Layout,
    content: &Inspector,
    model: &Model,
) {
    let t = c.tokens;
    let ext = model
        .chrome
        .state
        .ext
        .as_ref()
        .filter(|_| shape_selected(content));
    let shape = match content {
        Inspector::Shape(p) => Some(p),
        Inspector::NoSelection => None,
    };
    match section {
        0 => match shape {
            None => empty_card(c, body),
            Some(p) => {
                text(
                    c,
                    &format!("Shape {}", p.index),
                    inset(body, 12, 8, 20),
                    t.text_primary,
                    13,
                    FontWeight::Regular,
                );
                ext::draw_selection(c, l, ext);
            }
        },
        1 => {
            let label = match ext.map(|e| e.fill) {
                Some(FillKind::Linear) => Some("linear gradient".to_owned()),
                Some(FillKind::Radial) => Some("radial gradient".to_owned()),
                _ => shape.map(ShapeProps::fill_text),
            };
            colour_field(c, inset(body, 12, 8, 28), label, shape.and_then(|p| p.fill));
            draw_swatches(c, body, false, shape.and_then(|p| p.fill));
            ext::draw_fill(c, l, ext);
        }
        2 => {
            colour_field(
                c,
                inset(body, 12, 8, 28),
                shape.map(ShapeProps::stroke_text),
                shape.and_then(|p| p.stroke),
            );
            let w = shape.map(ShapeProps::width_text);
            number_field(c, inset(body, 12, 44, 28), "Width", w.as_deref(), "px");
            draw_swatches(c, body, true, shape.and_then(|p| p.stroke));
            draw_width_presets(c, body, shape.map(|p| p.stroke_width));
            ext::draw_stroke(c, l, ext);
        }
        3 => {
            let (x, y) = shape.map_or((None, None), |p| {
                (Some(number(p.transform.e)), Some(number(p.transform.f)))
            });
            let row = inset(body, 12, 8, 28);
            let half = (row.width - 8) / 2;
            number_field(c, Rect { width: half, ..row }, "X", x.as_deref(), "");
            number_field(
                c,
                Rect {
                    x: row.x + half + 8,
                    width: half,
                    ..row
                },
                "Y",
                y.as_deref(),
                "",
            );
            ext::draw_transform(c, l, ext);
        }
        _ => {
            let loop_s = model.chrome.state.loop_seconds;
            if model.chrome.state.open && loop_s > 0.0 {
                number_field(
                    c,
                    inset(body, 12, 8, 28),
                    "Loop",
                    Some(&strip::format_time(loop_s)),
                    "",
                );
            } else {
                text(
                    c,
                    "No animation",
                    inset(body, 12, 8, 20),
                    t.text_tertiary,
                    12,
                    FontWeight::Regular,
                );
            }
        }
    }
}

fn shape_selected(content: &Inspector) -> bool {
    matches!(content, Inspector::Shape(_))
}

fn draw_swatches(c: &mut WidgetCanvas<'_>, body: Rect, stroke: bool, selected: Option<[u8; 4]>) {
    let t = c.tokens;
    let y = body.y + if stroke { 78 } else { 42 };
    for (index, hex) in crate::model::INSPECTOR_SWATCHES.iter().enumerate() {
        let x = body.x + 12 + index as u32 * 28;
        let r = Rect {
            x,
            y,
            width: 20,
            height: 20,
        };
        let rgba = [
            u8::from_str_radix(&hex[1..3], 16).unwrap_or(0),
            u8::from_str_radix(&hex[3..5], 16).unwrap_or(0),
            u8::from_str_radix(&hex[5..7], 16).unwrap_or(0),
            255,
        ];
        let rgb = u32::from_be_bytes(rgba);
        c.paint_round(r, 5, rgb, t.control_edge, 1);
        if selected.is_some_and(|s| s == rgba) {
            c.paint_round(
                Rect {
                    x: x.saturating_sub(2),
                    y: y.saturating_sub(2),
                    width: 24,
                    height: 24,
                },
                6,
                0,
                t.accent,
                1,
            );
            c.paint_round(r, 5, rgb, t.control_edge, 1);
        }
    }
}

fn draw_width_presets(c: &mut WidgetCanvas<'_>, body: Rect, selected: Option<f64>) {
    let labels = ["1", "2", "4", "8"];
    for (index, label) in labels.iter().enumerate() {
        let r = Rect {
            x: body.x + 64 + index as u32 * 38,
            y: body.y + 106,
            width: 32,
            height: 22,
        };
        let t = c.tokens;
        c.paint_round(r, 5, t.surface[3], t.control_edge, 1);
        if selected == Some([1.0, 2.0, 4.0, 8.0][index]) {
            c.paint_round(
                Rect {
                    x: r.x.saturating_sub(1),
                    y: r.y.saturating_sub(1),
                    width: r.width + 2,
                    height: r.height + 2,
                },
                6,
                0,
                t.accent,
                1,
            );
            c.paint_round(r, 5, t.surface[3], t.control_edge, 1);
        }
        centred(c, label, r, t.text_primary, 12, FontWeight::Semibold);
    }
}

/// The one empty-state card: icon, title and one hint line.
fn empty_card(c: &mut WidgetCanvas<'_>, body: Rect) {
    let t = c.tokens;
    let card = inset(body, 12, 8, body.height.saturating_sub(16));
    c.paint_round(card, 8, t.surface[3], 0, 0);
    let icon = Rect {
        x: card.x + (card.width.saturating_sub(16)) / 2,
        y: card.y + 18,
        width: 16,
        height: 16,
    };
    c.paint_icon_color(IconKind::SELECT, icon, t.text_secondary);
    let row = |y: u32| Rect {
        x: card.x + 8,
        y: card.y + y,
        width: card.width.saturating_sub(16),
        height: 20,
    };
    centred(
        c,
        section_empty_text(0, &Inspector::NoSelection).unwrap_or(""),
        row(44),
        t.text_primary,
        13,
        FontWeight::Semibold,
    );
    centred(
        c,
        CARD_HINT,
        row(66),
        t.text_tertiary,
        12,
        FontWeight::Regular,
    );
}

/// Field container: level 3 fill, 6 px radius, no edge.
fn field_box(c: &mut WidgetCanvas<'_>, r: Rect) {
    let fill = c.tokens.surface[3];
    c.paint_round(r, 6, fill, 0, 0);
}

fn colour_field(c: &mut WidgetCanvas<'_>, r: Rect, label: Option<String>, colour: Option<[u8; 4]>) {
    let t = c.tokens;
    field_box(c, r);
    let swatch = Rect {
        x: r.x + 8,
        y: r.y + (r.height - 16) / 2,
        width: 16,
        height: 16,
    };
    match colour {
        Some(col) => c.paint_round(swatch, 4, pack(col), t.control_edge, 1),
        None => c.paint_round(swatch, 4, 0, t.control_edge, 1),
    }
    let (s, color) = match &label {
        Some(s) => (s.as_str(), t.text_primary),
        None => ("\u{2014}", t.text_secondary),
    };
    mono(
        c,
        s,
        Rect {
            x: r.x + 32,
            y: r.y,
            width: r.width.saturating_sub(40),
            height: r.height,
        },
        color,
        12,
    );
}

fn number_field(c: &mut WidgetCanvas<'_>, r: Rect, caption: &str, value: Option<&str>, unit: &str) {
    let t = c.tokens;
    field_box(c, r);
    text(
        c,
        caption,
        Rect {
            x: r.x + 8,
            y: r.y,
            width: 52,
            height: r.height,
        },
        t.text_secondary,
        12,
        FontWeight::Regular,
    );
    let vx = r.x + 8 + text_width(caption, 12).max(14) + 12;
    let vrect = Rect {
        x: vx,
        y: r.y,
        width: (r.x + r.width).saturating_sub(vx + 6),
        height: r.height,
    };
    match value {
        Some(v) => {
            mono(c, v, vrect, t.text_primary, 12);
            if !unit.is_empty() {
                let ux = vx + v.chars().count() as u32 * 8 + 4;
                let width = vrect.width.saturating_sub(ux - vx);
                // Bounded to the field: a rect past the window edge is rejected.
                mono(
                    c,
                    unit,
                    Rect {
                        x: ux,
                        width,
                        ..vrect
                    },
                    t.text_secondary,
                    12,
                );
            }
        }
        None => mono(c, "\u{2014}", vrect, t.text_secondary, 12),
    }
}

#[cfg(test)]
mod tests {
    use crate::chrome::{Inspector, ShapeProps};
    use crate::model::{Layout, Mode, Model};
    use crate::render::render;
    use vexel_geometry::affine_identity;

    fn frame(inspector: Inspector) -> (Vec<u8>, u32) {
        let mut model = Model::new(true);
        model.chrome.inspector = inspector;
        let frame = render(1280, 820, 1, &model).expect("shell renders");
        (frame.pixels().to_vec(), frame.width())
    }

    fn region_differs(a: &[u8], b: &[u8], width: u32, r: vexel_ui::Rect) -> bool {
        (r.y..r.y + r.height).any(|y| {
            let row = (y * width + r.x) as usize * 4;
            let end = row + r.width as usize * 4;
            a[row..end] != b[row..end]
        })
    }

    #[test]
    fn both_inspector_states_render_and_selection_changes_the_panel_only() {
        let shape = Inspector::Shape(ShapeProps {
            index: 2,
            fill: Some([0xfd, 0xd8, 0x35, 255]),
            stroke: Some([0, 0, 0, 255]),
            stroke_width: 1.0,
            transform: affine_identity(),
        });
        let (none, width) = frame(Inspector::NoSelection);
        let (selected, _) = frame(shape);
        let layout = Layout::new(1280, 820, Mode::Draw).unwrap();
        let panel = layout.inspector.expect("inspector at 1280x820");
        assert!(region_differs(&none, &selected, width, panel));
        assert!(!region_differs(&none, &selected, width, layout.rail));
        assert!(!region_differs(&none, &selected, width, layout.top));
    }

    #[test]
    fn pen_and_line_rail_glyphs_paint_symbol_pixels() {
        let (pixels, width) = frame(Inspector::NoSelection);
        let layout = Layout::new(1280, 820, Mode::Draw).unwrap();
        for slot in [1, 4] {
            let r = layout.tools[slot];
            let glyph = vexel_ui::Rect {
                x: r.x + 2,
                y: r.y + 6,
                width: r.width - 4,
                height: 16,
            };
            let at = |x: u32, y: u32| &pixels[(y * width + x) as usize * 4..][..4];
            let background = at(glyph.x + 1, glyph.y + 1);
            let painted = (glyph.y..glyph.y + glyph.height)
                .flat_map(|y| (glyph.x..glyph.x + glyph.width).map(move |x| (x, y)))
                .filter(|&(x, y)| at(x, y) != background)
                .count();
            assert!(painted >= 8, "slot {slot}: {painted} glyph pixels");
        }
    }
}
