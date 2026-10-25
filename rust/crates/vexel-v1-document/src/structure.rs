//! Source-preserving tree edits for V1 documents.

use crate::{ElementKind, V1Document, V1DocumentError, path_data};
use vexel_document_edit::{SourceSpan, document_view_first_child, document_view_next_sibling};

/// Most sibling ids one `group` call wraps.
pub const MAX_GROUP_IDS: usize = 1024;

/// Sibling z-order movement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reorder {
    Front,
    Back,
    Forward,
    Backward,
}

impl V1Document {
    /// Returns the editable geometry bounds of a simple shape or path in document space.
    pub fn geometry_bounds(&self, id: &str) -> Result<(f64, f64, f64, f64), V1DocumentError> {
        let node = self.shape(id)?;
        self.geometry_bounds_node(node)
    }

    fn geometry_bounds_node(
        &self,
        node: vexel_document_edit::DocumentHandle,
    ) -> Result<(f64, f64, f64, f64), V1DocumentError> {
        if self.kind_of(node) == ElementKind::Group {
            let model = self.editor.try_model()?;
            let mut child = document_view_first_child(model, node);
            let mut union: Option<(f64, f64, f64, f64)> = None;
            while let Some(item) = child {
                if self
                    .raw_attr(item, b"transform")
                    .is_some_and(|t| crate::draw::parse_translate(t).is_none())
                {
                    return Err(V1DocumentError::UnsupportedTransform);
                }
                let b = self.geometry_bounds_node(item)?;
                union = Some(match union {
                    Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
                    None => b,
                });
                child = document_view_next_sibling(model, item);
            }
            let mut bounds = union.ok_or(V1DocumentError::Unsupported)?;
            if let Some(raw) = self.raw_attr(node, b"transform") {
                let (tx, ty) = crate::draw::parse_translate(raw)
                    .ok_or(V1DocumentError::UnsupportedTransform)?;
                bounds.0 += tx;
                bounds.1 += ty;
                bounds.2 += tx;
                bounds.3 += ty;
            }
            return Ok(bounds);
        }
        let number = |name: &[u8], default: f64| match self.raw_attr(node, name) {
            None => Ok(default),
            Some(raw) => crate::draw::parse_number(raw).ok_or(V1DocumentError::InvalidGeometry),
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
            ElementKind::Circle => {
                let (cx, cy, radius) =
                    (number(b"cx", 0.0)?, number(b"cy", 0.0)?, number(b"r", 0.0)?);
                (cx - radius, cy - radius, cx + radius, cy + radius)
            }
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
            ElementKind::Group | ElementKind::Text | ElementKind::Use | ElementKind::Other => {
                return Err(V1DocumentError::Unsupported);
            }
        };
        if let Some(raw) = self.raw_attr(node, b"transform") {
            let (tx, ty) =
                crate::draw::parse_translate(raw).ok_or(V1DocumentError::UnsupportedTransform)?;
            bounds.0 += tx;
            bounds.1 += ty;
            bounds.2 += tx;
            bounds.3 += ty;
        }
        Ok(bounds)
    }

    /// Groups 1..=1024 sibling elements, preserving their document order.
    pub fn group(&mut self, ids: &[String]) -> Result<String, V1DocumentError> {
        if ids.is_empty() || ids.len() > MAX_GROUP_IDS {
            return Err(V1DocumentError::Unsupported);
        }
        let model = self.editor.try_model()?;
        let mut selected = Vec::new();
        selected
            .try_reserve_exact(ids.len())
            .map_err(|_| V1DocumentError::Allocation)?;
        let mut parent = None;
        for id in ids {
            let node = self.find_node(id)?;
            let p = model.parent(node).ok_or(V1DocumentError::Unsupported)?;
            if parent.is_some_and(|old| old != p) {
                return Err(V1DocumentError::Unsupported);
            }
            parent = Some(p);
            selected.push(node);
        }
        let parent = parent.ok_or(V1DocumentError::Unsupported)?;
        let mut children = Vec::new();
        let mut child = document_view_first_child(model, parent);
        while let Some(node) = child {
            children.push(node);
            child = document_view_next_sibling(model, node);
        }
        let order: Vec<_> = children
            .iter()
            .copied()
            .filter(|n| selected.contains(n))
            .collect();
        if order.len() != selected.len() {
            return Err(V1DocumentError::Unsupported);
        }
        let spans: Vec<_> = order
            .iter()
            .map(|n| {
                self.editor
                    .source_span(*n)
                    .ok_or(V1DocumentError::MissingStableId)
            })
            .collect::<Result<_, _>>()?;
        let first = spans.first().ok_or(V1DocumentError::Unsupported)?.offset;
        let last = spans.last().ok_or(V1DocumentError::Unsupported)?;
        let end = last
            .offset
            .checked_add(last.length)
            .ok_or(V1DocumentError::Unsupported)?;
        let id = self.next_stable_id()?;
        // Pull selected elements into one group; keep all intervening sibling bytes outside it.
        let replacement = self.group_selected_runs(first, end, &spans, id.as_bytes())?;
        self.editor.replace_source_span(
            SourceSpan {
                offset: first,
                length: end - first,
            },
            &replacement,
        )?;
        self.editor
            .history_relabel_latest(vexel_document_edit::HistoryAction::EditObjects);
        Ok(id)
    }

    fn group_selected_runs(
        &self,
        first: usize,
        end: usize,
        spans: &[SourceSpan],
        id: &[u8],
    ) -> Result<Vec<u8>, V1DocumentError> {
        let source = self.source()?;
        let mut out = Vec::new();
        out.try_reserve(end - first + id.len() + 32)
            .map_err(|_| V1DocumentError::Allocation)?;
        out.extend_from_slice(b"<g id=\"");
        out.extend_from_slice(id);
        out.extend_from_slice(b"\">");
        for span in spans {
            out.extend_from_slice(&source[span.offset..span.offset + span.length]);
        }
        out.extend_from_slice(b"</g>");
        let mut cursor = first;
        for span in spans {
            out.extend_from_slice(&source[cursor..span.offset]);
            cursor = span.offset + span.length;
        }
        out.extend_from_slice(&source[cursor..end]);
        Ok(out)
    }

    /// Ungroups a group. Groups carrying any transform are refused as unsupported.
    pub fn ungroup(&mut self, id: &str) -> Result<Vec<String>, V1DocumentError> {
        let node = self.find_node(id)?;
        if self.kind_of(node) != crate::ElementKind::Group {
            return Err(V1DocumentError::Unsupported);
        }
        if self.raw_attr(node, b"transform").is_some() {
            return Err(V1DocumentError::UnsupportedTransform);
        }
        let model = self.editor.try_model()?;
        let mut ids = Vec::new();
        let mut child = document_view_first_child(model, node);
        while let Some(n) = child {
            if let Some(raw) = self.raw_attr(n, b"id") {
                let text =
                    std::str::from_utf8(raw).map_err(|_| V1DocumentError::MissingStableId)?;
                ids.push(text.to_owned());
            }
            child = document_view_next_sibling(model, n);
        }
        let span = self
            .editor
            .source_span(node)
            .ok_or(V1DocumentError::MissingStableId)?;
        let source = self.source()?;
        let fragment = &source[span.offset..span.offset + span.length];
        let open = fragment
            .iter()
            .position(|b| *b == b'>')
            .ok_or(V1DocumentError::Unsupported)?
            + 1;
        let close = fragment
            .windows(2)
            .rposition(|w| w == b"</")
            .unwrap_or(open);
        let replacement = if close >= open {
            &fragment[open..close]
        } else {
            &[]
        };
        self.editor.replace_source_span(span, replacement)?;
        self.editor
            .history_relabel_latest(vexel_document_edit::HistoryAction::EditObjects);
        Ok(ids)
    }

    /// Deep-copies an element immediately after it, assigning fresh IDs to all ID-bearing descendants.
    pub fn duplicate(&mut self, id: &str) -> Result<String, V1DocumentError> {
        self.duplicate_element(id)
    }

    pub(crate) fn next_stable_id(&self) -> Result<String, V1DocumentError> {
        self.next_stable_id_from(&[])
    }

    pub(crate) fn next_stable_id_from(
        &self,
        reserved: &[String],
    ) -> Result<String, V1DocumentError> {
        for n in 1u64..u64::MAX {
            let id = format!("vexel-object-{n}");
            if !reserved.contains(&id) && self.editor.try_find_id(id.as_bytes())?.is_none() {
                return Ok(id);
            }
        }
        Err(V1DocumentError::FormattingLimit)
    }

    /// Reorders an element among its siblings. Moving to an existing edge is a no-op.
    pub fn reorder(&mut self, id: &str, to: Reorder) -> Result<(), V1DocumentError> {
        let node = self.find_node(id)?;
        let model = self.editor.try_model()?;
        let parent = model.parent(node).ok_or(V1DocumentError::Unsupported)?;
        let mut siblings = Vec::new();
        let mut c = document_view_first_child(model, parent);
        while let Some(n) = c {
            siblings.push(n);
            c = document_view_next_sibling(model, n);
        }
        let i = siblings
            .iter()
            .position(|n| *n == node)
            .ok_or(V1DocumentError::Unsupported)?;
        let j = match to {
            Reorder::Front => siblings.len() - 1,
            Reorder::Back => 0,
            Reorder::Forward => (i + 1).min(siblings.len() - 1),
            Reorder::Backward => i.saturating_sub(1),
        };
        if i == j {
            return Ok(());
        }
        let a = self
            .editor
            .source_span(node)
            .ok_or(V1DocumentError::MissingStableId)?;
        let b = self
            .editor
            .source_span(siblings[j])
            .ok_or(V1DocumentError::MissingStableId)?;
        let source = self.source()?;
        let start = a.offset.min(b.offset);
        let end = (a.offset + a.length).max(b.offset + b.length);
        let mut bytes = Vec::new();
        if a.offset < b.offset {
            bytes.extend_from_slice(&source[a.offset + a.length..end]);
            bytes.extend_from_slice(&source[a.offset..a.offset + a.length]);
        } else {
            bytes.extend_from_slice(&source[a.offset..a.offset + a.length]);
            bytes.extend_from_slice(&source[start..a.offset]);
        }
        self.editor.replace_source_span(
            SourceSpan {
                offset: start,
                length: end - start,
            },
            &bytes,
        )?;
        self.editor
            .history_relabel_latest(vexel_document_edit::HistoryAction::EditObjects);
        Ok(())
    }

    /// Moves an element immediately before or after a sibling target.
    pub fn reorder_relative(
        &mut self,
        id: &str,
        target: &str,
        before: bool,
    ) -> Result<(), V1DocumentError> {
        let node = self.find_node(id)?;
        let target_node = self.find_node(target)?;
        let model = self.editor.try_model()?;
        let parent = model.parent(node).ok_or(V1DocumentError::Unsupported)?;
        if model.parent(target_node) != Some(parent) || node == target_node {
            return Err(V1DocumentError::Unsupported);
        }
        let mut siblings = Vec::new();
        let mut child = document_view_first_child(model, parent);
        while let Some(item) = child {
            siblings.push(item);
            child = document_view_next_sibling(model, item);
        }
        let from = siblings
            .iter()
            .position(|item| *item == node)
            .ok_or(V1DocumentError::Unsupported)?;
        let target_index = siblings
            .iter()
            .position(|item| *item == target_node)
            .ok_or(V1DocumentError::Unsupported)?;
        let desired = if before {
            target_index
        } else {
            target_index + 1
        };
        let destination = if from < desired { desired - 1 } else { desired };
        if from == destination {
            return Ok(());
        }
        let a = self
            .editor
            .source_span(node)
            .ok_or(V1DocumentError::MissingStableId)?;
        let b = self
            .editor
            .source_span(siblings[destination])
            .ok_or(V1DocumentError::MissingStableId)?;
        let source = self.source()?;
        let start = a.offset.min(b.offset);
        let end = (a.offset + a.length).max(b.offset + b.length);
        let mut bytes = Vec::new();
        if a.offset < b.offset {
            bytes.extend_from_slice(&source[a.offset + a.length..end]);
            bytes.extend_from_slice(&source[a.offset..a.offset + a.length]);
        } else {
            bytes.extend_from_slice(&source[a.offset..a.offset + a.length]);
            bytes.extend_from_slice(&source[start..a.offset]);
        }
        self.editor.replace_source_span(
            SourceSpan {
                offset: start,
                length: end - start,
            },
            &bytes,
        )?;
        self.editor
            .history_relabel_latest(vexel_document_edit::HistoryAction::EditObjects);
        Ok(())
    }
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
