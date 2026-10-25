use crate::model::{checked_table_bytes, no_slot, Attribute, DocumentCore, DocumentModel, Node, OwnedDocumentModel, MAX_NODES};
use crate::types::allocate_document_id;
use crate::DocumentSourceSpan;
use std::mem::size_of;
use std::sync::Arc;
use vexel_xml::{StreamStatus, XmlDiagnostic, XmlError, XmlSpan, XmlToken, XmlTokenStream, TokenKind};

const TOKEN_STORAGE_LIMIT: usize = 64 * 1024 * 1024;
const MAX_DEPTH: usize = 256;
const MAX_OPEN_NAME_BYTES: usize = MAX_DEPTH * size_of::<XmlSpan>();
const STEP_BYTES: usize = 64 * 1024;
const LARGE_SOURCE_LIMIT: usize = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Large-source parsing errors. Input and XML failures retain the existing
/// parse diagnostic. Cancellation during XML scanning reports offset 0 because
/// the accepted XML stream exposes no cursor; cancellation while building the
/// index reports the next token's source offset. Source-cap checks precede
/// cancellation; parser diagnostics precede later cancellation polls.
pub enum LargeDocumentError {
    Parse(DocumentParseError),
    Canceled { offset: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Owned parsing errors use the same parse and cancellation offset rules as
/// [`LargeDocumentError`]. The caller's Arc is cloned only after success.
pub enum OwnedParseError {
    Parse(DocumentParseError),
    Canceled { offset: usize },
}

enum BuildFailure {
    Parse(DocumentParseError),
    Canceled { offset: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentError {
    InvalidArgument,
    Parse,
    Limit,
    ResourceExhausted,
    OutOfMemory,
    IdExhausted,
}

impl DocumentError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::Parse => "parse",
            Self::Limit => "limit",
            Self::ResourceExhausted => "resource_exhausted",
            Self::OutOfMemory => "out_of_memory",
            Self::IdExhausted => "id_exhausted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentParseError {
    pub error: DocumentError,
    pub diagnostic: XmlDiagnostic,
}

pub fn parse_document(source: &[u8]) -> Result<DocumentModel<'_>, DocumentParseError> {
    parse_document_with_storage(source, TOKEN_STORAGE_LIMIT)
}

pub(crate) fn parse_document_with_storage(source: &[u8], storage_limit: usize) -> Result<DocumentModel<'_>, DocumentParseError> {
    let core = parse_core(source, vexel_xml::MAX_INPUT_BYTES, storage_limit, &mut || false)
        .map_err(|failure| match failure { BuildFailure::Parse(error) => error, BuildFailure::Canceled { .. } => unreachable!("disabled cancellation") })?;
    Ok(DocumentModel { source, core })
}

pub fn parse_document_large(source: &[u8]) -> Result<DocumentModel<'_>, LargeDocumentError> {
    parse_document_large_cancellable(source, || false)
}

pub fn parse_document_large_cancellable<C: FnMut() -> bool>(source: &[u8], mut cancel: C) -> Result<DocumentModel<'_>, LargeDocumentError> {
    let core = parse_core(source, LARGE_SOURCE_LIMIT, TOKEN_STORAGE_LIMIT, &mut cancel)
        .map_err(|failure| match failure { BuildFailure::Parse(error) => LargeDocumentError::Parse(error), BuildFailure::Canceled { offset } => LargeDocumentError::Canceled { offset } })?;
    Ok(DocumentModel { source, core })
}

pub fn parse_owned_document(source: &Arc<Vec<u8>>) -> Result<OwnedDocumentModel, OwnedParseError> {
    let core = parse_core(source.as_slice(), vexel_xml::MAX_INPUT_BYTES, TOKEN_STORAGE_LIMIT, &mut || false)
        .map_err(|failure| match failure { BuildFailure::Parse(error) => OwnedParseError::Parse(error), BuildFailure::Canceled { offset } => OwnedParseError::Canceled { offset } })?;
    Ok(OwnedDocumentModel { source: Arc::clone(source), core })
}

pub fn parse_owned_document_large(source: &Arc<Vec<u8>>) -> Result<OwnedDocumentModel, OwnedParseError> {
    parse_owned_document_large_cancellable(source, || false)
}

pub fn parse_owned_document_large_cancellable<C: FnMut() -> bool>(source: &Arc<Vec<u8>>, mut cancel: C) -> Result<OwnedDocumentModel, OwnedParseError> {
    let core = parse_core(source.as_slice(), LARGE_SOURCE_LIMIT, TOKEN_STORAGE_LIMIT, &mut cancel)
        .map_err(|failure| match failure { BuildFailure::Parse(error) => OwnedParseError::Parse(error), BuildFailure::Canceled { offset } => OwnedParseError::Canceled { offset } })?;
    Ok(OwnedDocumentModel { source: Arc::clone(source), core })
}

fn parse_core<C: FnMut() -> bool>(source: &[u8], source_limit: usize, storage_limit: usize, cancel: &mut C) -> Result<DocumentCore, BuildFailure> {
    let parse_error = |error, diagnostic| BuildFailure::Parse(DocumentParseError { error, diagnostic });
    if storage_limit < size_of::<XmlToken>() {
        return Err(parse_error(DocumentError::ResourceExhausted, XmlDiagnostic::default()));
    }
    if source.len() > source_limit {
        return Err(parse_error(DocumentError::Limit, XmlDiagnostic { category: XmlError::InputLimit, offset: 0 }));
    }

    let mut stream = XmlTokenStream::new(source, storage_limit).map_err(|_| parse_error(DocumentError::OutOfMemory, XmlDiagnostic::default()))?;
    stream.reserve_token_hint(source.len() / 13 + 16);
    if stream.token_storage_bytes() > storage_limit
        || stream.open_name_storage_bytes() > MAX_OPEN_NAME_BYTES
    {
        return Err(parse_error(DocumentError::OutOfMemory, XmlDiagnostic::default()));
    }
    let status = loop {
        let status = stream.step_with_cancel(STEP_BYTES, cancel);
        if stream.token_storage_bytes() > storage_limit
            || stream.open_name_storage_bytes() > MAX_OPEN_NAME_BYTES
        {
            return Err(parse_error(DocumentError::OutOfMemory, stream.result().diagnostic));
        }
        if status != StreamStatus::Ok { break status; }
    };
    let result = stream.result();
    if status == StreamStatus::Canceled { return Err(BuildFailure::Canceled { offset: 0 }); }
    if status != StreamStatus::Complete || result.diagnostic.category != XmlError::None {
        let error = match result.diagnostic.category {
            XmlError::InputLimit | XmlError::NodeLimit | XmlError::DepthLimit | XmlError::AttributeLimit
            | XmlError::TextLimit => DocumentError::Limit,
            XmlError::OutputExhausted => DocumentError::ResourceExhausted,
            XmlError::None => DocumentError::OutOfMemory,
            _ => DocumentError::Parse,
        };
        return Err(parse_error(error, result.diagnostic));
    }

    if result.element_count == 0 || result.element_count > MAX_NODES || result.element_count > u32::MAX as usize
        || result.attribute_count > vexel_xml::MAX_TOTAL_ATTRIBUTES
        || checked_table_bytes::<Node>(result.element_count).is_none()
        || checked_table_bytes::<Attribute>(result.attribute_count).is_none()
    {
        return Err(parse_error(DocumentError::Limit, result.diagnostic));
    }
    let tokens = stream.take().ok_or_else(|| parse_error(DocumentError::Parse, result.diagnostic))?;
    if stream.token_storage_bytes() != 0 {
        return Err(parse_error(DocumentError::Parse, result.diagnostic));
    }

    let id = allocate_document_id().ok_or_else(|| parse_error(DocumentError::IdExhausted, result.diagnostic))?;
    let (nodes, attributes, root) = build_tables(source, &tokens, result.element_count, result.attribute_count, cancel)
        .map_err(|failure| match failure { TableFailure::Parse(error) => parse_error(error, result.diagnostic), TableFailure::Canceled { offset } => BuildFailure::Canceled { offset } })?;
    Ok(DocumentCore::from_parts(id, nodes, attributes, Some(root)))
}

enum TableFailure { Parse(DocumentError), Canceled { offset: usize } }
impl From<DocumentError> for TableFailure { fn from(error: DocumentError) -> Self { Self::Parse(error) } }

fn build_tables(
    source: &[u8],
    tokens: &[XmlToken],
    node_count: usize,
    attribute_count: usize,
    cancel: &mut impl FnMut() -> bool,
) -> Result<(Vec<Node>, Vec<Attribute>, u32), TableFailure> {
    let parse_error = |error| TableFailure::Parse(error);
    let mut nodes: Vec<Node> = Vec::new();
    nodes.try_reserve_exact(node_count).map_err(|_| parse_error(DocumentError::OutOfMemory))?;
    if nodes.capacity() > MAX_NODES { return Err(parse_error(DocumentError::OutOfMemory)); }
    let mut attributes: Vec<Attribute> = Vec::new();
    attributes.try_reserve_exact(attribute_count).map_err(|_| parse_error(DocumentError::OutOfMemory))?;
    if attributes.capacity() > vexel_xml::MAX_TOTAL_ATTRIBUTES { return Err(parse_error(DocumentError::OutOfMemory)); }
    let mut stack: Vec<u32> = Vec::new();
    stack.try_reserve_exact(MAX_DEPTH).map_err(|_| parse_error(DocumentError::OutOfMemory))?;
    if stack.capacity() > MAX_DEPTH { return Err(parse_error(DocumentError::OutOfMemory)); }

    let mut root = None;
    let mut index = 0usize;
    while index < tokens.len() {
        if cancel() { return Err(TableFailure::Canceled { offset: tokens.get(index).map_or(source.len(), |token| token.source.offset) }); }
        let token = tokens[index];
        match token.kind {
            TokenKind::StartElement | TokenKind::EmptyElement => {
                if nodes.len() >= node_count || nodes.len() >= MAX_NODES || nodes.len() >= u32::MAX as usize
                    || token.name.slice(source).is_none() || checked_span(token.source, source).is_none()
                {
                    return Err(parse_error(DocumentError::Parse));
                }
                let slot = u32::try_from(nodes.len()).map_err(|_| parse_error(DocumentError::Limit))?;
                let parent = stack.last().copied().unwrap_or(no_slot());
                let depth = stack.len();
                let mut node = Node {
                    name: token.name,
                    source: DocumentSourceSpan { offset: token.source.offset, length: token.source.length },
                    first_attribute: attributes.len(),
                    depth,
                    parent,
                    ..Node::default()
                };
                if parent != no_slot() {
                    let parent_index = parent as usize;
                    let parent_node = nodes.get(parent_index).ok_or_else(|| parse_error(DocumentError::Parse))?;
                    let previous_last = parent_node.last_child;
                    if parent_node.first_child == no_slot() { nodes[parent_index].first_child = slot; }
                    else {
                        let previous = nodes.get_mut(previous_last as usize).ok_or_else(|| parse_error(DocumentError::Parse))?;
                        previous.next_sibling = slot;
                    }
                    nodes[parent_index].last_child = slot;
                } else if parent == no_slot() {
                    if root.replace(slot).is_some() { return Err(parse_error(DocumentError::Parse)); }
                } else {
                    return Err(parse_error(DocumentError::Parse));
                }

                index += 1;
                while let Some(attribute) = tokens.get(index).filter(|next| next.kind == TokenKind::Attribute) {
                    if cancel() { return Err(TableFailure::Canceled { offset: attribute.source.offset }); }
                    if attributes.len() >= attribute_count || attribute.name.slice(source).is_none()
                        || attribute.value.slice(source).is_none() || checked_span(attribute.source, source).is_none()
                    {
                        return Err(parse_error(DocumentError::Parse));
                    }
                    attributes.push(Attribute { name: attribute.name, value: attribute.value });
                    node.attribute_count += 1;
                    index += 1;
                }
                nodes.push(node);
                if token.kind == TokenKind::StartElement {
                    if stack.len() >= MAX_DEPTH { return Err(parse_error(DocumentError::Limit)); }
                    stack.push(slot);
                }
            }
            TokenKind::EndElement => {
                let Some(slot) = stack.pop() else { return Err(parse_error(DocumentError::Parse)); };
                let node = nodes.get_mut(slot as usize).ok_or_else(|| parse_error(DocumentError::Parse))?;
                let end = token.source.offset.checked_add(token.source.length).ok_or_else(|| parse_error(DocumentError::Parse))?;
                let node_end = node.source.offset.checked_add(node.source.length).ok_or_else(|| parse_error(DocumentError::Parse))?;
                if node.source.offset > token.source.offset || end < node_end || end > source.len()
                    || token.name.slice(source).is_none()
                {
                    return Err(parse_error(DocumentError::Parse));
                }
                node.source.length = end.checked_sub(node.source.offset).ok_or(DocumentError::Parse)?;
                index += 1;
            }
            _ => index += 1,
        }
    }
    if !stack.is_empty() || nodes.len() != node_count || attributes.len() != attribute_count {
        return Err(parse_error(DocumentError::Parse));
    }
    Ok((nodes, attributes, root.ok_or(DocumentError::Parse)?))
}

fn checked_span<'a>(span: XmlSpan, source: &'a [u8]) -> Option<&'a [u8]> {
    let end = span.offset.checked_add(span.length)?;
    source.get(span.offset..end)
}

