use crate::{EditError, alloc};
use std::sync::Arc;
use vexel_document::{
    DocumentError, DocumentHandle, DocumentSourceSpan, OwnedDocumentModel, OwnedParseError,
    parse_owned_document,
};
use vexel_xml::{
    StreamStatus, TokenKind, XmlDiagnostic, XmlError, XmlSpan, XmlToken, XmlTokenStream,
};
pub(crate) const TOKEN_BYTES: usize = 64 * 1024 * 1024;
#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeInfo {
    pub span: DocumentSourceSpan,
    pub name: XmlSpan,
    pub open: XmlSpan,
    pub close: XmlSpan,
    pub first_attribute: usize,
    pub attribute_count: usize,
    pub empty: bool,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct AttributeInfo {
    pub name: XmlSpan,
    pub value: XmlSpan,
    pub source: XmlSpan,
    pub quote: u8,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextInfo {
    pub value: XmlSpan,
    pub cdata: bool,
}
#[derive(Debug)]
pub(crate) struct Cache {
    pub model: OwnedDocumentModel,
    pub source: Arc<Vec<u8>>,
    pub tokens: Vec<XmlToken>,
    pub nodes: Vec<NodeInfo>,
    pub attributes: Vec<AttributeInfo>,
    pub texts: Vec<TextInfo>,
}
pub(crate) type ParseFailure = (EditError, XmlDiagnostic);
pub(crate) fn parse_tokens(source: &[u8]) -> Result<Vec<XmlToken>, ParseFailure> {
    alloc::checkpoint().map_err(|e| (e, XmlDiagnostic::default()))?;
    let mut stream = XmlTokenStream::new(source, TOKEN_BYTES)
        .map_err(|_| (EditError::OutOfMemory, XmlDiagnostic::default()))?;
    // One sized allocation instead of doubling through several large blocks;
    // the allocator keeps those freed blocks resident across repeated parses.
    stream.reserve_token_hint(source.len() / 13 + 16);
    let status = loop {
        let s = stream.step(64 * 1024, false);
        if s != StreamStatus::Ok {
            break s;
        }
    };
    let diagnostic = stream.result().diagnostic;
    if status != StreamStatus::Complete || diagnostic.category != XmlError::None {
        return Err((
            if diagnostic.category == XmlError::OutputExhausted {
                EditError::Limit
            } else {
                EditError::MalformedXml
            },
            diagnostic,
        ));
    }
    if stream.token_storage_bytes() > TOKEN_BYTES {
        return Err((EditError::Limit, diagnostic));
    }
    stream.take().ok_or((EditError::OutOfMemory, diagnostic))
}
impl Cache {
    pub fn build(source: Arc<Vec<u8>>) -> Result<Self, ParseFailure> {
        alloc::checkpoint().map_err(|e| (e, XmlDiagnostic::default()))?;
        let model = parse_owned_document(&source).map_err(|err| match err {
            OwnedParseError::Parse(p) => (
                match p.error {
                    DocumentError::Limit | DocumentError::ResourceExhausted => EditError::Limit,
                    DocumentError::Parse => EditError::MalformedXml,
                    _ => EditError::OutOfMemory,
                },
                p.diagnostic,
            ),
            OwnedParseError::Canceled { offset } => (
                EditError::InvalidOperation,
                XmlDiagnostic {
                    category: XmlError::None,
                    offset,
                },
            ),
        })?;
        let tokens = parse_tokens(&source)?;
        let mut nodes =
            alloc::vector(model.node_count()).map_err(|e| (e, XmlDiagnostic::default()))?;
        let attributes_count = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Attribute)
            .count();
        let texts_count = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Text | TokenKind::Cdata) && t.depth > 0)
            .count();
        let mut attributes =
            alloc::vector(attributes_count).map_err(|e| (e, XmlDiagnostic::default()))?;
        let mut texts = alloc::vector(texts_count).map_err(|e| (e, XmlDiagnostic::default()))?;
        let mut stack = [0usize; 256];
        let mut depth = 0usize;
        let mut index = 0usize;
        while index < tokens.len() {
            let token = tokens[index];
            match token.kind {
                TokenKind::StartElement | TokenKind::EmptyElement => {
                    if depth >= stack.len() || nodes.len() >= model.node_count() {
                        return Err((EditError::MalformedXml, XmlDiagnostic::default()));
                    }
                    let mut node = NodeInfo {
                        span: DocumentSourceSpan {
                            offset: token.source.offset,
                            length: token.source.length,
                        },
                        name: token.name,
                        open: token.source,
                        close: XmlSpan {
                            offset: token
                                .source
                                .end()
                                .ok_or((EditError::Limit, XmlDiagnostic::default()))?,
                            length: 0,
                        },
                        first_attribute: attributes.len(),
                        attribute_count: 0,
                        empty: token.kind == TokenKind::EmptyElement,
                    };
                    while index + 1 < tokens.len() && tokens[index + 1].kind == TokenKind::Attribute
                    {
                        index += 1;
                        let attr = tokens[index];
                        let quote = *source
                            .get(
                                attr.value
                                    .offset
                                    .checked_sub(1)
                                    .ok_or((EditError::MalformedXml, XmlDiagnostic::default()))?,
                            )
                            .ok_or((EditError::MalformedXml, XmlDiagnostic::default()))?;
                        attributes.push(AttributeInfo {
                            name: attr.name,
                            value: attr.value,
                            source: attr.source,
                            quote,
                        });
                        node.attribute_count += 1;
                    }
                    if !node.empty {
                        stack[depth] = nodes.len();
                        depth += 1;
                    }
                    nodes.push(node);
                }
                TokenKind::EndElement => {
                    if depth == 0 {
                        return Err((EditError::MalformedXml, XmlDiagnostic::default()));
                    }
                    depth -= 1;
                    let node: &mut NodeInfo = &mut nodes[stack[depth]];
                    node.close = token.source;
                    node.span.length = token
                        .source
                        .end()
                        .ok_or((EditError::Limit, XmlDiagnostic::default()))?
                        - node.span.offset;
                }
                TokenKind::Text | TokenKind::Cdata if depth > 0 => texts.push(TextInfo {
                    value: token.value,
                    cdata: token.kind == TokenKind::Cdata,
                }),
                _ => {}
            }
            index += 1;
        }
        if depth != 0 || nodes.len() != model.node_count() {
            return Err((EditError::MalformedXml, XmlDiagnostic::default()));
        }
        Ok(Self {
            model,
            source,
            tokens,
            nodes,
            attributes,
            texts,
        })
    }
    pub fn node(&self, handle: DocumentHandle) -> Result<NodeInfo, EditError> {
        self.model.tag_name(handle).ok_or(EditError::StaleHandle)?;
        self.nodes
            .get(handle.slot as usize)
            .copied()
            .ok_or(EditError::StaleHandle)
    }
    pub fn attribute(&self, node: NodeInfo, name: &[u8]) -> Option<AttributeInfo> {
        self.attributes
            .get(node.first_attribute..node.first_attribute + node.attribute_count)?
            .iter()
            .find(|a| a.name.slice(&self.source) == Some(name))
            .copied()
    }
}
