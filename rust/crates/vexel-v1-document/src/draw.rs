//! Source-preserving line, path, recolour, delete, move and hit-test edits.
//!
//! Every mutation is one editor history entry. Attribute edits splice only the
//! targeted start-tag bytes; nothing else is re-serialized.

use crate::{V1Document, V1DocumentError, numeric_text, path_data, validate_stable_id};
use vexel_document_edit::{
    DocumentHandle, HistoryAction, SourceSpan, TemplateAttribute, TemplateKind,
    document_view_first_child, document_view_next_sibling, encode_attribute,
};

/// Distance, in document units, added around line and path geometry by
/// [`V1Document::hit_test`] (path hits use the bounding box plus this margin).
pub const HIT_TOLERANCE: f64 = 2.0;
const MAX_STROKE_WIDTH: f64 = 256.0;
const MAX_COORDINATE: f64 = 1_000_000.0;

/// Shape category of a source element, as understood by the editing tools.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Rect,
    Ellipse,
    Circle,
    Line,
    Path,
    /// Present in the source but not movable or recolourable by the tools.
    Other,
}

impl ElementKind {
    fn from_tag(tag: &[u8]) -> Self {
        match tag {
            b"rect" => Self::Rect,
            b"ellipse" => Self::Ellipse,
            b"circle" => Self::Circle,
            b"line" => Self::Line,
            b"path" => Self::Path,
            _ => Self::Other,
        }
    }
}

fn check_color(value: &str) -> Result<(), V1DocumentError> {
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidColor)
    }
}

fn check_paint(value: &str) -> Result<(), V1DocumentError> {
    if value == "none" {
        Ok(())
    } else {
        check_color(value)
    }
}

fn check_width(width: f64) -> Result<(), V1DocumentError> {
    if width.is_finite() && (0.0..=MAX_STROKE_WIDTH).contains(&width) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidStrokeWidth)
    }
}

fn check_coordinate(value: f64) -> Result<(), V1DocumentError> {
    if value.is_finite() && value.abs() <= MAX_COORDINATE {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidGeometry)
    }
}

fn parse_number(raw: &[u8]) -> Option<f64> {
    if raw.is_empty()
        || !raw
            .iter()
            .all(|b| b.is_ascii_digit() || b"+-.eE".contains(b))
    {
        return None;
    }
    std::str::from_utf8(raw)
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Accepts exactly one `translate(x)` or `translate(x y)` (comma or space separated).
fn parse_translate(raw: &[u8]) -> Option<(f64, f64)> {
    let text = std::str::from_utf8(raw).ok()?.trim();
    let inner = text
        .strip_prefix("translate")?
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')?;
    if inner.contains(['(', ')']) {
        return None;
    }
    let mut parts = inner
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|t| !t.is_empty());
    let x = parse_number(parts.next()?.as_bytes())?;
    let y = match parts.next() {
        Some(part) => parse_number(part.as_bytes())?,
        None => 0.0,
    };
    parts.next().is_none().then_some((x, y))
}

fn plain(value: f64) -> String {
    format!("{}", value + 0.0)
}

fn distance_to_segment(px: f64, py: f64, (ax, ay, bx, by): (f64, f64, f64, f64)) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / length_squared).clamp(0.0, 1.0)
    };
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

impl V1Document {
    /// Returns the editable geometry bounds of a simple shape or path in document space.
    pub fn geometry_bounds(&self, id: &str) -> Result<(f64, f64, f64, f64), V1DocumentError> {
        let node = self.shape(id)?;
        let number = |name: &[u8], default: f64| match self.raw_attr(node, name) {
            None => Ok(default),
            Some(raw) => parse_number(raw).ok_or(V1DocumentError::InvalidGeometry),
        };
        let mut bounds = match self.kind_of(node) {
            ElementKind::Rect => {
                let (x, y, w, h) = (
                    number(b"x", 0.0)?,
                    number(b"y", 0.0)?,
                    number(b"width", 0.0)?,
                    number(b"height", 0.0)?,
                );
                (x, y, x + w, y + h)
            }
            ElementKind::Ellipse => {
                let (cx, cy) = (number(b"cx", 0.0)?, number(b"cy", 0.0)?);
                let (rx, ry) = (number(b"rx", 0.0)?, number(b"ry", 0.0)?);
                (cx - rx, cy - ry, cx + rx, cy + ry)
            }
            ElementKind::Circle => return Err(V1DocumentError::Unsupported),
            ElementKind::Line => {
                let (x1, y1, x2, y2) = (
                    number(b"x1", 0.0)?,
                    number(b"y1", 0.0)?,
                    number(b"x2", 0.0)?,
                    number(b"y2", 0.0)?,
                );
                (x1.min(x2), y1.min(y2), x1.max(x2), y1.max(y2))
            }
            ElementKind::Path => {
                let segments = path_data::parse(
                    self.raw_attr(node, b"d")
                        .ok_or(V1DocumentError::InvalidPathData)?,
                )?;
                path_data::bounds(&segments).ok_or(V1DocumentError::InvalidPathData)?
            }
            ElementKind::Other => return Err(V1DocumentError::Unsupported),
        };
        if let Some(raw) = self.raw_attr(node, b"transform") {
            let (tx, ty) = parse_translate(raw).ok_or(V1DocumentError::UnsupportedTransform)?;
            bounds.0 += tx;
            bounds.1 += ty;
            bounds.2 += tx;
            bounds.3 += ty;
        }
        Ok(bounds)
    }

    /// Resizes a shape to the requested document-space bounds in one undoable edit.
    pub fn resize_to_bounds(
        &mut self,
        id: &str,
        bounds: (f64, f64, f64, f64),
    ) -> Result<(), V1DocumentError> {
        if ![bounds.0, bounds.1, bounds.2, bounds.3]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let node = self.shape(id)?;
        let kind = self.kind_of(node);
        if bounds.2 - bounds.0 < 0.1 || bounds.3 - bounds.1 < 0.1 {
            if kind != ElementKind::Line {
                return Err(V1DocumentError::InvalidGeometry);
            }
        }
        for value in [bounds.0, bounds.1, bounds.2, bounds.3] {
            check_coordinate(value)?;
        }
        let old = self.geometry_bounds(id)?;
        let (tx, ty) = match self.raw_attr(node, b"transform") {
            Some(raw) => parse_translate(raw).ok_or(V1DocumentError::UnsupportedTransform)?,
            None => (0.0, 0.0),
        };
        let local = (old.0 - tx, old.1 - ty, old.2 - tx, old.3 - ty);
        if kind != ElementKind::Line && (local.2 <= local.0 || local.3 <= local.1) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let target = (bounds.0 - tx, bounds.1 - ty, bounds.2 - tx, bounds.3 - ty);
        let sx = if local.2 == local.0 {
            1.0
        } else {
            (target.2 - target.0) / (local.2 - local.0)
        };
        let sy = if local.3 == local.1 {
            1.0
        } else {
            (target.3 - target.1) / (local.3 - local.1)
        };
        let attr = |name: &[u8], value: f64| {
            Ok::<_, V1DocumentError>((name.to_vec(), numeric_text(value)?))
        };
        let edits = match kind {
            ElementKind::Rect => vec![
                attr(b"x", target.0)?,
                attr(b"y", target.1)?,
                attr(b"width", target.2 - target.0)?,
                attr(b"height", target.3 - target.1)?,
            ],
            ElementKind::Ellipse => vec![
                attr(b"cx", (target.0 + target.2) / 2.0)?,
                attr(b"cy", (target.1 + target.3) / 2.0)?,
                attr(b"rx", (target.2 - target.0) / 2.0)?,
                attr(b"ry", (target.3 - target.1) / 2.0)?,
            ],
            ElementKind::Circle => return Err(V1DocumentError::Unsupported),
            ElementKind::Line => {
                let n = |name| {
                    self.raw_attr(node, name)
                        .and_then(parse_number)
                        .ok_or(V1DocumentError::InvalidGeometry)
                };
                let (x1, y1, x2, y2) = (n(b"x1")?, n(b"y1")?, n(b"x2")?, n(b"y2")?);
                vec![
                    attr(b"x1", target.0 + (x1 - local.0) * sx)?,
                    attr(b"y1", target.1 + (y1 - local.1) * sy)?,
                    attr(b"x2", target.0 + (x2 - local.0) * sx)?,
                    attr(b"y2", target.1 + (y2 - local.1) * sy)?,
                ]
            }
            ElementKind::Path => {
                let raw = self
                    .raw_attr(node, b"d")
                    .ok_or(V1DocumentError::InvalidPathData)?;
                let scaled = path_data::scale(raw, local, target)?;
                vec![(b"d".to_vec(), scaled)]
            }
            ElementKind::Other => return Err(V1DocumentError::Unsupported),
        };
        let plans: Vec<(&[u8], Vec<u8>)> = edits
            .iter()
            .map(|(name, value)| (name.as_slice(), value.clone()))
            .collect();
        self.patch(node, &plans, HistoryAction::EditAttribute)
    }

    /// Inserts a line with an automatic stable id. Endpoints must lie inside the
    /// document and differ; `stroke` is `#rrggbb`; width is finite in 0..=256.
    pub fn add_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
    ) -> Result<(), V1DocumentError> {
        self.add_line_impl(x1, y1, x2, y2, stroke, width, None)
    }

    /// Inserts a line with a caller-selected stable source id.
    #[allow(clippy::too_many_arguments)]
    pub fn add_line_with_id(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
        element_id: &str,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        self.add_line_impl(x1, y1, x2, y2, stroke, width, Some(element_id))
    }

    #[allow(clippy::too_many_arguments)]
    fn add_line_impl(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
        element_id: Option<&str>,
    ) -> Result<(), V1DocumentError> {
        let (w, h) = (f64::from(self.width), f64::from(self.height));
        let inside = |x: f64, y: f64| {
            x.is_finite() && y.is_finite() && (0.0..=w).contains(&x) && (0.0..=h).contains(&y)
        };
        if !inside(x1, y1) || !inside(x2, y2) || (x1 == x2 && y1 == y2) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        check_color(stroke)?;
        check_width(width)?;
        let values = [
            numeric_text(x1)?,
            numeric_text(y1)?,
            numeric_text(x2)?,
            numeric_text(y2)?,
            numeric_text(width)?,
        ];
        let mut attrs = vec![
            TemplateAttribute {
                name: b"x1",
                value: &values[0],
            },
            TemplateAttribute {
                name: b"y1",
                value: &values[1],
            },
            TemplateAttribute {
                name: b"x2",
                value: &values[2],
            },
            TemplateAttribute {
                name: b"y2",
                value: &values[3],
            },
            TemplateAttribute {
                name: b"stroke",
                value: stroke.as_bytes(),
            },
            TemplateAttribute {
                name: b"stroke-width",
                value: &values[4],
            },
        ];
        if let Some(id) = element_id {
            attrs.push(TemplateAttribute {
                name: b"id",
                value: id.as_bytes(),
            });
        }
        self.insert_shape(TemplateKind::Line, &attrs)
    }

    /// Inserts a `path` with an automatic stable id. `d` must satisfy
    /// [`crate::validate_path_data`]; `fill` and `stroke` are `#rrggbb` or `none`.
    pub fn add_path(
        &mut self,
        d: &str,
        fill: &str,
        stroke: &str,
        width: f64,
    ) -> Result<(), V1DocumentError> {
        self.add_path_impl(d, fill, stroke, width, None)
    }

    /// Inserts a `path` with a caller-selected stable source id.
    pub fn add_path_with_id(
        &mut self,
        d: &str,
        fill: &str,
        stroke: &str,
        width: f64,
        element_id: &str,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        self.add_path_impl(d, fill, stroke, width, Some(element_id))
    }

    fn add_path_impl(
        &mut self,
        d: &str,
        fill: &str,
        stroke: &str,
        width: f64,
        element_id: Option<&str>,
    ) -> Result<(), V1DocumentError> {
        path_data::validate_path_data(d)?;
        check_paint(fill)?;
        check_paint(stroke)?;
        check_width(width)?;
        let width_text = numeric_text(width)?;
        let mut attrs = vec![
            TemplateAttribute {
                name: b"d",
                value: d.as_bytes(),
            },
            TemplateAttribute {
                name: b"fill",
                value: fill.as_bytes(),
            },
            TemplateAttribute {
                name: b"stroke",
                value: stroke.as_bytes(),
            },
            TemplateAttribute {
                name: b"stroke-width",
                value: &width_text,
            },
        ];
        if let Some(id) = element_id {
            attrs.push(TemplateAttribute {
                name: b"id",
                value: id.as_bytes(),
            });
        }
        self.insert_shape(TemplateKind::Path, &attrs)
    }

    fn insert_shape(
        &mut self,
        kind: TemplateKind,
        attrs: &[TemplateAttribute<'_>],
    ) -> Result<(), V1DocumentError> {
        let root = self.editor.root().ok_or(V1DocumentError::Edit(
            vexel_document_edit::EditError::InvalidOperation,
        ))?;
        self.editor.insert_template(root, kind, attrs)?;
        Ok(())
    }

    /// Sets `fill` on a rect, ellipse, circle, line or path.
    pub fn set_fill(&mut self, id: &str, colour: &str) -> Result<(), V1DocumentError> {
        check_color(colour)?;
        let node = self.shape(id)?;
        self.patch(
            node,
            &[(b"fill", colour.as_bytes().to_vec())],
            HistoryAction::EditAttribute,
        )
    }

    /// Sets `stroke` and `stroke-width` (finite, 0..=256) as one history entry.
    pub fn set_stroke(
        &mut self,
        id: &str,
        colour: &str,
        width: f64,
    ) -> Result<(), V1DocumentError> {
        check_color(colour)?;
        check_width(width)?;
        let node = self.shape(id)?;
        let width = numeric_text(width)?;
        self.patch(
            node,
            &[
                (b"stroke", colour.as_bytes().to_vec()),
                (b"stroke-width", width),
            ],
            HistoryAction::EditAttribute,
        )
    }

    /// Deletes the element with this stable id and its subtree.
    pub fn delete(&mut self, id: &str) -> Result<(), V1DocumentError> {
        let node = self.find_node(id)?;
        self.editor.delete_element(node)?;
        Ok(())
    }

    /// Moves a shape by a document-space delta. Rect, ellipse, circle and line
    /// coordinates are rewritten; a path gains or updates `transform="translate(..)"`
    /// and its `d` bytes stay untouched. Any other existing `transform` is refused.
    pub fn translate(&mut self, id: &str, dx: f64, dy: f64) -> Result<(), V1DocumentError> {
        check_coordinate(dx)?;
        check_coordinate(dy)?;
        let node = self.find_node(id)?;
        let kind = self.kind_of(node);
        if kind == ElementKind::Other {
            return Err(V1DocumentError::Unsupported);
        }
        let (tx, ty) = match self.raw_attr(node, b"transform") {
            None => (0.0, 0.0),
            Some(raw) => parse_translate(raw).ok_or(V1DocumentError::UnsupportedTransform)?,
        };
        let names: &[&[u8]] = match kind {
            ElementKind::Rect => &[b"x", b"y"],
            ElementKind::Ellipse | ElementKind::Circle => &[b"cx", b"cy"],
            ElementKind::Line => &[b"x1", b"y1", b"x2", b"y2"],
            _ => &[],
        };
        let mut edits: Vec<(&[u8], Vec<u8>)> = Vec::new();
        if kind == ElementKind::Path {
            let (nx, ny) = (tx + dx, ty + dy);
            check_coordinate(nx)?;
            check_coordinate(ny)?;
            edits.push((
                b"transform",
                format!("translate({} {})", plain(nx), plain(ny)).into_bytes(),
            ));
        }
        for (index, name) in names.iter().enumerate() {
            let old = match self.raw_attr(node, name) {
                None => 0.0,
                Some(raw) => parse_number(raw).ok_or(V1DocumentError::InvalidGeometry)?,
            };
            let new = old + if index % 2 == 0 { dx } else { dy };
            check_coordinate(new)?;
            edits.push((name, numeric_text(new)?));
        }
        self.patch(node, &edits, HistoryAction::Move)
    }

    /// Classifies the element with this stable id.
    pub fn element_kind(&self, id: &str) -> Result<ElementKind, V1DocumentError> {
        Ok(self.kind_of(self.find_node(id)?))
    }

    /// Returns the id of the topmost top-level shape under a document point.
    ///
    /// Only direct children of the root that carry an `id` are tested, in source
    /// order (later is higher). Rect/ellipse/circle use their geometry, lines use
    /// segment distance, paths use their bounding box; lines and paths get
    /// [`HIT_TOLERANCE`] plus half the stroke width. A `translate` transform is
    /// honoured; other transforms are ignored for hit testing (and refuse moves).
    pub fn hit_test(&self, x: f64, y: f64) -> Option<String> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let model = self.editor.try_model().ok()?;
        let mut child = document_view_first_child(model, model.root()?);
        let mut hit = None;
        while let Some(node) = child {
            if let Some(id) = self.hit_id(node, x, y) {
                hit = Some(id);
            }
            child = document_view_next_sibling(model, node);
        }
        hit
    }

    fn hit_id(&self, node: DocumentHandle, x: f64, y: f64) -> Option<String> {
        let id = self
            .raw_attr(node, b"id")
            .filter(|raw| !raw.contains(&b'&'))?;
        let kind = self.kind_of(node);
        let (tx, ty) = self
            .raw_attr(node, b"transform")
            .and_then(parse_translate)
            .unwrap_or((0.0, 0.0));
        let (px, py) = (x - tx, y - ty);
        let num = |name: &[u8], default: f64| match self.raw_attr(node, name) {
            None => Some(default),
            Some(raw) => parse_number(raw),
        };
        let stroked = self.raw_attr(node, b"stroke").is_some_and(|s| s != b"none");
        let half = if stroked {
            num(b"stroke-width", 0.0)?.max(0.0) / 2.0
        } else {
            0.0
        };
        let inside = match kind {
            ElementKind::Rect => {
                let (rx, ry, w, h) = (
                    num(b"x", 0.0)?,
                    num(b"y", 0.0)?,
                    num(b"width", 0.0)?,
                    num(b"height", 0.0)?,
                );
                w > 0.0
                    && h > 0.0
                    && px >= rx - half
                    && px <= rx + w + half
                    && py >= ry - half
                    && py <= ry + h + half
            }
            ElementKind::Ellipse | ElementKind::Circle => {
                let (cx, cy) = (num(b"cx", 0.0)?, num(b"cy", 0.0)?);
                let (rx, ry) = if kind == ElementKind::Circle {
                    let r = num(b"r", 0.0)?;
                    (r, r)
                } else {
                    (num(b"rx", 0.0)?, num(b"ry", 0.0)?)
                };
                rx > 0.0
                    && ry > 0.0
                    && ((px - cx) / (rx + half)).powi(2) + ((py - cy) / (ry + half)).powi(2) <= 1.0
            }
            ElementKind::Line => {
                let seg = (
                    num(b"x1", 0.0)?,
                    num(b"y1", 0.0)?,
                    num(b"x2", 0.0)?,
                    num(b"y2", 0.0)?,
                );
                distance_to_segment(px, py, seg) <= HIT_TOLERANCE + half
            }
            ElementKind::Path => {
                let segments = path_data::parse(self.raw_attr(node, b"d")?).ok()?;
                let (x0, y0, x1, y1) = path_data::bounds(&segments)?;
                let margin = HIT_TOLERANCE + half;
                px >= x0 - margin && px <= x1 + margin && py >= y0 - margin && py <= y1 + margin
            }
            ElementKind::Other => false,
        };
        if !inside {
            return None;
        }
        std::str::from_utf8(id).ok().map(str::to_owned)
    }

    fn find_node(&self, id: &str) -> Result<DocumentHandle, V1DocumentError> {
        validate_stable_id(id)?;
        self.editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)
    }

    fn shape(&self, id: &str) -> Result<DocumentHandle, V1DocumentError> {
        let node = self.find_node(id)?;
        if self.kind_of(node) == ElementKind::Other {
            return Err(V1DocumentError::Unsupported);
        }
        Ok(node)
    }

    fn kind_of(&self, node: DocumentHandle) -> ElementKind {
        self.editor
            .tag_name(node)
            .map_or(ElementKind::Other, ElementKind::from_tag)
    }

    fn raw_attr(&self, node: DocumentHandle, name: &[u8]) -> Option<&[u8]> {
        (0..self.editor.attribute_count(node)).find_map(|index| {
            let attribute = self.editor.attribute_at(node, index)?;
            (self.editor.span_bytes(attribute.name)? == name)
                .then(|| self.editor.span_bytes(attribute.value))
                .flatten()
        })
    }

    /// Sets or appends several attributes on one start tag as a single history
    /// entry by splicing only the bytes between the first and last change.
    fn patch(
        &mut self,
        node: DocumentHandle,
        edits: &[(&[u8], Vec<u8>)],
        action: HistoryAction,
    ) -> Result<(), V1DocumentError> {
        let mut plans: Vec<(usize, usize, Vec<u8>)> = Vec::new();
        let mut appended: Vec<u8> = Vec::new();
        let mut last_end = None;
        for index in 0..self.editor.attribute_count(node) {
            if let Some(attribute) = self.editor.attribute_at(node, index) {
                last_end = Some(attribute.value.offset + attribute.value.length + 1);
            }
        }
        for (name, value) in edits {
            let existing = (0..self.editor.attribute_count(node)).find_map(|index| {
                let attribute = self.editor.attribute_at(node, index)?;
                (self.editor.span_bytes(attribute.name)? == *name).then_some(attribute)
            });
            match existing {
                Some(attribute) => plans.push((
                    attribute.value.offset,
                    attribute.value.length,
                    encode_attribute(value, attribute.quote)?,
                )),
                None => {
                    appended.push(b' ');
                    appended.extend_from_slice(name);
                    appended.extend_from_slice(b"=\"");
                    appended.extend_from_slice(&encode_attribute(value, b'"')?);
                    appended.push(b'"');
                }
            }
        }
        if !appended.is_empty() {
            let at = match last_end {
                Some(end) => end,
                None => {
                    let span = self
                        .editor
                        .source_span(node)
                        .ok_or(V1DocumentError::MissingStableId)?;
                    span.offset + 1 + self.editor.tag_name(node).map_or(0, <[u8]>::len)
                }
            };
            plans.push((at, 0, appended));
        }
        plans.sort_by_key(|plan| plan.0);
        let (Some(first), Some(end)) = (
            plans.first().map(|p| p.0),
            plans.iter().map(|p| p.0 + p.1).max(),
        ) else {
            return Ok(());
        };
        let original = self
            .editor
            .span_bytes(SourceSpan {
                offset: first,
                length: end - first,
            })
            .ok_or(V1DocumentError::MissingStableId)?
            .to_vec();
        let mut replacement = Vec::new();
        replacement
            .try_reserve(original.len() + plans.iter().map(|p| p.2.len()).sum::<usize>())
            .map_err(|_| V1DocumentError::Allocation)?;
        let mut cursor = 0usize;
        for (offset, remove, bytes) in &plans {
            replacement.extend_from_slice(&original[cursor..offset - first]);
            replacement.extend_from_slice(bytes);
            cursor = offset - first + remove;
        }
        replacement.extend_from_slice(&original[cursor..]);
        self.editor.replace_source_span(
            SourceSpan {
                offset: first,
                length: end - first,
            },
            &replacement,
        )?;
        self.editor.history_relabel_latest(action);
        Ok(())
    }
}
