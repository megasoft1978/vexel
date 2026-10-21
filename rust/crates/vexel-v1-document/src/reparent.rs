//! Moving an element into another group in one source edit.

use crate::{V1Document, V1DocumentError};
use vexel_document_edit::{SourceSpan, document_view_first_child, document_view_next_sibling};

impl V1Document {
    /// Moves `id` into `parent_id` (`"root"` or a group), before the
    /// `index`-th child of the destination counted without `id` itself, or last
    /// when `index` is absent or past the end. The element keeps its id and
    /// bytes; transforms of the old and new ancestors are not compensated.
    pub fn reparent(
        &mut self,
        id: &str,
        parent_id: &str,
        index: Option<usize>,
    ) -> Result<(), V1DocumentError> {
        let node = self.find_node(id)?;
        let model = self.editor.try_model()?;
        let old_parent = model.parent(node).ok_or(V1DocumentError::Unsupported)?;
        let parent = if parent_id == "root" {
            self.editor.root().ok_or(V1DocumentError::Unsupported)?
        } else {
            let parent = self.find_node(parent_id)?;
            if self.editor.tag_name(parent) != Some(b"g") {
                return Err(V1DocumentError::Unsupported);
            }
            parent
        };
        let mut up = Some(parent);
        while let Some(at) = up {
            if at == node {
                return Err(V1DocumentError::Unsupported);
            }
            up = model.parent(at);
        }
        let mut siblings = Vec::new();
        let mut child = document_view_first_child(model, parent);
        while let Some(item) = child {
            if item != node {
                siblings.push(item);
            }
            child = document_view_next_sibling(model, item);
        }
        let moved = self
            .editor
            .source_span(node)
            .ok_or(V1DocumentError::MissingStableId)?;
        let source = self.source()?;
        let element = &source[moved.offset..moved.offset + moved.length];
        let mut edits: Vec<(usize, usize, Vec<u8>)> =
            vec![(moved.offset, moved.offset + moved.length, Vec::new())];
        let position = index.unwrap_or(siblings.len()).min(siblings.len());
        if let Some(&next) = siblings.get(position) {
            let at = self
                .editor
                .source_span(next)
                .ok_or(V1DocumentError::MissingStableId)?
                .offset;
            edits.push((at, at, element.to_vec()));
        } else if let Some(&last) = siblings.last() {
            let span = self
                .editor
                .source_span(last)
                .ok_or(V1DocumentError::MissingStableId)?;
            let at = span.offset + span.length;
            edits.push((at, at, element.to_vec()));
        } else if old_parent == parent {
            return Ok(());
        } else {
            // An empty destination group: rewrite its tag around the element.
            let span = self
                .editor
                .source_span(parent)
                .ok_or(V1DocumentError::MissingStableId)?;
            let tag = self
                .editor
                .tag_name(parent)
                .ok_or(V1DocumentError::Unsupported)?
                .to_vec();
            let whole = &source[span.offset..span.offset + span.length];
            let mut group = Vec::new();
            if whole.ends_with(b"/>") {
                let head = whole[..whole.len() - 2].trim_ascii_end();
                group.extend_from_slice(head);
                group.push(b'>');
                group.extend_from_slice(element);
                group.extend_from_slice(b"</");
                group.extend_from_slice(&tag);
                group.push(b'>');
            } else {
                let close = whole
                    .windows(2)
                    .rposition(|w| w == b"</")
                    .ok_or(V1DocumentError::Unsupported)?;
                group.extend_from_slice(&whole[..close]);
                group.extend_from_slice(element);
                group.extend_from_slice(&whole[close..]);
            }
            edits.push((span.offset, span.offset + span.length, group));
        }
        edits.sort_by_key(|(start, end, _)| (*start, *end));
        let start = edits[0].0;
        let end = edits.iter().map(|edit| edit.1).max().unwrap_or(start);
        let mut out = Vec::new();
        out.try_reserve(end - start + element.len())
            .map_err(|_| V1DocumentError::Allocation)?;
        let mut cursor = start;
        for (from, to, bytes) in &edits {
            out.extend_from_slice(&source[cursor..*from]);
            out.extend_from_slice(bytes);
            cursor = cursor.max(*to);
        }
        out.extend_from_slice(&source[cursor..end]);
        if out == source[start..end] {
            return Ok(());
        }
        self.editor.replace_source_span(
            SourceSpan {
                offset: start,
                length: end - start,
            },
            &out,
        )?;
        self.editor
            .history_relabel_latest(vexel_document_edit::HistoryAction::EditObjects);
        Ok(())
    }
}

#[cfg(test)]
#[path = "reparent_tests.rs"]
mod tests;
