//! Duplication that isolates paint and clip definitions.
//!
//! A copy gets fresh ids for its ID-bearing descendants. Every gradient or
//! clip definition it references is cloned into the top-level `<defs>` under
//! a fresh id, and the copy's `url(#...)` references are rewritten, so editing
//! the copy never changes the original. Subtree-internal definitions are
//! renamed with the subtree instead of cloned.

use crate::paint_defs::{CLIP_PREFIX, GRADIENT_PREFIX};
use crate::{V1Document, V1DocumentError};
use std::sync::Arc;
use vexel_document_edit::{
    EditError, EditLimits, Editor, HistoryAction, SourceSpan, document_view_first_child,
    document_view_next_sibling, encode_attribute,
};

/// An `id` attribute inside a fragment. `root` marks the fragment's own element.
struct IdSite {
    value: SourceSpan,
    quote: u8,
    root: bool,
    old: String,
}

/// An attribute holding `url(#...)` references (paint, clip or style).
struct UrlSite {
    value: SourceSpan,
    text: String,
}

#[derive(Default)]
struct Scan {
    ids: Vec<IdSite>,
    urls: Vec<UrlSite>,
}

/// Replaces `remove` bytes at `offset` with `bytes`.
type Splice = (usize, usize, Vec<u8>);

impl V1Document {
    /// Deep-copies an element immediately after it. Returns the copy's id.
    /// One edit, one history entry.
    pub(crate) fn duplicate_element(&mut self, id: &str) -> Result<String, V1DocumentError> {
        let node = self.find_node(id)?;
        let span = self
            .editor
            .source_span(node)
            .ok_or(V1DocumentError::MissingStableId)?;
        let original = self
            .editor
            .span_bytes(span)
            .ok_or(V1DocumentError::MissingStableId)?
            .to_vec();
        let scan = scan_fragment(&original)?;

        let mut reserved: Vec<String> = Vec::new();
        let mut fresh = Vec::with_capacity(scan.ids.len());
        let mut renames: Vec<(String, String)> = Vec::new();
        for site in &scan.ids {
            let new = self.next_stable_id_from(&reserved)?;
            reserved.push(new.clone());
            if !renames.iter().any(|(old, _)| *old == site.old) {
                renames.push((site.old.clone(), new.clone()));
            }
            fresh.push(new);
        }
        let root_id = scan
            .ids
            .iter()
            .zip(&fresh)
            .find(|(site, _)| site.root)
            .map(|(_, new)| new.clone())
            .ok_or(V1DocumentError::MissingStableId)?;

        // Clone each referenced definition once per subtree; shared references reuse the clone.
        let mut clones: Vec<(Vec<u8>, String)> = Vec::new();
        for site in &scan.urls {
            for target in url_targets(&site.text) {
                if renames.iter().any(|(old, _)| old == target) {
                    continue;
                }
                let Some((prefix, bytes)) = self.paint_def(target) else {
                    continue;
                };
                let new = self.next_def_id(prefix, &reserved)?;
                reserved.push(new.clone());
                renames.push((target.to_owned(), new.clone()));
                clones.push((bytes, new));
            }
        }

        let mut clone_markup = Vec::new();
        for (bytes, new_root) in &clones {
            let inner = scan_fragment(bytes)?;
            let mut inner_fresh = Vec::with_capacity(inner.ids.len());
            for site in &inner.ids {
                let new = if site.root {
                    new_root.clone()
                } else {
                    let stable = self.next_stable_id_from(&reserved)?;
                    reserved.push(stable.clone());
                    stable
                };
                inner_fresh.push(new);
            }
            clone_markup.extend(rewrite(bytes, &inner, &inner_fresh, &renames)?);
        }

        let copy = rewrite(&original, &scan, &fresh, &renames)?;
        let end = span.offset + span.length;
        let mut splices: Vec<Splice> = vec![(end, 0, copy)];
        if !clone_markup.is_empty() {
            let defs = self
                .top_level_defs()?
                .first()
                .copied()
                .ok_or(V1DocumentError::MissingStableId)?;
            let defs_span = self
                .editor
                .source_span(defs)
                .ok_or(V1DocumentError::MissingStableId)?;
            let defs_bytes = self
                .editor
                .span_bytes(defs_span)
                .ok_or(V1DocumentError::MissingStableId)?;
            let defs_end = defs_span.offset + defs_span.length;
            if defs_bytes.ends_with(b"/>") {
                let mut bytes = b">".to_vec();
                bytes.extend_from_slice(&clone_markup);
                bytes.extend_from_slice(b"</defs>");
                splices.push((defs_end - 2, 2, bytes));
            } else {
                let close = defs_bytes
                    .windows(2)
                    .rposition(|w| w == b"</")
                    .ok_or(V1DocumentError::MissingStableId)?;
                splices.push((defs_span.offset + close, 0, clone_markup));
            }
        }

        // Every splice lands in one contiguous region so the edit is a single history entry.
        splices.sort_by_key(|splice| splice.0);
        let start = splices.first().map_or(end, |splice| splice.0);
        let stop = splices
            .iter()
            .map(|splice| splice.0 + splice.1)
            .max()
            .unwrap_or(end);
        let region = SourceSpan {
            offset: start,
            length: stop - start,
        };
        let source: &[u8] = if region.length == 0 {
            &[]
        } else {
            self.editor
                .span_bytes(region)
                .ok_or(V1DocumentError::MissingStableId)?
        };
        let mut out = Vec::new();
        let mut cursor = start;
        for (offset, remove, bytes) in splices {
            out.extend_from_slice(&source[cursor - start..offset - start]);
            out.extend_from_slice(&bytes);
            cursor = offset + remove;
        }
        out.extend_from_slice(&source[cursor - start..]);
        self.editor.replace_source_span(region, &out)?;
        self.editor
            .history_relabel_latest(HistoryAction::EditObjects);
        Ok(root_id)
    }

    /// Returns the prefix and source bytes of a gradient or clip definition by id.
    fn paint_def(&self, id: &str) -> Option<(&'static str, Vec<u8>)> {
        let (prefix, node) = match self.def_child(id, &[b"linearGradient", b"radialGradient"]) {
            Ok(node) => (GRADIENT_PREFIX, node),
            Err(_) => (CLIP_PREFIX, self.def_child(id, &[b"clipPath"]).ok()?),
        };
        let span = self.editor.source_span(node)?;
        Some((prefix, self.editor.span_bytes(span)?.to_vec()))
    }
}

/// Parses a fragment and records its id attributes and `url(#...)` attributes.
fn scan_fragment(source: &[u8]) -> Result<Scan, V1DocumentError> {
    let parsed = Editor::create(&Arc::new(source.to_vec()), EditLimits::default());
    if parsed.error != EditError::Ok {
        return Err(V1DocumentError::Edit(parsed.error));
    }
    let editor = parsed
        .editor
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let model = editor.try_model()?;
    let root = editor
        .root()
        .ok_or(V1DocumentError::Edit(EditError::InvalidOperation))?;
    let mut out = Scan::default();
    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let mut child = document_view_first_child(model, node);
        while let Some(item) = child {
            nodes.push(item);
            child = document_view_next_sibling(model, item);
        }
        for index in 0..editor.attribute_count(node) {
            let Some(attribute) = editor.attribute_at(node, index) else {
                continue;
            };
            let Some(value) = editor.span_bytes(attribute.value) else {
                continue;
            };
            if editor.span_bytes(attribute.name) == Some(b"id") {
                let old =
                    std::str::from_utf8(value).map_err(|_| V1DocumentError::MissingStableId)?;
                out.ids.push(IdSite {
                    value: attribute.value,
                    quote: attribute.quote,
                    root: node == root,
                    old: old.to_owned(),
                });
            } else if let Ok(text) = std::str::from_utf8(value) {
                if text.contains("url(#") {
                    out.urls.push(UrlSite {
                        value: attribute.value,
                        text: text.to_owned(),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Applies fresh ids (in scan order) and `url(#...)` renames to a fragment.
fn rewrite(
    source: &[u8],
    scan: &Scan,
    fresh: &[String],
    renames: &[(String, String)],
) -> Result<Vec<u8>, V1DocumentError> {
    let mut splices: Vec<Splice> = Vec::new();
    for (site, new) in scan.ids.iter().zip(fresh) {
        splices.push((
            site.value.offset,
            site.value.length,
            encode_attribute(new.as_bytes(), site.quote)?,
        ));
    }
    for site in &scan.urls {
        let text = rename_urls(&site.text, renames);
        if text != site.text {
            splices.push((site.value.offset, site.value.length, text.into_bytes()));
        }
    }
    splices.sort_by_key(|splice| std::cmp::Reverse(splice.0));
    let mut out = source.to_vec();
    for (offset, length, bytes) in splices {
        out.splice(offset..offset + length, bytes);
    }
    Ok(out)
}

/// Ids referenced by `url(#id)` occurrences in an attribute value.
fn url_targets(text: &str) -> impl Iterator<Item = &str> {
    text.split("url(#")
        .skip(1)
        .filter_map(|rest| rest.split_once(')').map(|(id, _)| id))
}

fn rename_urls(text: &str, renames: &[(String, String)]) -> String {
    renames.iter().fold(text.to_owned(), |acc, (old, new)| {
        acc.replace(&format!("url(#{old})"), &format!("url(#{new})"))
    })
}
