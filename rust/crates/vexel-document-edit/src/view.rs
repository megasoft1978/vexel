use crate::EditError;
use vexel_document::{DocumentHandle, OwnedDocumentModel};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentViewRow {
    pub handle: DocumentHandle,
    pub depth: usize,
}

pub fn document_view_node_count(view: &OwnedDocumentModel) -> usize {
    view.live_node_count()
}
pub fn document_view_root(view: &OwnedDocumentModel) -> Option<DocumentHandle> {
    view.root()
}
pub fn document_view_first_child(
    view: &OwnedDocumentModel,
    parent: DocumentHandle,
) -> Option<DocumentHandle> {
    view.first_child(parent)
}
pub fn document_view_next_sibling(
    view: &OwnedDocumentModel,
    node: DocumentHandle,
) -> Option<DocumentHandle> {
    view.next_sibling(node)
}
pub fn document_view_tag<'a>(
    view: &'a OwnedDocumentModel,
    node: DocumentHandle,
) -> Option<&'a [u8]> {
    view.tag_name(node)
}
pub fn document_view_attribute_count(view: &OwnedDocumentModel, node: DocumentHandle) -> usize {
    view.attribute_count(node)
}
pub fn document_view_attribute_at<'a>(
    view: &'a OwnedDocumentModel,
    node: DocumentHandle,
    index: usize,
) -> Option<(&'a [u8], &'a [u8])> {
    view.attribute_at(node, index)
}

/// Enumerates live rows from the stable slot interval. `next_slot` identifies
/// the first slot not examined when the caller's row buffer fills.
pub fn document_view_range(
    view: &OwnedDocumentModel,
    first_slot: usize,
    slot_limit: usize,
    rows: &mut [DocumentViewRow],
) -> Result<(usize, usize), EditError> {
    if first_slot > slot_limit || slot_limit > view.node_count() {
        return Err(EditError::InvalidArgument);
    }
    let mut cursor = first_slot;
    let mut count = 0usize;
    while cursor < slot_limit && count < rows.len() {
        if let Some(handle) = view.node_at(cursor) {
            let depth = view.depth(handle).ok_or(EditError::InvalidOperation)?;
            rows[count] = DocumentViewRow { handle, depth };
            count += 1;
        }
        cursor += 1;
    }
    Ok((count, cursor))
}
