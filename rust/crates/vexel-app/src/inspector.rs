//! Read-only inspector panel: the picked shape's fill, stroke, stroke width
//! and transform, or an explicit "No selection" with placeholder fields.

use crate::chrome::{Inspector, NO_SELECTION};
use crate::render::label;
use vexel_ui::{FontWeight, Rect, WidgetCanvas};

const PLACEHOLDER: &str = "\u{2014}";

pub(crate) fn draw_inspector(c: &mut WidgetCanvas<'_>, r: Rect, content: &Inspector) {
    let tokens = c.tokens;
    c.paint_rect(r, tokens.panel);
    c.paint_rect(
        Rect {
            x: r.x,
            y: r.y,
            width: 1,
            height: r.height,
        },
        tokens.separator,
    );
    // The full inspector has fixed rows through y=374. On compact layouts the
    // layout omits it; keep this guard so future callers cannot paint them past
    // a shorter panel or turn one bad label into a failed whole-frame render.
    if r.height < 414 {
        return;
    }
    let (heading, hint, fill, stroke, position, matrix) = match content {
        Inspector::NoSelection => (
            NO_SELECTION.to_owned(),
            "Pick a shape with the Select tool to see its properties.".to_owned(),
            None,
            None,
            None,
            None,
        ),
        Inspector::Shape(p) => (
            format!("Shape {}", p.index),
            "Read-only; editing is not available yet.".to_owned(),
            Some(p.fill_text()),
            Some(format!("{}, width {}", p.stroke_text(), p.width_text())),
            Some(p.position_text()),
            Some(p.matrix_text()),
        ),
    };
    let width = r.width.saturating_sub(32);
    label(
        c,
        "Inspector",
        Rect {
            x: r.x + 16,
            y: r.y + 14,
            width: r.width - 28,
            height: 24,
        },
        tokens.text_primary,
        14,
        FontWeight::Semibold,
    );
    c.paint_rect(
        Rect {
            x: r.x + 14,
            y: r.y + 48,
            width: r.width - 28,
            height: 1,
        },
        tokens.separator,
    );
    label(
        c,
        &heading,
        Rect {
            x: r.x + 16,
            y: r.y + 62,
            width: r.width - 28,
            height: 22,
        },
        tokens.text_secondary,
        12,
        FontWeight::Regular,
    );
    label(
        c,
        &hint,
        Rect {
            x: r.x + 16,
            y: r.y + 94,
            width,
            height: 48,
        },
        tokens.text_tertiary,
        11,
        FontWeight::Regular,
    );
    // (caption row y, box y, box height, value)
    let fields = [
        ("Fill", 166, 190, 32, fill, PLACEHOLDER),
        ("Stroke", 242, 266, 30, stroke, PLACEHOLDER),
    ];
    for (caption, caption_y, box_y, height, value, placeholder) in fields {
        field(
            c,
            r,
            (caption, caption_y),
            (box_y, height),
            value,
            placeholder,
        );
    }
    label(
        c,
        "Transform",
        Rect {
            x: r.x + 16,
            y: r.y + 318,
            width,
            height: 20,
        },
        tokens.text_secondary,
        11,
        FontWeight::Semibold,
    );
    for (i, (value, placeholder)) in [(position, "Position"), (matrix, "Matrix")]
        .into_iter()
        .enumerate()
    {
        field(c, r, ("", 0), (344 + i as u32 * 40, 30), value, placeholder);
    }
}

/// One captioned input box. Without a value the placeholder is drawn dimmed.
fn field(
    c: &mut WidgetCanvas<'_>,
    r: Rect,
    (caption, caption_y): (&str, u32),
    (box_y, height): (u32, u32),
    value: Option<String>,
    placeholder: &str,
) {
    let tokens = c.tokens;
    if !caption.is_empty() {
        label(
            c,
            caption,
            Rect {
                x: r.x + 16,
                y: r.y + caption_y,
                width: r.width.saturating_sub(32),
                height: 20,
            },
            tokens.text_secondary,
            11,
            FontWeight::Semibold,
        );
    }
    c.paint_round(
        Rect {
            x: r.x + 16,
            y: r.y + box_y,
            width: r.width.saturating_sub(32),
            height,
        },
        4,
        tokens.panel_raised,
        tokens.separator,
        1,
    );
    let (text, color) = match &value {
        Some(text) => (text.as_str(), tokens.text_primary),
        None => (placeholder, tokens.disabled_text),
    };
    label(
        c,
        text,
        Rect {
            x: r.x + 24,
            y: r.y + box_y + height.saturating_sub(20) / 2,
            width: r.width.saturating_sub(48),
            height: 20,
        },
        color,
        11,
        FontWeight::Regular,
    );
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
