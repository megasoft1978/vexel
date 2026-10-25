//! Bounded, read-only queries over the source-owned SVG document.

use crate::{ElementKind, V1Document, V1DocumentError};
use vexel_document_edit::{
    DocumentHandle, Editor, document_view_first_child, document_view_next_sibling,
};

const MAX_ATTRIBUTE_COUNT: usize = 64;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 256;

/// A bounded description of one SVG element.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementInfo {
    pub id: String,
    pub tag: String,
    pub kind: ElementKind,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub attributes: Vec<(String, String)>,
    pub bounds: Option<(f64, f64, f64, f64)>,
}

/// Filters elements by kind, ID prefix and an optional attribute predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub kind: Option<ElementKind>,
    pub id_prefix: Option<String>,
    /// Exact stable ID filter. When set, `id_prefix` remains available for compatibility.
    pub id: Option<String>,
    /// Exact source tag filter.
    pub tag: Option<String>,
    pub attribute: Option<(String, Option<String>)>,
    /// Number of matching elements to skip before collecting results.
    pub cursor: usize,
    pub limit: usize,
}

/// One node in a flat, pre-order tree page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeEntry {
    pub tag: String,
    pub id: Option<String>,
    pub depth: usize,
    pub parent_id: Option<String>,
}

/// A bounded tree representation of the SVG element hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub tag: String,
    pub id: Option<String>,
    pub children: Vec<TreeNode>,
}

impl V1Document {
    /// Returns a bounded description for an element with a stable source ID.
    pub fn element_get(&self, id: &str) -> Result<ElementInfo, V1DocumentError> {
        let node = self
            .editor
            .try_find_id(id.as_bytes())?
            .ok_or(V1DocumentError::MissingStableId)?;
        let mut parent = self.editor.model().parent(node);
        let mut parent_id = None;
        while let Some(candidate) = parent {
            if let Some(value) = raw_attr(&self.editor, candidate, b"id") {
                parent_id = Some(utf8(value)?);
                break;
            }
            parent = self.editor.model().parent(candidate);
        }
        let mut children = Vec::new();
        let mut child = document_view_first_child(self.editor.model(), node);
        while let Some(candidate) = child {
            if let Some(value) = raw_attr(&self.editor, candidate, b"id") {
                children.push(utf8(value)?);
            }
            child = document_view_next_sibling(self.editor.model(), candidate);
        }
        let mut attributes = Vec::new();
        for index in 0..self.editor.attribute_count(node).min(MAX_ATTRIBUTE_COUNT) {
            let Some(attr) = self.editor.attribute_at(node, index) else {
                continue;
            };
            let name = self
                .editor
                .span_bytes(attr.name)
                .ok_or(V1DocumentError::Unsupported)?;
            let value = self
                .editor
                .span_bytes(attr.value)
                .ok_or(V1DocumentError::Unsupported)?;
            attributes.push((
                utf8(name)?,
                utf8(truncate_utf8(value, MAX_ATTRIBUTE_VALUE_BYTES))?,
            ));
        }
        let tag = self
            .editor
            .tag_name(node)
            .ok_or(V1DocumentError::Unsupported)?;
        let bounds = self.geometry_bounds(id).ok();
        Ok(ElementInfo {
            id: id.to_owned(),
            tag: utf8(tag)?,
            kind: kind_of(&self.editor, node),
            parent: parent_id,
            children,
            attributes,
            bounds,
        })
    }

    /// Returns matching stable IDs in document order.
    pub fn element_query(&self, q: &Query) -> Result<Vec<String>, V1DocumentError> {
        if !(1..=512).contains(&q.limit) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let root = self.editor.root().ok_or(V1DocumentError::Unsupported)?;
        let mut stack = vec![root];
        let mut result = Vec::new();
        let mut matches_seen = 0usize;
        while let Some(node) = stack.pop() {
            if let Some(raw_id) = raw_attr(&self.editor, node, b"id") {
                let id = utf8(raw_id)?;
                let tag = self
                    .editor
                    .tag_name(node)
                    .ok_or(V1DocumentError::Unsupported)?;
                let kind_matches = q
                    .kind
                    .is_none_or(|kind| kind_of(&self.editor, node) == kind);
                let prefix_matches = q
                    .id_prefix
                    .as_ref()
                    .is_none_or(|prefix| id.starts_with(prefix));
                let id_matches = q.id.as_ref().is_none_or(|exact| id == *exact);
                let tag_matches = q
                    .tag
                    .as_ref()
                    .is_none_or(|expected| tag == expected.as_bytes());
                let attr_matches = match &q.attribute {
                    None => true,
                    Some((name, value)) => (0..self.editor.attribute_count(node)).any(|i| {
                        let Some(a) = self.editor.attribute_at(node, i) else {
                            return false;
                        };
                        if self.editor.span_bytes(a.name) != Some(name.as_bytes()) {
                            return false;
                        }
                        let Some(expected) = value else { return true };
                        decode_attr_matches(&self.editor, a.value, expected)
                    }),
                };
                if kind_matches && prefix_matches && id_matches && tag_matches && attr_matches {
                    if matches_seen >= q.cursor {
                        result.push(id);
                        if result.len() == q.limit {
                            break;
                        }
                    }
                    matches_seen = matches_seen.saturating_add(1);
                }
            }
            let mut children = Vec::new();
            let mut child = document_view_first_child(self.editor.model(), node);
            while let Some(candidate) = child {
                children.push(candidate);
                child = document_view_next_sibling(self.editor.model(), candidate);
            }
            stack.extend(children.into_iter().rev());
        }
        Ok(result)
    }

    /// Returns one flat pre-order page of the complete element tree.
    pub fn tree_page(
        &self,
        cursor: usize,
        limit: usize,
    ) -> Result<Vec<TreeEntry>, V1DocumentError> {
        if !(1..=512).contains(&limit) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let root = self.editor.root().ok_or(V1DocumentError::Unsupported)?;
        let mut stack = vec![(root, 0usize, None)];
        let mut entries = Vec::new();
        while let Some((node, depth, parent_id)) = stack.pop() {
            let tag = utf8(
                self.editor
                    .tag_name(node)
                    .ok_or(V1DocumentError::Unsupported)?,
            )?;
            let id = raw_attr(&self.editor, node, b"id").map(utf8).transpose()?;
            entries.push(TreeEntry {
                tag,
                id: id.clone(),
                depth,
                parent_id: parent_id.clone(),
            });
            let mut children = Vec::new();
            let mut child = document_view_first_child(self.editor.model(), node);
            while let Some(candidate) = child {
                children.push(candidate);
                child = document_view_next_sibling(self.editor.model(), candidate);
            }
            let nearest_id = id.clone().or(parent_id);
            stack.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, depth + 1, nearest_id.clone())),
            );
        }
        Ok(entries.into_iter().skip(cursor).take(limit).collect())
    }

    /// Returns the SVG tree, bounded by depth and total emitted nodes.
    pub fn tree_get(
        &self,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<(TreeNode, bool), V1DocumentError> {
        if max_depth > 16 || !(1..=1000).contains(&max_nodes) {
            return Err(V1DocumentError::InvalidGeometry);
        }
        let root = self.editor.root().ok_or(V1DocumentError::Unsupported)?;
        let mut remaining = max_nodes;
        let mut truncated = false;
        let tree = self.build_tree(root, 0, max_depth, &mut remaining, &mut truncated)?;
        Ok((tree, truncated))
    }

    fn build_tree(
        &self,
        node: DocumentHandle,
        depth: usize,
        max_depth: usize,
        remaining: &mut usize,
        truncated: &mut bool,
    ) -> Result<TreeNode, V1DocumentError> {
        *remaining -= 1;
        let tag = utf8(
            self.editor
                .tag_name(node)
                .ok_or(V1DocumentError::Unsupported)?,
        )?;
        let id = raw_attr(&self.editor, node, b"id").map(utf8).transpose()?;
        let mut children = Vec::new();
        let mut child = document_view_first_child(self.editor.model(), node);
        if depth == max_depth && child.is_some() {
            *truncated = true;
        }
        while depth < max_depth {
            let Some(candidate) = child else { break };
            if *remaining == 0 {
                *truncated = true;
                break;
            }
            children.push(self.build_tree(
                candidate,
                depth + 1,
                max_depth,
                remaining,
                truncated,
            )?);
            child = document_view_next_sibling(self.editor.model(), candidate);
        }
        Ok(TreeNode { tag, id, children })
    }
}

fn decode_attr_matches(
    editor: &Editor,
    span: vexel_document_edit::SourceSpan,
    expected: &str,
) -> bool {
    let Some(raw) = editor.span_bytes(span) else {
        return false;
    };
    if !raw.contains(&b'&') {
        return raw == expected.as_bytes();
    }
    let mut decoded = vec![0; raw.len()];
    editor
        .decoded_span(span, &mut decoded)
        .is_ok_and(|length| decoded[..length] == *expected.as_bytes())
}

fn kind_of(editor: &Editor, node: DocumentHandle) -> ElementKind {
    match editor.tag_name(node).unwrap_or_default() {
        b"rect" => ElementKind::Rect,
        b"ellipse" => ElementKind::Ellipse,
        b"circle" => ElementKind::Circle,
        b"line" => ElementKind::Line,
        b"path" => ElementKind::Path,
        b"g" => ElementKind::Group,
        b"text" => ElementKind::Text,
b"use" => ElementKind::Use,
        _ => ElementKind::Other,
    }
}

fn raw_attr<'a>(editor: &'a Editor, node: DocumentHandle, name: &[u8]) -> Option<&'a [u8]> {
    (0..editor.attribute_count(node)).find_map(|i| {
        let attr = editor.attribute_at(node, i)?;
        (editor.span_bytes(attr.name)? == name)
            .then(|| editor.span_bytes(attr.value))
            .flatten()
    })
}

fn utf8(bytes: &[u8]) -> Result<String, V1DocumentError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| V1DocumentError::Unsupported)
}

fn truncate_utf8(bytes: &[u8], limit: usize) -> &[u8] {
    let mut end = bytes.len().min(limit);
    while std::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    &bytes[..end]
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
