//! Gradient and clip-path definitions plus paint/clip references.
//!
//! Definitions live in a top-level `<defs>` (created as the first child of the
//! root when absent). Every public edit is one history entry and only splices
//! the bytes it must change.

use crate::{V1Document, V1DocumentError, validate_stable_id};
use vexel_document_edit::{
    DocumentHandle, EditError, HistoryAction, SourceSpan, document_view_first_child,
    document_view_next_sibling, encode_attribute,
};

const MAX_COORDINATE: f64 = 1_000_000.0;
const MIN_STOPS: usize = 2;
const MAX_STOPS: usize = 16;
const MAX_AUTO_ID_ATTEMPTS: u64 = 100_000;
const GRADIENT_PREFIX: &str = "vexel-gradient-";
const CLIP_PREFIX: &str = "vexel-clip-";

/// One gradient colour stop; `color` is `#rrggbb`, offset and opacity are 0..=1.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: String,
    pub opacity: f64,
}

/// Coordinate system of gradient geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GradientUnits {
    UserSpace,
    ObjectBoundingBox,
}

/// Behaviour outside the gradient vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Spread {
    Pad,
    Reflect,
    Repeat,
}

impl GradientUnits {
    const fn text(self) -> &'static str {
        match self {
            Self::UserSpace => "userSpaceOnUse",
            Self::ObjectBoundingBox => "objectBoundingBox",
        }
    }
}

impl Spread {
    const fn text(self) -> &'static str {
        match self {
            Self::Pad => "pad",
            Self::Reflect => "reflect",
            Self::Repeat => "repeat",
        }
    }
}

fn coordinate(value: f64) -> Result<(), V1DocumentError> {
    if value.is_finite() && value.abs() <= MAX_COORDINATE {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidGeometry)
    }
}

fn positive(value: f64) -> Result<(), V1DocumentError> {
    coordinate(value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidGeometry)
    }
}

fn unit_interval(value: f64) -> Result<(), V1DocumentError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidGeometry)
    }
}

fn color(value: &str) -> Result<(), V1DocumentError> {
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        Ok(())
    } else {
        Err(V1DocumentError::InvalidColor)
    }
}

/// Plain decimal text (never scientific) with negative zero normalized.
fn num(value: f64) -> String {
    format!("{}", value + 0.0)
}

fn validate_stops(stops: &[GradientStop]) -> Result<(), V1DocumentError> {
    if !(MIN_STOPS..=MAX_STOPS).contains(&stops.len()) {
        return Err(V1DocumentError::InvalidGeometry);
    }
    let mut previous = 0.0;
    for stop in stops {
        unit_interval(stop.offset)?;
        unit_interval(stop.opacity)?;
        color(&stop.color)?;
        if stop.offset < previous {
            return Err(V1DocumentError::InvalidGeometry);
        }
        previous = stop.offset;
    }
    Ok(())
}

fn gradient_markup(
    tag: &str,
    id: &str,
    geometry: &str,
    units: GradientUnits,
    spread: Spread,
    stops: &[GradientStop],
) -> Vec<u8> {
    let mut out = format!(
        "<{tag} id=\"{id}\"{geometry} gradientUnits=\"{}\" spreadMethod=\"{}\">",
        units.text(),
        spread.text()
    );
    for (index, stop) in stops.iter().enumerate() {
        out.push_str(&format!(
            "<stop id=\"{id}-stop-{index}\" offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
            num(stop.offset),
            stop.color,
            num(stop.opacity)
        ));
    }
    out.push_str(&format!("</{tag}>"));
    out.into_bytes()
}

/// How one attribute changes on a start tag.
type AttrEdit<'a> = (&'a [u8], Option<Vec<u8>>);

impl V1Document {
    /// Adds a `linearGradient` to the top-level `<defs>` and returns its id
    /// (`vexel-gradient-N`). Stops get ids `<gradient-id>-stop-<i>`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_linear_gradient(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        units: GradientUnits,
        spread: Spread,
        stops: &[GradientStop],
    ) -> Result<String, V1DocumentError> {
        for value in [x1, y1, x2, y2] {
            coordinate(value)?;
        }
        validate_stops(stops)?;
        let id = self.next_def_id(GRADIENT_PREFIX)?;
        let geometry = format!(
            " x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
            num(x1),
            num(y1),
            num(x2),
            num(y2)
        );
        let markup = gradient_markup("linearGradient", &id, &geometry, units, spread, stops);
        self.insert_into_defs(&markup)?;
        Ok(id)
    }

    /// Adds a `radialGradient` (centre, radius, focal point) and returns its id.
    #[allow(clippy::too_many_arguments)]
    pub fn add_radial_gradient(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        fx: f64,
        fy: f64,
        units: GradientUnits,
        spread: Spread,
        stops: &[GradientStop],
    ) -> Result<String, V1DocumentError> {
        for value in [cx, cy, fx, fy] {
            coordinate(value)?;
        }
        positive(r)?;
        validate_stops(stops)?;
        let id = self.next_def_id(GRADIENT_PREFIX)?;
        let geometry = format!(
            " cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\"",
            num(cx),
            num(cy),
            num(r),
            num(fx),
            num(fy)
        );
        let markup = gradient_markup("radialGradient", &id, &geometry, units, spread, stops);
        self.insert_into_defs(&markup)?;
        Ok(id)
    }

    /// Edits one stop (by position among the gradient's `<stop>` children).
    /// Offsets must stay within 0..=1 and ordered against the neighbouring stops.
    /// Passing no change is a no-op that records no history.
    pub fn set_gradient_stop(
        &mut self,
        gradient_id: &str,
        index: usize,
        offset: Option<f64>,
        color_value: Option<&str>,
        opacity: Option<f64>,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(gradient_id)?;
        if let Some(value) = offset {
            unit_interval(value)?;
        }
        if let Some(value) = opacity {
            unit_interval(value)?;
        }
        if let Some(value) = color_value {
            color(value)?;
        }
        let gradient = self.def_child(gradient_id, &[b"linearGradient", b"radialGradient"])?;
        let stops: Vec<DocumentHandle> = self
            .children(gradient)?
            .into_iter()
            .filter(|node| self.editor.tag_name(*node) == Some(b"stop"))
            .collect();
        let stop = *stops.get(index).ok_or(V1DocumentError::MissingStableId)?;
        if let Some(value) = offset {
            let neighbour = |at: usize| {
                stops
                    .get(at)
                    .and_then(|node| self.raw_value(*node, b"offset"))
                    .and_then(parse_offset)
            };
            let below = index.checked_sub(1).and_then(neighbour).unwrap_or(0.0);
            let above = neighbour(index + 1).unwrap_or(1.0);
            if value < below || value > above {
                return Err(V1DocumentError::InvalidGeometry);
            }
        }
        let mut edits: Vec<AttrEdit<'_>> = Vec::new();
        if let Some(value) = offset {
            edits.push((b"offset", Some(num(value).into_bytes())));
        }
        if let Some(value) = color_value {
            edits.push((b"stop-color", Some(value.as_bytes().to_vec())));
        }
        if let Some(value) = opacity {
            edits.push((b"stop-opacity", Some(num(value).into_bytes())));
        }
        self.patch_attrs(stop, &edits)
    }

    /// Sets `fill` to `url(#gradient_id)` on a shape element.
    pub fn set_fill_ref(
        &mut self,
        element_id: &str,
        gradient_id: &str,
    ) -> Result<(), V1DocumentError> {
        self.set_paint_ref(b"fill", element_id, gradient_id)
    }

    /// Sets `stroke` to `url(#gradient_id)` on a shape element.
    pub fn set_stroke_ref(
        &mut self,
        element_id: &str,
        gradient_id: &str,
    ) -> Result<(), V1DocumentError> {
        self.set_paint_ref(b"stroke", element_id, gradient_id)
    }

    /// Adds a `clipPath` holding one rectangle and returns its id (`vexel-clip-N`).
    pub fn add_clip_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<String, V1DocumentError> {
        coordinate(x)?;
        coordinate(y)?;
        positive(w)?;
        positive(h)?;
        let id = self.next_def_id(CLIP_PREFIX)?;
        let markup = format!(
            "<clipPath id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            num(x),
            num(y),
            num(w),
            num(h)
        );
        self.insert_into_defs(markup.as_bytes())?;
        Ok(id)
    }

    /// Adds a `clipPath` holding one ellipse and returns its id.
    pub fn add_clip_ellipse(
        &mut self,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
    ) -> Result<String, V1DocumentError> {
        coordinate(cx)?;
        coordinate(cy)?;
        positive(rx)?;
        positive(ry)?;
        let id = self.next_def_id(CLIP_PREFIX)?;
        let markup = format!(
            "<clipPath id=\"{id}\"><ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"/></clipPath>",
            num(cx),
            num(cy),
            num(rx),
            num(ry)
        );
        self.insert_into_defs(markup.as_bytes())?;
        Ok(id)
    }

    /// Sets `clip-path="url(#id)"` on a shape or group, or removes the
    /// attribute when `clip_id` is `None` (a no-op if it is already absent).
    pub fn set_clip(
        &mut self,
        element_id: &str,
        clip_id: Option<&str>,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        let value = match clip_id {
            Some(clip) => {
                validate_stable_id(clip)?;
                self.def_child(clip, &[b"clipPath"])?;
                Some(format!("url(#{clip})").into_bytes())
            }
            None => None,
        };
        let node = self.paintable(element_id, true)?;
        self.patch_attrs(node, &[(b"clip-path", value)])
    }

    fn set_paint_ref(
        &mut self,
        attribute: &[u8],
        element_id: &str,
        gradient_id: &str,
    ) -> Result<(), V1DocumentError> {
        validate_stable_id(element_id)?;
        validate_stable_id(gradient_id)?;
        self.def_child(gradient_id, &[b"linearGradient", b"radialGradient"])?;
        let node = self.paintable(element_id, false)?;
        let value = format!("url(#{gradient_id})").into_bytes();
        self.patch_attrs(node, &[(attribute, Some(value))])
    }

    /// Finds an element the tools accept: shapes, plus groups when `allow_group`.
    fn paintable(
        &self,
        element_id: &str,
        allow_group: bool,
    ) -> Result<DocumentHandle, V1DocumentError> {
        let node = self
            .editor
            .try_find_id(element_id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        match self.editor.tag_name(node) {
            Some(b"rect" | b"ellipse" | b"circle" | b"line" | b"path") => Ok(node),
            Some(b"g") if allow_group => Ok(node),
            _ => Err(V1DocumentError::Unsupported),
        }
    }

    fn children(&self, node: DocumentHandle) -> Result<Vec<DocumentHandle>, V1DocumentError> {
        let model = self.editor.try_model()?;
        let mut out = Vec::new();
        let mut child = document_view_first_child(model, node);
        while let Some(current) = child {
            out.push(current);
            child = document_view_next_sibling(model, current);
        }
        Ok(out)
    }

    fn root_node(&self) -> Result<DocumentHandle, V1DocumentError> {
        self.editor
            .root()
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))
    }

    fn top_level_defs(&self) -> Result<Vec<DocumentHandle>, V1DocumentError> {
        Ok(self
            .children(self.root_node()?)?
            .into_iter()
            .filter(|node| self.editor.tag_name(*node) == Some(b"defs"))
            .collect())
    }

    fn raw_value(&self, node: DocumentHandle, name: &[u8]) -> Option<&[u8]> {
        (0..self.editor.attribute_count(node)).find_map(|index| {
            let attribute = self.editor.attribute_at(node, index)?;
            (self.editor.span_bytes(attribute.name)? == name)
                .then(|| self.editor.span_bytes(attribute.value))
                .flatten()
        })
    }

    /// Finds a direct child of a top-level `<defs>` by id and tag.
    fn def_child(&self, id: &str, tags: &[&[u8]]) -> Result<DocumentHandle, V1DocumentError> {
        let mut wrong_kind = false;
        for defs in self.top_level_defs()? {
            for node in self.children(defs)? {
                if self.raw_value(node, b"id") != Some(id.as_bytes()) {
                    continue;
                }
                if self
                    .editor
                    .tag_name(node)
                    .is_some_and(|tag| tags.contains(&tag))
                {
                    return Ok(node);
                }
                wrong_kind = true;
            }
        }
        Err(if wrong_kind {
            V1DocumentError::Unsupported
        } else {
            V1DocumentError::MissingStableId
        })
    }

    /// Returns `<prefix>N` with N one past the largest numbered id already in
    /// the definitions, then skips any collision elsewhere in the document.
    fn next_def_id(&self, prefix: &str) -> Result<String, V1DocumentError> {
        let mut next = 1u64;
        for defs in self.top_level_defs()? {
            for node in self.children(defs)? {
                let number = self
                    .raw_value(node, b"id")
                    .and_then(|raw| raw.strip_prefix(prefix.as_bytes()))
                    .and_then(|rest| std::str::from_utf8(rest).ok())
                    .and_then(|text| text.parse::<u64>().ok());
                if let Some(number) = number {
                    next = next.max(number.saturating_add(1));
                }
            }
        }
        for _ in 0..MAX_AUTO_ID_ATTEMPTS {
            let id = format!("{prefix}{next}");
            if self.editor.try_find_id(id.as_bytes())?.is_none() {
                return Ok(id);
            }
            next += 1;
        }
        Err(V1DocumentError::Edit(EditError::Limit))
    }

    /// Appends `markup` to the top-level `<defs>`, creating `<defs>` as the
    /// root's first child when none exists. One history entry either way.
    fn insert_into_defs(&mut self, markup: &[u8]) -> Result<(), V1DocumentError> {
        if let Some(defs) = self.top_level_defs()?.first().copied() {
            self.editor.insert_xml_fragment(defs, markup)?;
            return Ok(());
        }
        let root = self.root_node()?;
        let span = self
            .editor
            .source_span(root)
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        let tag = self
            .editor
            .tag_name(root)
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?
            .to_vec();
        let mut scan = span.offset + 1 + tag.len();
        for index in 0..self.editor.attribute_count(root) {
            if let Some(attribute) = self.editor.attribute_at(root, index) {
                scan = scan.max(attribute.value.offset + attribute.value.length + 1);
            }
        }
        let tail = self
            .editor
            .span_bytes(SourceSpan {
                offset: scan,
                length: span.offset + span.length - scan,
            })
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        let close = tail
            .iter()
            .position(|byte| *byte == b'>')
            .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
        let (at, remove, mut replacement) = if close > 0 && tail[close - 1] == b'/' {
            (scan + close - 1, 2, b">".to_vec())
        } else {
            (scan + close + 1, 0, Vec::new())
        };
        replacement.extend_from_slice(b"<defs>");
        replacement.extend_from_slice(markup);
        replacement.extend_from_slice(b"</defs>");
        if remove == 2 {
            replacement.extend_from_slice(b"</");
            replacement.extend_from_slice(&tag);
            replacement.push(b'>');
        }
        self.editor.replace_source_span(
            SourceSpan {
                offset: at,
                length: remove,
            },
            &replacement,
        )?;
        self.editor.history_relabel_latest(HistoryAction::AddShape);
        Ok(())
    }

    /// Sets, appends or removes attributes on one start tag as a single
    /// history entry, splicing only the bytes between the first and last change.
    fn patch_attrs(
        &mut self,
        node: DocumentHandle,
        edits: &[AttrEdit<'_>],
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
            match (existing, value) {
                (Some(attribute), Some(value)) => plans.push((
                    attribute.value.offset,
                    attribute.value.length,
                    encode_attribute(value, attribute.quote)?,
                )),
                (Some(attribute), None) => {
                    let end = attribute.value.offset + attribute.value.length + 1;
                    let mut start = attribute.name.offset;
                    while start > 0
                        && self
                            .editor
                            .span_bytes(SourceSpan {
                                offset: start - 1,
                                length: 1,
                            })
                            .is_some_and(|b| matches!(b[0], b' ' | b'\t' | b'\r' | b'\n'))
                    {
                        start -= 1;
                    }
                    plans.push((start, end - start, Vec::new()));
                }
                (None, Some(value)) => {
                    appended.push(b' ');
                    appended.extend_from_slice(name);
                    appended.extend_from_slice(b"=\"");
                    appended.extend_from_slice(&encode_attribute(value, b'"')?);
                    appended.push(b'"');
                }
                (None, None) => {}
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
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }
}

fn parse_offset(raw: &[u8]) -> Option<f64> {
    let text = std::str::from_utf8(raw).ok()?.trim();
    match text.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f64>().ok().map(|v| v / 100.0),
        None => text.parse::<f64>().ok(),
    }
    .filter(|v| v.is_finite())
}

#[cfg(test)]
#[path = "paint_defs_tests.rs"]
mod tests;
