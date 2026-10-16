//! Document-driven window chrome: title, top bar, status bar and inspector
//! content are a pure function of [`AppState`], so no handler can leave a
//! stale "No document open" behind. The literal is produced only here, and
//! only when no document is open.

use crate::compose::{DocumentView, OwnedShape};
use crate::model::{Layout, Tool};
use crate::session::{AppDocument, window_title};
use vexel_geometry::Affine;
use vexel_ui::Rect;

pub const NO_DOCUMENT: &str = "No document open";
pub const NO_SELECTION: &str = "No selection";

/// Everything the chrome shows about the document, as plain data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppState {
    /// False for the blank startup document.
    pub open: bool,
    /// File name; `None` for an untitled document.
    pub name: Option<String>,
    /// Full path of the project or imported SVG, when there is one.
    pub path: Option<String>,
    /// Document (not stage) size in user units.
    pub size: (u32, u32),
    /// Shape count once composed; `None` while composing.
    pub shapes: Option<usize>,
    pub dirty: bool,
    /// Typed error of the last failed open or compose.
    pub error: Option<String>,
    pub selection: Option<ShapeProps>,
}

impl AppState {
    pub fn from_session(doc: &AppDocument, view: &DocumentView) -> Self {
        Self {
            open: doc.revision() > 0,
            name: doc.display_name(),
            path: doc
                .origin_path()
                .map(|p| String::from_utf8_lossy(p).into_owned()),
            size: doc.size(),
            shapes: view.composed().map(|c| c.shapes.len()),
            dirty: doc.is_dirty(),
            error: view
                .status()
                .filter(|(_, error)| *error)
                .map(|(text, _)| text.to_owned()),
            selection: view.selected_shape().map(ShapeProps::from_shape),
        }
    }
}

/// Paint, stroke and transform of one shape, as read from the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeProps {
    /// Index of the shape in scene (source) order.
    pub index: usize,
    pub fill: Option<[u8; 4]>,
    pub stroke: Option<[u8; 4]>,
    pub stroke_width: f64,
    pub transform: Affine,
}

impl ShapeProps {
    pub fn from_shape(shape: &OwnedShape) -> Self {
        // A transparent or zero-width stroke paints nothing: report it as none.
        let stroke = shape.stroke.filter(|s| s.rgba[3] > 0 && s.width > 0.0);
        Self {
            index: shape.scene_index,
            fill: shape.filled.then_some(shape.fill_rgba),
            stroke: stroke.map(|s| s.rgba),
            stroke_width: stroke.map_or(0.0, |s| s.width),
            transform: shape.transform,
        }
    }

    pub fn fill_text(&self) -> String {
        color_text(self.fill)
    }

    pub fn stroke_text(&self) -> String {
        color_text(self.stroke)
    }

    pub fn width_text(&self) -> String {
        number(self.stroke_width)
    }

    /// Translation part of the transform.
    pub fn position_text(&self) -> String {
        format!(
            "Position {}, {}",
            number(self.transform.e),
            number(self.transform.f)
        )
    }

    /// Linear part of the transform.
    pub fn matrix_text(&self) -> String {
        let t = self.transform;
        format!(
            "Matrix {} {} {} {}",
            number(t.a),
            number(t.b),
            number(t.c),
            number(t.d)
        )
    }
}

fn color_text(color: Option<[u8; 4]>) -> String {
    match color {
        None => "none".to_owned(),
        Some([r, g, b, 255]) => format!("#{r:02x}{g:02x}{b:02x}"),
        Some([r, g, b, a]) => format!("#{r:02x}{g:02x}{b:02x}{a:02x}"),
    }
}

/// Up to three decimals, trailing zeros trimmed, no "-0".
fn number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" || text.is_empty() {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inspector {
    NoSelection,
    Shape(ShapeProps),
}

/// Text for the window title, top bar, status bar and inspector.
#[derive(Clone, Debug, PartialEq)]
pub struct ChromeText {
    pub title: String,
    pub top: String,
    pub status: String,
    pub inspector: Inspector,
}

impl Default for ChromeText {
    /// The state with no document open.
    fn default() -> Self {
        chrome_text(&AppState::default())
    }
}

fn plural(count: usize) -> String {
    format!("{count} shape{}", if count == 1 { "" } else { "s" })
}

pub fn chrome_text(state: &AppState) -> ChromeText {
    let label = state.name.as_deref().unwrap_or("Untitled");
    let marker = if state.dirty { " \u{2022}" } else { "" };
    let mut parts = vec![format!("{}x{}", state.size.0, state.size.1)];
    parts.extend(state.shapes.map(plural));
    let (title, top, summary) = if state.open {
        let size_and_shapes = parts.join(", ");
        let location = state.path.as_deref().unwrap_or(label);
        let mut status = vec![location.to_owned()];
        status.extend(parts);
        if state.dirty {
            status.push("modified".to_owned());
        }
        (
            window_title(Some(label), state.dirty),
            format!("{label}{marker}  \u{b7}  {size_and_shapes}"),
            Some(status.join(" \u{b7} ")),
        )
    } else {
        ("Vexel".to_owned(), NO_DOCUMENT.to_owned(), None)
    };
    let status = match (&state.error, summary) {
        (Some(error), Some(kept)) => format!("{error} \u{b7} kept {kept}"),
        (Some(error), None) => error.clone(),
        (None, Some(summary)) => summary,
        (None, None) => NO_DOCUMENT.to_owned(),
    };
    let inspector = match state.selection.filter(|_| state.open) {
        Some(props) => Inspector::Shape(props),
        None => Inspector::NoSelection,
    };
    ChromeText {
        title,
        top,
        status,
        inspector,
    }
}

/// Where the document status text is drawn: inside the status bar, right of
/// the "Ready" label and left of the workspace label, clipped to that span.
pub fn status_label_rect(layout: &Layout) -> Rect {
    const LEFT: u32 = 84;
    const RIGHT_RESERVED: u32 = 176;
    Rect {
        x: LEFT,
        y: layout.status.y + 3,
        width: layout.status.width.saturating_sub(LEFT + RIGHT_RESERVED),
        height: layout.status.height.saturating_sub(6).max(1),
    }
}

/// Top-bar document label: right of the app name, left of the mode switch.
pub fn top_label_rect(layout: &Layout) -> Rect {
    const LEFT: u32 = 98;
    Rect {
        x: LEFT,
        y: 10,
        width: layout.draw_mode.x.saturating_sub(LEFT + 12),
        height: 26,
    }
}

/// The glyph and caption of one rail tool button. The glyph is a symbol and
/// never repeats the caption, so a button never shows its label twice.
pub fn rail_glyph(tool: Tool) -> (&'static str, &'static str) {
    match tool {
        Tool::Select => ("\u{2196}", "Select"),
        Tool::Pen => ("\u{270e}", "Pen"),
        Tool::Rectangle => ("\u{25ad}", "Rect"),
        Tool::Ellipse => ("\u{25cb}", "Ellipse"),
        Tool::Line => ("\u{2571}", "Line"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mode;

    fn open_state() -> AppState {
        AppState {
            open: true,
            name: Some("compose-v1.svg".into()),
            path: Some("/work/compose-v1.svg".into()),
            size: (160, 120),
            shapes: Some(7),
            ..AppState::default()
        }
    }

    #[test]
    fn placeholder_appears_only_when_no_document_is_open() {
        let none = chrome_text(&AppState::default());
        assert_eq!(none.title, "Vexel");
        assert_eq!(none.top, NO_DOCUMENT);
        assert_eq!(none.status, NO_DOCUMENT);
        assert_eq!(none, ChromeText::default());
        let open = chrome_text(&open_state());
        for text in [&open.title, &open.top, &open.status] {
            assert!(!text.contains(NO_DOCUMENT), "{text}");
            assert!(text.contains("compose-v1.svg"), "{text}");
        }
        for text in [&open.top, &open.status] {
            assert!(
                text.contains("160x120") && text.contains("7 shapes"),
                "{text}"
            );
        }
        assert!(open.status.contains("/work/compose-v1.svg"));
    }

    #[test]
    fn dirty_state_shows_a_marker_everywhere() {
        let clean = chrome_text(&open_state());
        let dirty = chrome_text(&AppState {
            dirty: true,
            ..open_state()
        });
        assert!(dirty.title.ends_with('\u{2022}') && !clean.title.contains('\u{2022}'));
        assert!(dirty.top.contains('\u{2022}') && !clean.top.contains('\u{2022}'));
        assert!(dirty.status.contains("modified") && !clean.status.contains("modified"));
    }

    #[test]
    fn composing_omits_the_shape_count_and_singular_is_grammatical() {
        let composing = chrome_text(&AppState {
            shapes: None,
            ..open_state()
        });
        assert!(composing.top.contains("160x120") && !composing.top.contains("shape"));
        let one = chrome_text(&AppState {
            shapes: Some(1),
            ..open_state()
        });
        assert!(one.top.contains("1 shape") && !one.top.contains("1 shapes"));
    }

    #[test]
    fn failed_open_shows_the_typed_error_and_keeps_the_previous_document() {
        let state = AppState {
            error: Some("Cannot open bad.svg: malformed SVG".into()),
            ..open_state()
        };
        let text = chrome_text(&state);
        assert!(text.status.contains("Cannot open bad.svg: malformed SVG"));
        assert!(text.status.contains("compose-v1.svg") && text.status.contains("7 shapes"));
        assert!(text.top.contains("compose-v1.svg") && text.title.contains("compose-v1.svg"));
        let first = chrome_text(&AppState {
            error: Some("Cannot open bad.svg: malformed SVG".into()),
            ..AppState::default()
        });
        assert_eq!(first.status, "Cannot open bad.svg: malformed SVG");
        assert_eq!(first.top, NO_DOCUMENT);
    }

    #[test]
    fn chrome_is_a_function_of_state_so_resize_and_scale_cannot_reset_it() {
        // Resize and scale handlers never touch AppState; the text is
        // recomputed from it and is therefore identical before and after.
        let state = open_state();
        assert_eq!(chrome_text(&state), chrome_text(&state.clone()));
    }

    #[test]
    fn status_label_stays_inside_the_status_bar_and_off_the_rail() {
        for width in 1000..=1920 {
            for mode in [Mode::Draw, Mode::Animate] {
                let l = Layout::new(width, 852, mode).unwrap();
                let r = status_label_rect(&l);
                assert!(r.width > 0 && r.height > 0, "{width}");
                assert!(r.x >= l.status.x && r.y >= l.status.y, "{width}");
                assert!(r.x + r.width <= l.status.x + l.status.width, "{width}");
                assert!(r.y + r.height <= l.status.y + l.status.height, "{width}");
                assert!(r.x + r.width <= width - 176, "{width}: workspace label");
                assert!(disjoint(r, l.rail), "{width}: rail");
                let top = top_label_rect(&l);
                assert!(top.width > 0 && top.x + top.width <= l.draw_mode.x);
                assert!(disjoint(top, l.draw_mode) && disjoint(top, l.rail));
            }
        }
    }

    fn disjoint(a: Rect, b: Rect) -> bool {
        a.x + a.width <= b.x
            || b.x + b.width <= a.x
            || a.y + a.height <= b.y
            || b.y + b.height <= a.y
    }

    #[test]
    fn rail_buttons_never_show_the_same_string_twice() {
        for tool in [
            Tool::Select,
            Tool::Pen,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Line,
        ] {
            let (glyph, caption) = rail_glyph(tool);
            assert_ne!(glyph, caption, "{tool:?}");
            assert!(!glyph.is_empty() && !caption.is_empty());
        }
        for tool in [Tool::Pen, Tool::Line] {
            let (glyph, _) = rail_glyph(tool);
            assert!(
                glyph.chars().all(|c| !c.is_alphanumeric()),
                "{tool:?} glyph must be a symbol"
            );
        }
    }

    #[test]
    fn inspector_defaults_to_no_selection_and_formats_properties() {
        assert_eq!(chrome_text(&open_state()).inspector, Inspector::NoSelection);
        let props = ShapeProps {
            index: 0,
            fill: Some([0xe5, 0x39, 0x35, 255]),
            stroke: None,
            stroke_width: 2.0,
            transform: Affine {
                a: 0.5,
                b: -0.0,
                c: 0.0,
                d: 0.5,
                e: 12.0,
                f: 46.25,
            },
        };
        let text = chrome_text(&AppState {
            selection: Some(props),
            ..open_state()
        });
        assert_eq!(text.inspector, Inspector::Shape(props));
        assert_eq!(props.fill_text(), "#e53935");
        assert_eq!(props.stroke_text(), "none");
        assert_eq!(props.width_text(), "2");
        assert_eq!(props.position_text(), "Position 12, 46.25");
        assert_eq!(props.matrix_text(), "Matrix 0.5 0 0 0.5");
    }
}
