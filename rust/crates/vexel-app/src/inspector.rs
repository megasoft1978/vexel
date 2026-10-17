//! Inspector: collapsible grouped sections (Selection, Fill, Stroke,
//! Transform, Animation), read-only for now. Geometry comes from the layout
//! module; this file paints rows into the section rects.

use crate::animation::strip;
use crate::chrome::{Inspector, ShapeProps, number};
use crate::layout::paint::{mono, paint_focus, text, text_width};
use crate::layout::{Focus, Layout, SECTION_BODY_H, SECTION_NAMES, SECTIONS};
use crate::model::Model;
use vexel_ui::{FontWeight, IconKind, Rect, WidgetCanvas};

pub const NOTHING_SELECTED: &str = "Nothing selected";
pub const PICK_HINT: &str = "Pick a shape with the Select tool.";

/// Empty-state line of a section: only Selection has one.
pub fn section_empty_text(section: usize, content: &Inspector) -> Option<&'static str> {
    (section == 0 && matches!(content, Inspector::NoSelection)).then_some(NOTHING_SELECTED)
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

pub(crate) fn draw_inspector(c: &mut WidgetCanvas<'_>, l: &Layout, model: &Model) {
    let (Some(panel), content) = (l.inspector, &model.chrome.inspector) else {
        return;
    };
    let t = c.tokens;
    c.paint_rect(panel, t.panel);
    for i in 0..SECTIONS {
        let head = l.section_headers[i];
        if head.width == 0 {
            continue;
        }
        let open = l.section_bodies[i].height > 0;
        c.paint_rect(head, t.panel_raised);
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
        if body.height == SECTION_BODY_H[i] {
            draw_body(c, i, body, content, model);
        }
    }
}

fn draw_body(
    c: &mut WidgetCanvas<'_>,
    section: usize,
    body: Rect,
    content: &Inspector,
    model: &Model,
) {
    let t = c.tokens;
    let shape = match content {
        Inspector::Shape(p) => Some(p),
        Inspector::NoSelection => None,
    };
    match section {
        0 => match shape {
            None => {
                text(
                    c,
                    section_empty_text(0, content).unwrap_or(""),
                    inset(body, 12, 8, 20),
                    t.text_secondary,
                    13,
                    FontWeight::Regular,
                );
                text(
                    c,
                    PICK_HINT,
                    inset(body, 12, 28, 20),
                    t.text_tertiary,
                    12,
                    FontWeight::Regular,
                );
            }
            Some(p) => {
                text(
                    c,
                    &format!("Shape {}", p.index),
                    inset(body, 12, 8, 20),
                    t.text_primary,
                    13,
                    FontWeight::Regular,
                );
                text(
                    c,
                    "Read-only for now",
                    inset(body, 12, 28, 20),
                    t.text_tertiary,
                    12,
                    FontWeight::Regular,
                );
            }
        },
        1 => colour_field(
            c,
            inset(body, 12, 8, 28),
            shape.map(ShapeProps::fill_text),
            shape.and_then(|p| p.fill),
        ),
        2 => {
            colour_field(
                c,
                inset(body, 12, 8, 28),
                shape.map(ShapeProps::stroke_text),
                shape.and_then(|p| p.stroke),
            );
            let w = shape.map(ShapeProps::width_text);
            number_field(c, inset(body, 12, 44, 28), "Width", w.as_deref(), "px");
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
            let m = shape.map(|p| {
                let t = p.transform;
                format!(
                    "{} {} {} {}",
                    number(t.a),
                    number(t.b),
                    number(t.c),
                    number(t.d)
                )
            });
            number_field(c, inset(body, 12, 44, 28), "Matrix", m.as_deref(), "");
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
                mono(c, unit, Rect { x: ux, ..vrect }, t.text_secondary, 12);
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
