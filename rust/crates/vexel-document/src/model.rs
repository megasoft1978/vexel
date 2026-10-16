use crate::{DocumentHandle, DocumentSourceSpan};
use std::mem::size_of;
use std::sync::Arc;

pub(crate) const MAX_NODES: usize = 1_000_000;
const NO_SLOT: u32 = u32::MAX;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Node {
    pub name: vexel_xml::XmlSpan,
    pub source: DocumentSourceSpan,
    pub first_attribute: usize,
    pub attribute_count: usize,
    pub depth: usize,
    pub parent: u32,
    pub first_child: u32,
    pub last_child: u32,
    pub next_sibling: u32,
    pub generation: u32,
    pub live: bool,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: vexel_xml::XmlSpan::default(),
            source: DocumentSourceSpan::default(),
            first_attribute: 0,
            attribute_count: 0,
            depth: 0,
            parent: NO_SLOT,
            first_child: NO_SLOT,
            last_child: NO_SLOT,
            next_sibling: NO_SLOT,
            generation: 1,
            live: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Attribute {
    pub name: vexel_xml::XmlSpan,
    pub value: vexel_xml::XmlSpan,
}

/// Index data shared by borrowed and Arc-owned source models.
#[derive(Debug)]
pub(crate) struct DocumentCore {
    pub(crate) id: u64,
    pub(crate) nodes: Vec<Node>,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) live_node_count: usize,
    pub(crate) root: Option<u32>,
}

impl DocumentCore {
    pub(crate) fn from_parts(
        id: u64,
        nodes: Vec<Node>,
        attributes: Vec<Attribute>,
        root: Option<u32>,
    ) -> Self {
        let live_node_count = nodes.len();
        Self { id, nodes, attributes, live_node_count, root }
    }

    pub fn document_id(&self) -> u64 { self.id }
    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn live_node_count(&self) -> usize { self.live_node_count }

    pub fn root(&self) -> Option<DocumentHandle> {
        let slot = self.root?;
        self.nodes.get(slot as usize).filter(|node| node.live)?;
        Some(self.handle(slot))
    }

    pub fn first_child(&self, parent: DocumentHandle) -> Option<DocumentHandle> {
        let node = self.resolve(parent)?;
        let mut child = node.first_child;
        while child != NO_SLOT {
            let current = self.nodes.get(child as usize)?;
            if current.live { return Some(self.handle(child)); }
            child = current.next_sibling;
        }
        None
    }

    pub fn next_sibling(&self, handle: DocumentHandle) -> Option<DocumentHandle> {
        let node = self.resolve(handle)?;
        let mut sibling = node.next_sibling;
        while sibling != NO_SLOT {
            let current = self.nodes.get(sibling as usize)?;
            if current.live { return Some(self.handle(sibling)); }
            sibling = current.next_sibling;
        }
        None
    }

    pub fn parent(&self, handle: DocumentHandle) -> Option<DocumentHandle> {
        let node = self.resolve(handle)?;
        if node.parent == NO_SLOT { return None; }
        self.nodes.get(node.parent as usize).filter(|parent| parent.live)?;
        Some(self.handle(node.parent))
    }

    pub fn attribute_count(&self, handle: DocumentHandle) -> usize {
        self.resolve(handle).map_or(0, |node| node.attribute_count)
    }

    fn attribute_spans(&self, handle: DocumentHandle, index: usize) -> Option<(vexel_xml::XmlSpan, vexel_xml::XmlSpan)> {
        let node = self.resolve(handle)?;
        if index >= node.attribute_count { return None; }
        let attribute = self.attributes.get(node.first_attribute.checked_add(index)?)?;
        Some((attribute.name, attribute.value))
    }

    pub fn depth(&self, handle: DocumentHandle) -> Option<usize> {
        Some(self.resolve(handle)?.depth)
    }

    pub fn node_at(&self, slot: usize) -> Option<DocumentHandle> {
        let index = u32::try_from(slot).ok()?;
        self.nodes.get(slot).filter(|node| node.live)?;
        Some(self.handle(index))
    }

    pub fn source_span(&self, handle: DocumentHandle) -> Option<DocumentSourceSpan> {
        Some(self.resolve(handle)?.source)
    }

    /// Removes a node and all descendants. Slots remain stable; each live
    /// generation advances so previously issued handles cannot resolve.
    pub fn remove_subtree(&mut self, handle: DocumentHandle) -> bool {
        if self.resolve(handle).is_none() { return false; }
        for index in 0..self.nodes.len() {
            if !self.is_descendant_or_self(index, handle.slot as usize) { continue; }
            let node = &mut self.nodes[index];
            if node.live {
                node.live = false;
                self.live_node_count -= 1;
                if node.generation != u32::MAX { node.generation += 1; }
            }
        }
        if self.root == Some(handle.slot) { self.root = None; }
        true
    }

    fn is_descendant_or_self(&self, index: usize, ancestor: usize) -> bool {
        let mut current = index;
        loop {
            if current == ancestor { return true; }
            let Some(node) = self.nodes.get(current) else { return false; };
            if node.parent == NO_SLOT { return false; }
            current = node.parent as usize;
        }
    }

    fn resolve(&self, handle: DocumentHandle) -> Option<&Node> {
        if handle.document_id != self.id { return None; }
        let node = self.nodes.get(handle.slot as usize)?;
        (node.live && node.generation == handle.generation).then_some(node)
    }

    fn handle(&self, slot: u32) -> DocumentHandle {
        DocumentHandle { document_id: self.id, slot, generation: self.nodes[slot as usize].generation }
    }
}

/// An immutable-source document with safely owned indexes and mutable live
/// state. The source borrow must outlive this model.
#[derive(Debug)]
pub struct DocumentModel<'source> {
    pub(crate) source: &'source [u8],
    pub(crate) core: DocumentCore,
}

/// A source-owning model. Cloning this value is intentionally unsupported so
/// each document identity has exactly one owner and one live-state table.
#[derive(Debug)]
pub struct OwnedDocumentModel {
    pub(crate) source: Arc<Vec<u8>>,
    pub(crate) core: DocumentCore,
}

macro_rules! document_query_methods {
    ($bytes:ty) => {
            pub fn document_id(&self) -> u64 { self.core.document_id() }
            pub fn source(&self) -> $bytes { &self.source }
            pub fn node_count(&self) -> usize { self.core.node_count() }
            pub fn live_node_count(&self) -> usize { self.core.live_node_count() }
            pub fn root(&self) -> Option<DocumentHandle> { self.core.root() }
            pub fn first_child(&self, parent: DocumentHandle) -> Option<DocumentHandle> { self.core.first_child(parent) }
            pub fn next_sibling(&self, handle: DocumentHandle) -> Option<DocumentHandle> { self.core.next_sibling(handle) }
            pub fn parent(&self, handle: DocumentHandle) -> Option<DocumentHandle> { self.core.parent(handle) }
            pub fn tag_name(&self, handle: DocumentHandle) -> Option<$bytes> {
                self.core.resolve(handle)?.name.slice(&self.source)
            }
            pub fn attribute_count(&self, handle: DocumentHandle) -> usize { self.core.attribute_count(handle) }
            pub fn attribute_at(&self, handle: DocumentHandle, index: usize) -> Option<($bytes, $bytes)> {
                let (name, value) = self.core.attribute_spans(handle, index)?;
                Some((name.slice(&self.source)?, value.slice(&self.source)?))
            }
            pub fn depth(&self, handle: DocumentHandle) -> Option<usize> { self.core.depth(handle) }
            pub fn node_at(&self, slot: usize) -> Option<DocumentHandle> { self.core.node_at(slot) }
            pub fn source_span(&self, handle: DocumentHandle) -> Option<DocumentSourceSpan> { self.core.source_span(handle) }
            pub fn remove_subtree(&mut self, handle: DocumentHandle) -> bool { self.core.remove_subtree(handle) }
    };
}

impl<'source> DocumentModel<'source> {
    document_query_methods!(&'source [u8]);
}

impl OwnedDocumentModel {
    document_query_methods!(&[u8]);

    pub fn source_owner(&self) -> &Arc<Vec<u8>> { &self.source }
}

pub(crate) fn checked_table_bytes<T>(count: usize) -> Option<usize> {
    count.checked_mul(size_of::<T>())
}

pub(crate) const fn no_slot() -> u32 { NO_SLOT }
