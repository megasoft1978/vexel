use crate::{alloc, cache::Cache, *};
use vexel_document::DocumentHandle;
impl Editor {
    pub fn set_attribute(
        &mut self,
        node: DocumentHandle,
        name: &[u8],
        value: &[u8],
    ) -> Result<(), EditError> {
        checked_name(name)?;
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        let Some(attr) = cache.attribute(info, name) else {
            return self.add_attribute(node, name, value);
        };
        let encoded = encode_attribute(value, attr.quote)?;
        self.apply_validated(
            span(info.open),
            attr.value.offset,
            attr.value.length,
            &encoded,
            false,
            true,
            "Edit attribute",
            node.slot,
            true,
        )
    }
    pub fn add_attribute(
        &mut self,
        node: DocumentHandle,
        name: &[u8],
        value: &[u8],
    ) -> Result<(), EditError> {
        checked_name(name)?;
        let encoded = encode_attribute(value, b'"')?;
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        if cache.attribute(info, name).is_some() {
            return Err(EditError::DuplicateAttribute);
        }
        let open = info
            .open
            .slice(&cache.source)
            .ok_or(EditError::InvalidOperation)?;
        let insert_at = start_tag_trailing_start(open);
        let size = name
            .len()
            .checked_add(encoded.len())
            .and_then(|n| n.checked_add(4))
            .ok_or(EditError::Limit)?;
        let mut patch = alloc::vector(size)?;
        patch.push(b' ');
        patch.extend_from_slice(name);
        patch.extend_from_slice(b"=\"");
        patch.extend_from_slice(&encoded);
        patch.push(b'"');
        self.apply_validated(
            span(info.open),
            info.open.offset + insert_at,
            0,
            &patch,
            false,
            true,
            "Add attribute",
            node.slot,
            false,
        )
    }
    pub fn remove_attribute(&mut self, node: DocumentHandle, name: &[u8]) -> Result<(), EditError> {
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        let attr = cache.attribute(info, name).ok_or(EditError::NotFound)?;
        // Attribute token source excludes its preceding whitespace. C retains it.
        self.apply_validated(
            span(info.open),
            attr.source.offset,
            attr.source.length,
            &[],
            false,
            true,
            "Remove attribute",
            node.slot,
            false,
        )
    }
    pub fn set_text_content(&mut self, node: DocumentHandle, text: &[u8]) -> Result<(), EditError> {
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        let encoded = encode_text(text)?;
        if info.empty {
            let name = info
                .name
                .slice(&cache.source)
                .ok_or(EditError::StaleHandle)?;
            let total = encoded
                .len()
                .checked_add(name.len())
                .and_then(|n| n.checked_add(4))
                .ok_or(EditError::Limit)?;
            let mut patch = alloc::vector(total)?;
            patch.push(b'>');
            patch.extend_from_slice(&encoded);
            patch.extend_from_slice(b"</");
            patch.extend_from_slice(name);
            patch.push(b'>');
            self.apply_validated(
                info.span,
                info.open.offset + info.open.length - 2,
                2,
                &patch,
                false,
                false,
                "Edit text",
                node.slot,
                false,
            )
        } else {
            let start = info.open.offset + info.open.length;
            self.apply_validated(
                info.span,
                start,
                info.close.offset - start,
                &encoded,
                false,
                false,
                "Edit text",
                node.slot,
                false,
            )
        }
    }
    pub fn delete_element(&mut self, node: DocumentHandle) -> Result<(), EditError> {
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        if cache.model.root() == Some(node) {
            return Err(EditError::RootDelete);
        }
        let parent = cache.model.parent(node).ok_or(EditError::StaleHandle)?;
        let region = cache.node(parent)?.span;
        self.apply_validated(
            region,
            info.span.offset,
            info.span.length,
            &[],
            false,
            false,
            "Delete",
            node.slot,
            false,
        )
    }
    pub fn duplicate_element(&mut self, node: DocumentHandle) -> Result<DocumentHandle, EditError> {
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        if cache.model.root() == Some(node) {
            return Err(EditError::InvalidOperation);
        }
        let parent = cache.model.parent(node).ok_or(EditError::StaleHandle)?;
        let region = cache.node(parent)?.span;
        let copy = self.copy_range(info.span.offset, info.span.length)?;
        let offset = info.span.offset + info.span.length;
        // Duplication deliberately preserves every byte, including nested IDs.
        self.apply_returning(
            region,
            offset,
            0,
            &copy,
            false,
            "Duplicate",
            node.slot,
            offset,
        )
    }
    pub fn reorder_sibling(
        &mut self,
        node: DocumentHandle,
        target: DocumentHandle,
        before: bool,
    ) -> Result<(), EditError> {
        let cache = self.handle_index(node)?;
        let a = cache.node(node)?;
        let b = cache.node(target)?;
        let parent = cache
            .model
            .parent(node)
            .ok_or(EditError::InvalidOperation)?;
        if node.slot == target.slot || cache.model.parent(target) != Some(parent) {
            return Err(EditError::InvalidOperation);
        }
        let region = cache.node(parent)?.span;
        if if before {
            a.span.offset < b.span.offset
        } else {
            a.span.offset > b.span.offset
        } {
            return Ok(());
        }
        let start = a.span.offset.min(b.span.offset);
        let end = (a.span.offset + a.span.length).max(b.span.offset + b.span.length);
        let old = self.copy_range(start, end - start)?;
        let mut patch = alloc::vector(old.len())?;
        if a.span.offset < b.span.offset {
            let gap = b.span.offset - (a.span.offset + a.span.length);
            patch.extend_from_slice(&old[a.span.length..a.span.length + gap]);
            patch.extend_from_slice(&old[a.span.length + gap..]);
            patch.extend_from_slice(&old[..a.span.length]);
        } else {
            let gap = a.span.offset - (b.span.offset + b.span.length);
            patch.extend_from_slice(&old[b.span.length + gap..]);
            patch.extend_from_slice(&old[..b.span.length]);
            patch.extend_from_slice(&old[b.span.length..b.span.length + gap]);
        }
        self.apply_validated(
            region,
            start,
            old.len(),
            &patch,
            false,
            false,
            "Move",
            node.slot,
            false,
        )
    }
    pub fn wrap_in_group(&mut self, node: DocumentHandle) -> Result<DocumentHandle, EditError> {
        let cache = self.handle_index(node)?;
        let info = cache.node(node)?;
        if cache.model.root() == Some(node) {
            return Err(EditError::InvalidOperation);
        }
        let mut patch = alloc::vector(info.span.length.checked_add(7).ok_or(EditError::Limit)?)?;
        patch.extend_from_slice(b"<g>");
        patch.extend_from_slice(
            self.span_bytes(info.span)
                .ok_or(EditError::InvalidOperation)?,
        );
        patch.extend_from_slice(b"</g>");
        self.apply_returning(
            info.span,
            info.span.offset,
            info.span.length,
            &patch,
            false,
            "Group",
            node.slot,
            info.span.offset,
        )
    }
    pub fn ungroup(&mut self, group: DocumentHandle) -> Result<(), EditError> {
        let cache = self.handle_index(group)?;
        let info = cache.node(group)?;
        if info.name.slice(&cache.source) != Some(b"g") {
            return Err(EditError::InvalidOperation);
        }
        let start = if info.empty {
            info.span.offset + info.span.length - 2
        } else {
            info.open.offset + info.open.length
        };
        let end = if info.empty { start } else { info.close.offset };
        let inner = self.copy_range(start, end - start)?;
        let root = cache.model.root() == Some(group);
        let region = if root {
            SourceSpan {
                offset: 0,
                length: self.byte_len,
            }
        } else {
            cache
                .node(cache.model.parent(group).ok_or(EditError::StaleHandle)?)?
                .span
        };
        self.apply_validated(
            region,
            info.span.offset,
            info.span.length,
            &inner,
            !root,
            false,
            "Ungroup",
            group.slot,
            false,
        )
    }
    pub fn replace_source_span(
        &mut self,
        span: SourceSpan,
        replacement: &[u8],
    ) -> Result<(), EditError> {
        self.index().map_err(|_| EditError::InvalidOperation)?;
        if span.offset > self.byte_len || span.length > self.byte_len - span.offset {
            return Err(EditError::InvalidOperation);
        }
        if span.length > MAX_FRAGMENT_BYTES || replacement.len() > MAX_FRAGMENT_BYTES {
            return Err(EditError::Limit);
        }
        self.apply_validated(
            SourceSpan {
                offset: 0,
                length: self.byte_len,
            },
            span.offset,
            span.length,
            replacement,
            false,
            false,
            "Edit path",
            0,
            false,
        )
    }
    pub fn replace_xml_region(
        &mut self,
        parent: DocumentHandle,
        region: SourceSpan,
        replacement: &[u8],
    ) -> Result<(), EditError> {
        self.replace_xml_region_labeled(parent, region, replacement, HistoryAction::EditObjects)
    }
    pub fn replace_xml_region_labeled(
        &mut self,
        parent: DocumentHandle,
        region: SourceSpan,
        replacement: &[u8],
        action: HistoryAction,
    ) -> Result<(), EditError> {
        let info = self.handle_index(parent)?.node(parent)?;
        if region.offset < info.span.offset
            || region.offset - info.span.offset > info.span.length
            || region.length > info.span.length - (region.offset - info.span.offset)
        {
            return Err(EditError::InvalidOperation);
        }
        if region.length > MAX_FRAGMENT_BYTES || replacement.len() > MAX_FRAGMENT_BYTES {
            return Err(EditError::Limit);
        }
        self.apply_validated(
            info.span,
            region.offset,
            region.length,
            replacement,
            true,
            false,
            action.label(),
            parent.slot,
            false,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_validated(
        &mut self,
        region: SourceSpan,
        offset: usize,
        remove: usize,
        insert: &[u8],
        fragment: bool,
        start_tag: bool,
        label: &'static str,
        target: u32,
        coalescible: bool,
    ) -> Result<(), EditError> {
        self.validate_replace(region, offset, remove, insert, fragment, start_tag)?;
        self.replace(offset, remove, insert, label, target, coalescible)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_returning(
        &mut self,
        region: SourceSpan,
        offset: usize,
        remove: usize,
        insert: &[u8],
        fragment: bool,
        label: &'static str,
        target: u32,
        returned_offset: usize,
    ) -> Result<DocumentHandle, EditError> {
        self.validate_replace(region, offset, remove, insert, fragment, false)?;
        let cache = self.prepared_cache(offset, remove, insert)?;
        let handle =
            handle_at_offset(&cache, returned_offset).ok_or(EditError::InvalidOperation)?;
        self.publish_with_cache(offset, remove, insert, label, target, cache)?;
        Ok(handle)
    }
}
pub(crate) fn handle_at_offset(cache: &Cache, offset: usize) -> Option<DocumentHandle> {
    let mut best = None;
    for (slot, node) in cache.nodes.iter().enumerate() {
        if offset < node.span.offset || offset - node.span.offset >= node.span.length {
            continue;
        }
        let handle = cache.model.node_at(slot)?;
        let depth = cache.model.depth(handle)?;
        if best.is_none_or(|(d, _)| depth > d) {
            best = Some((depth, handle));
        }
    }
    best.map(|(_, h)| h)
}
pub(crate) fn span(s: vexel_xml::XmlSpan) -> SourceSpan {
    SourceSpan {
        offset: s.offset,
        length: s.length,
    }
}
pub(crate) fn start_tag_trailing_start(bytes: &[u8]) -> usize {
    if bytes.len() < 2 || bytes.last() != Some(&b'>') {
        return bytes.len();
    }
    let mut at = bytes.len() - 1;
    if at > 0 && bytes[at - 1] == b'/' {
        at -= 1;
    }
    while at > 0 && matches!(bytes[at - 1], b' ' | b'\t' | b'\r' | b'\n') {
        at -= 1;
    }
    at
}
pub fn name_valid(name: &[u8]) -> bool {
    checked_name(name).is_ok()
}
pub(crate) fn checked_name(name: &[u8]) -> Result<(), EditError> {
    if name.is_empty() || name.len() > 4096 {
        return Err(EditError::InvalidName);
    }
    let mut probe = alloc::vector(name.len().checked_add(13).ok_or(EditError::Limit)?)?;
    probe.extend_from_slice(b"<probe ");
    probe.extend_from_slice(name);
    probe.extend_from_slice(b"=\"x\"/>");
    crate::cache::parse_tokens(&probe)
        .map(|_| ())
        .map_err(|(e, _)| {
            if e == EditError::OutOfMemory {
                e
            } else {
                EditError::InvalidName
            }
        })
}
pub fn encode_attribute(input: &[u8], quote: u8) -> Result<Vec<u8>, EditError> {
    if !matches!(quote, b'\'' | b'"') {
        return Err(EditError::InvalidArgument);
    }
    encode(input, quote, false)
}
pub fn encode_text(input: &[u8]) -> Result<Vec<u8>, EditError> {
    encode(input, 0, true)
}
fn encode(input: &[u8], quote: u8, text: bool) -> Result<Vec<u8>, EditError> {
    let capacity = input
        .len()
        .checked_mul(6)
        .and_then(|n| n.checked_add(1))
        .ok_or(EditError::OutOfMemory)?;
    let mut out = alloc::vector(if input.is_empty() { 0 } else { capacity })?;
    for &b in input {
        let replacement: &[u8] = match b {
            b'&' => b"&amp;",
            b'<' => b"&lt;",
            b'>' if text => b"&gt;",
            b'\'' if !text && quote == b'\'' => b"&apos;",
            b'"' if !text && quote == b'"' => b"&quot;",
            _ => {
                out.push(b);
                continue;
            }
        };
        out.extend_from_slice(replacement);
    }
    Ok(out)
}
