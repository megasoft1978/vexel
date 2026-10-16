use crate::{alloc, cache::NodeInfo, *};
#[derive(Clone, Copy)]
struct GroupNode {
    handle: DocumentHandle,
    info: NodeInfo,
}
impl Editor {
    fn collect_group_nodes(
        &self,
        handles: &[DocumentHandle],
        require_groups: bool,
    ) -> Result<([Option<GroupNode>; MAX_GROUP_NODES], DocumentHandle), EditError> {
        if handles.is_empty() {
            return Err(EditError::InvalidArgument);
        }
        if handles.len() > MAX_GROUP_NODES {
            return Err(EditError::Limit);
        }
        let cache = self.index()?;
        let mut nodes = [None; MAX_GROUP_NODES];
        let mut common = None;
        for (i, &handle) in handles.iter().enumerate() {
            let info = cache.node(handle)?;
            let parent = cache.model.parent(handle).ok_or(EditError::StaleHandle)?;
            if common.is_some_and(|p| p != parent) {
                return Err(EditError::InvalidOperation);
            }
            common = Some(parent);
            if require_groups && info.name.slice(&cache.source) != Some(b"g") {
                return Err(EditError::InvalidOperation);
            }
            nodes[i] = Some(GroupNode { handle, info });
        }
        nodes[..handles.len()]
            .sort_unstable_by_key(|n| n.expect("filled group slot").info.span.offset);
        for pair in nodes[..handles.len()].windows(2) {
            let a = pair[0].expect("filled group slot");
            let b = pair[1].expect("filled group slot");
            if a.info.span.offset >= b.info.span.offset
                || a.info
                    .span
                    .offset
                    .checked_add(a.info.span.length)
                    .is_none_or(|end| end > b.info.span.offset)
                || cache.model.next_sibling(a.handle) != Some(b.handle)
            {
                return Err(EditError::InvalidOperation);
            }
        }
        Ok((nodes, common.ok_or(EditError::InvalidArgument)?))
    }
    pub fn group_siblings(
        &mut self,
        handles: &[DocumentHandle],
    ) -> Result<DocumentHandle, EditError> {
        let (nodes, parent) = self.collect_group_nodes(handles, false)?;
        let first = nodes[0].expect("nonempty groups").info.span;
        let last = nodes[handles.len() - 1].expect("nonempty groups").info.span;
        let end = last
            .offset
            .checked_add(last.length)
            .ok_or(EditError::Limit)?;
        let length = end.checked_sub(first.offset).ok_or(EditError::Limit)?;
        if length > MAX_FRAGMENT_BYTES - 7 {
            return Err(EditError::Limit);
        }
        let mut patch = alloc::vector(length + 7)?;
        patch.extend_from_slice(b"<g>");
        patch.extend_from_slice(
            self.span_bytes(SourceSpan {
                offset: first.offset,
                length,
            })
            .ok_or(EditError::InvalidOperation)?,
        );
        patch.extend_from_slice(b"</g>");
        let parent_span = self.index()?.node(parent)?.span;
        self.apply_returning(
            parent_span,
            first.offset,
            length,
            &patch,
            true,
            "Group",
            parent.slot,
            first.offset,
        )
    }
    pub fn ungroup_siblings(&mut self, handles: &[DocumentHandle]) -> Result<(), EditError> {
        let (nodes, parent) = self.collect_group_nodes(handles, true)?;
        let first = nodes[0].expect("nonempty groups").info.span;
        let last = nodes[handles.len() - 1].expect("nonempty groups").info.span;
        let end = last
            .offset
            .checked_add(last.length)
            .ok_or(EditError::Limit)?;
        let length = end.checked_sub(first.offset).ok_or(EditError::Limit)?;
        if length > MAX_FRAGMENT_BYTES {
            return Err(EditError::Limit);
        }
        let mut new_length = length;
        for node in nodes[..handles.len()].iter().flatten() {
            let info = node.info;
            let inner = if info.empty {
                0
            } else {
                info.close
                    .offset
                    .checked_sub(info.open.offset + info.open.length)
                    .ok_or(EditError::InvalidOperation)?
            };
            new_length = new_length
                .checked_sub(info.span.length - inner)
                .ok_or(EditError::InvalidOperation)?;
        }
        let cache = self.index()?;
        let parent_span = cache.node(parent)?.span;
        let mut patch = alloc::vector(new_length)?;
        let mut cursor = first.offset;
        for node in nodes[..handles.len()].iter().flatten() {
            let info = node.info;
            patch.extend_from_slice(
                cache
                    .source
                    .get(cursor..info.span.offset)
                    .ok_or(EditError::InvalidOperation)?,
            );
            if !info.empty {
                patch.extend_from_slice(
                    cache
                        .source
                        .get(info.open.offset + info.open.length..info.close.offset)
                        .ok_or(EditError::InvalidOperation)?,
                );
            }
            cursor = info.span.offset + info.span.length;
        }
        patch.extend_from_slice(
            cache
                .source
                .get(cursor..end)
                .ok_or(EditError::InvalidOperation)?,
        );
        if patch.len() != new_length {
            return Err(EditError::InvalidOperation);
        }
        self.apply_validated(
            parent_span,
            first.offset,
            length,
            &patch,
            true,
            false,
            "Ungroup",
            parent.slot,
            false,
        )
    }
}
