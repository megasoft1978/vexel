//! Renaming an element id together with the local references that name it.

use crate::{V1Document, V1DocumentError, validate_stable_id};
use vexel_document_edit::{HistoryAction, SourceSpan};

/// Byte ranges of `#old` references in `source`: `href="#old"` (either quote,
/// also `xlink:href`) and `url(#old)` (optionally quoted). Only the id text
/// is returned.
fn reference_sites(source: &[u8], old: &[u8]) -> Vec<(usize, usize)> {
    let mut sites = Vec::new();
    let mut at = 0;
    while at + old.len() < source.len() {
        let Some(found) = source[at..].iter().position(|b| *b == b'#') else {
            break;
        };
        let hash = at + found;
        at = hash + 1;
        let start = hash + 1;
        let end = start + old.len();
        if source.get(start..end) != Some(old) {
            continue;
        }
        let before = &source[..hash];
        let after = source.get(end).copied();
        let href = [b'"', b'\''].iter().any(|quote| {
            before.last() == Some(quote)
                && before[..before.len() - 1].ends_with(b"href=")
                && after == Some(*quote)
        });
        let url = |quote: Option<u8>| {
            let prefix_ok = match quote {
                None => before.ends_with(b"url("),
                Some(q) => {
                    before.last() == Some(&q) && before[..before.len() - 1].ends_with(b"url(")
                }
            };
            prefix_ok
                && match quote {
                    None => after == Some(b')'),
                    Some(q) => after == Some(q) && source.get(end + 1) == Some(&b')'),
                }
        };
        if href || url(None) || url(Some(b'"')) || url(Some(b'\'')) {
            sites.push((start, old.len()));
        }
    }
    sites
}

impl V1Document {
    /// Gives the element `old` the id `new` and rewrites the `href="#old"` and
    /// `url(#old)` references to it, as one history entry. `new` must be a
    /// valid id not used by any element.
    pub fn rename_element(&mut self, old: &str, new: &str) -> Result<(), V1DocumentError> {
        validate_stable_id(new)?;
        let node = self.find_node(old)?;
        if old == new {
            return Ok(());
        }
        if self.editor.try_find_id(new.as_bytes())?.is_some() {
            return Err(V1DocumentError::InvalidStableId);
        }
        let id_site = (0..self.editor.attribute_count(node))
            .filter_map(|index| self.editor.attribute_at(node, index))
            .find(|attribute| self.editor.span_bytes(attribute.name) == Some(b"id".as_slice()))
            .map(|attribute| (attribute.value.offset, attribute.value.length))
            .ok_or(V1DocumentError::MissingStableId)?;
        let source = self.source()?;
        let mut sites = reference_sites(&source, old.as_bytes());
        sites.push(id_site);
        sites.sort_unstable();
        sites.dedup();
        let first = sites[0].0;
        let last = sites[sites.len() - 1];
        let mut out = Vec::new();
        out.try_reserve(last.0 + last.1 - first + new.len() * sites.len())
            .map_err(|_| V1DocumentError::Allocation)?;
        let mut cursor = first;
        for (offset, length) in &sites {
            out.extend_from_slice(&source[cursor..*offset]);
            out.extend_from_slice(new.as_bytes());
            cursor = offset + length;
        }
        self.editor.replace_source_span(
            SourceSpan {
                offset: first,
                length: last.0 + last.1 - first,
            },
            &out,
        )?;
        self.editor
            .history_relabel_latest(HistoryAction::EditAttribute);
        Ok(())
    }
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod tests;
