use crate::{EditError, Editor, alloc, cache::Cache};
use vexel_document::{DocumentHandle, DocumentSourceSpan};
pub type SourceSpan = DocumentSourceSpan;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attribute {
    pub name: SourceSpan,
    pub value: SourceSpan,
    pub quote: u8,
}
impl Editor {
    pub fn root(&self) -> Option<DocumentHandle> {
        self.index().ok()?.model.root()
    }
    pub fn find_id(&self, id: &[u8]) -> Option<DocumentHandle> {
        self.try_find_id(id).ok().flatten()
    }
    pub fn try_find_id(&self, id: &[u8]) -> Result<Option<DocumentHandle>, EditError> {
        self.lookup_id(id, false)
    }
    pub(crate) fn id_exists(&self, id: &[u8]) -> Result<bool, EditError> {
        self.lookup_id(id, true).map(|h| h.is_some())
    }
    fn lookup_id(&self, id: &[u8], raw_fast: bool) -> Result<Option<DocumentHandle>, EditError> {
        let cache = self.index()?;
        for (slot, node) in cache.nodes.iter().enumerate() {
            for attr in
                &cache.attributes[node.first_attribute..node.first_attribute + node.attribute_count]
            {
                if attr.name.slice(&cache.source) != Some(b"id") {
                    continue;
                }
                let raw = attr
                    .value
                    .slice(&cache.source)
                    .ok_or(EditError::InvalidOperation)?;
                if raw_fast && raw == id {
                    return Ok(cache.model.node_at(slot));
                }
                if !raw.contains(&b'&') {
                    if raw == id {
                        return Ok(cache.model.node_at(slot));
                    }
                    continue;
                }
                let mut decoded = alloc::vector(raw.len().max(1))?;
                decode_entities(raw, &mut decoded)?;
                if decoded == id {
                    return Ok(cache.model.node_at(slot));
                }
            }
        }
        Ok(None)
    }
    pub fn element_at_offset(&self, offset: usize) -> Option<DocumentHandle> {
        let cache = self.index().ok()?;
        if offset >= cache.source.len() {
            return None;
        }
        crate::ops::handle_at_offset(cache, offset)
    }
    pub fn tag_name(&self, handle: DocumentHandle) -> Option<&[u8]> {
        self.index().ok()?.model.tag_name(handle)
    }
    pub fn source_span(&self, handle: DocumentHandle) -> Option<SourceSpan> {
        self.index().ok()?.model.source_span(handle)
    }
    pub fn attribute_count(&self, handle: DocumentHandle) -> usize {
        self.index().map_or(0, |c| c.model.attribute_count(handle))
    }
    pub fn attribute_at(&self, handle: DocumentHandle, index: usize) -> Option<Attribute> {
        let cache = self.handle_index(handle).ok()?;
        let node = cache.node(handle).ok()?;
        if index >= node.attribute_count {
            return None;
        }
        let a = cache.attributes.get(node.first_attribute + index)?;
        Some(Attribute {
            name: SourceSpan {
                offset: a.name.offset,
                length: a.name.length,
            },
            value: SourceSpan {
                offset: a.value.offset,
                length: a.value.length,
            },
            quote: a.quote,
        })
    }
    pub fn span_bytes(&self, span: SourceSpan) -> Option<&[u8]> {
        self.index()
            .ok()?
            .source
            .get(span.offset..span.offset.checked_add(span.length)?)
    }
    pub fn span_equals(&self, span: SourceSpan, bytes: &[u8]) -> bool {
        self.span_bytes(span) == Some(bytes)
    }
    pub fn materialize_span(
        &self,
        span: SourceSpan,
        output: &mut [u8],
    ) -> Result<usize, EditError> {
        let bytes = self.span_bytes(span).ok_or(EditError::InvalidArgument)?;
        if output.len() < bytes.len() {
            return Err(EditError::InvalidArgument);
        }
        output[..bytes.len()].copy_from_slice(bytes);
        Ok(bytes.len())
    }
    pub fn decoded_span(&self, span: SourceSpan, output: &mut [u8]) -> Result<usize, EditError> {
        decode_bytes(
            self.span_bytes(span).ok_or(EditError::InvalidArgument)?,
            output,
        )
    }
    pub fn decode_span_into(
        &self,
        span: SourceSpan,
        output: &mut [u8],
        out_length: &mut usize,
    ) -> bool {
        let Some(raw) = self.span_bytes(span) else {
            return false;
        };
        let Ok(n) = visit_decoded(raw, &mut |_| Ok(())) else {
            return false;
        };
        *out_length = n;
        decode_bytes(raw, output).is_ok()
    }
    pub fn text_content(
        &self,
        handle: DocumentHandle,
        output: &mut [u8],
    ) -> Result<usize, EditError> {
        let cache = self.handle_index(handle)?;
        let n = visit_text(cache, handle, &mut |_| Ok(()))?;
        if output.len() < n {
            return Err(EditError::InvalidArgument);
        }
        let mut at = 0usize;
        visit_text(cache, handle, &mut |bytes| {
            output[at..at + bytes.len()].copy_from_slice(bytes);
            at += bytes.len();
            Ok(())
        })?;
        Ok(n)
    }
    pub fn text_content_into(
        &self,
        handle: DocumentHandle,
        output: &mut [u8],
        out_length: &mut usize,
    ) -> bool {
        let Ok(cache) = self.handle_index(handle) else {
            return false;
        };
        let Ok(n) = visit_text(cache, handle, &mut |_| Ok(())) else {
            return false;
        };
        *out_length = n;
        self.text_content(handle, output).is_ok()
    }
    pub fn text_bytes(&self, handle: DocumentHandle) -> Result<Vec<u8>, EditError> {
        let cache = self.handle_index(handle)?;
        let n = visit_text(cache, handle, &mut |_| Ok(()))?;
        let mut out = alloc::vector(n)?;
        out.resize(n, 0);
        self.text_content(handle, &mut out)?;
        Ok(out)
    }
    pub fn element_path(
        &self,
        handle: DocumentHandle,
        output: &mut [u8],
    ) -> Result<usize, EditError> {
        let cache = self.index()?;
        let n = visit_path(cache, handle, &mut |_| Ok(()))?;
        if output.len() < n {
            return Err(EditError::InvalidArgument);
        }
        let mut at = 0usize;
        visit_path(cache, handle, &mut |bytes| {
            output[at..at + bytes.len()].copy_from_slice(bytes);
            at += bytes.len();
            Ok(())
        })?;
        Ok(n)
    }
    /// Two passes report the complete required length and avoid partial prefix writes.
    pub fn element_path_into(
        &self,
        handle: DocumentHandle,
        output: &mut [u8],
        out_length: &mut usize,
    ) -> bool {
        let Ok(cache) = self.index() else {
            return false;
        };
        let Ok(n) = visit_path(cache, handle, &mut |_| Ok(())) else {
            return false;
        };
        *out_length = n;
        self.element_path(handle, output).is_ok()
    }
    pub fn path_bytes(&self, handle: DocumentHandle) -> Result<Vec<u8>, EditError> {
        let cache = self.index()?;
        let n = visit_path(cache, handle, &mut |_| Ok(()))?;
        let mut out = alloc::vector(n)?;
        out.resize(n, 0);
        self.element_path(handle, &mut out)?;
        Ok(out)
    }
}
fn emit(
    bytes: &[u8],
    length: &mut usize,
    visitor: &mut impl FnMut(&[u8]) -> Result<(), EditError>,
) -> Result<(), EditError> {
    *length = length.checked_add(bytes.len()).ok_or(EditError::Limit)?;
    visitor(bytes)
}
fn visit_text(
    cache: &Cache,
    handle: DocumentHandle,
    visitor: &mut impl FnMut(&[u8]) -> Result<(), EditError>,
) -> Result<usize, EditError> {
    let node = cache.node(handle)?;
    let end = node
        .span
        .offset
        .checked_add(node.span.length)
        .ok_or(EditError::Limit)?;
    let mut length = 0usize;
    for text in &cache.texts {
        if text.value.offset < node.span.offset
            || text.value.offset >= end
            || text.value.length > end - text.value.offset
        {
            continue;
        }
        let raw = text
            .value
            .slice(&cache.source)
            .ok_or(EditError::InvalidOperation)?;
        if text.cdata {
            emit(raw, &mut length, visitor)?;
        } else {
            visit_decoded(raw, &mut |bytes| emit(bytes, &mut length, visitor))?;
        }
    }
    Ok(length)
}
fn visit_path(
    cache: &Cache,
    handle: DocumentHandle,
    visitor: &mut impl FnMut(&[u8]) -> Result<(), EditError>,
) -> Result<usize, EditError> {
    let mut chain = [handle; 256];
    let mut count = 0usize;
    let mut current = handle;
    loop {
        cache.node(current)?;
        if count >= chain.len() {
            return Err(EditError::InvalidOperation);
        }
        chain[count] = current;
        count += 1;
        let Some(parent) = cache.model.parent(current) else {
            break;
        };
        current = parent;
    }
    if count == 256 {
        return Err(EditError::InvalidOperation);
    }
    let mut length = 0usize;
    for node in chain[..count].iter().rev() {
        let name = cache.model.tag_name(*node).ok_or(EditError::StaleHandle)?;
        let mut ordinal = 1usize;
        if let Some(parent) = cache.model.parent(*node) {
            let mut sibling = cache.model.first_child(parent);
            while let Some(h) = sibling {
                if h == *node {
                    break;
                }
                if cache.model.tag_name(h) == Some(name) {
                    ordinal += 1;
                }
                sibling = cache.model.next_sibling(h);
            }
        }
        emit(b"/", &mut length, visitor)?;
        emit(name, &mut length, visitor)?;
        emit(b"[", &mut length, visitor)?;
        let mut digits = [0u8; 24];
        let mut n = 0usize;
        loop {
            digits[n] = b'0' + (ordinal % 10) as u8;
            n += 1;
            ordinal /= 10;
            if ordinal == 0 {
                break;
            }
        }
        for digit in digits[..n].iter().rev() {
            emit(&[*digit], &mut length, visitor)?;
        }
        emit(b"]", &mut length, visitor)?;
    }
    Ok(length)
}
pub fn decode_bytes(raw: &[u8], output: &mut [u8]) -> Result<usize, EditError> {
    let n = visit_decoded(raw, &mut |_| Ok(()))?;
    if output.len() < n {
        return Err(EditError::InvalidArgument);
    }
    let mut at = 0usize;
    visit_decoded(raw, &mut |bytes| {
        output[at..at + bytes.len()].copy_from_slice(bytes);
        at += bytes.len();
        Ok(())
    })?;
    Ok(n)
}
pub(crate) fn decode_entities(raw: &[u8], out: &mut Vec<u8>) -> Result<(), EditError> {
    visit_decoded(raw, &mut |bytes| alloc::append(out, bytes)).map(|_| ())
}
fn visit_decoded(
    raw: &[u8],
    visitor: &mut impl FnMut(&[u8]) -> Result<(), EditError>,
) -> Result<usize, EditError> {
    let mut at = 0usize;
    let mut length = 0usize;
    while at < raw.len() {
        if raw[at] != b'&' {
            let start = at;
            while at < raw.len() && raw[at] != b'&' {
                at += 1;
            }
            emit(&raw[start..at], &mut length, visitor)?;
            continue;
        }
        let start = at + 1;
        let end = start
            + raw[start..]
                .iter()
                .position(|b| *b == b';')
                .ok_or(EditError::MalformedXml)?;
        let entity = &raw[start..end];
        let bytes: &[u8] = match entity {
            b"amp" => b"&",
            b"lt" => b"<",
            b"gt" => b">",
            b"quot" => b"\"",
            b"apos" => b"'",
            value if value.starts_with(b"#") => {
                let (radix, digits) = if matches!(value.get(1), Some(b'x' | b'X')) {
                    (16, &value[2..])
                } else {
                    (10, &value[1..])
                };
                if digits.is_empty() {
                    return Err(EditError::MalformedXml);
                }
                let mut cp = 0u32;
                for &digit in digits {
                    let value = match digit {
                        b'0'..=b'9' => (digit - b'0') as u32,
                        b'a'..=b'f' if radix == 16 => (digit - b'a' + 10) as u32,
                        b'A'..=b'F' if radix == 16 => (digit - b'A' + 10) as u32,
                        _ => return Err(EditError::MalformedXml),
                    };
                    if value >= radix {
                        return Err(EditError::MalformedXml);
                    }
                    cp = cp
                        .checked_mul(radix)
                        .and_then(|n| n.checked_add(value))
                        .ok_or(EditError::MalformedXml)?;
                }
                let mut buffer = [0; 4];
                let n = encode_codepoint(cp, &mut buffer);
                emit(&buffer[..n], &mut length, visitor)?;
                at = end + 1;
                continue;
            }
            _ => return Err(EditError::MalformedXml),
        };
        emit(bytes, &mut length, visitor)?;
        at = end + 1;
    }
    Ok(length)
}

// Raw decoder parity deliberately does not revalidate XML Char: the tokenizer
// validates document references. As in C, external raw helpers return bytes.
fn encode_codepoint(cp: u32, out: &mut [u8; 4]) -> usize {
    if cp <= 0x7f {
        out[0] = cp as u8;
        1
    } else if cp <= 0x7ff {
        out[0] = (0xc0 | (cp >> 6)) as u8;
        out[1] = (0x80 | (cp & 0x3f)) as u8;
        2
    } else if cp <= 0xffff {
        out[0] = (0xe0 | (cp >> 12)) as u8;
        out[1] = (0x80 | ((cp >> 6) & 0x3f)) as u8;
        out[2] = (0x80 | (cp & 0x3f)) as u8;
        3
    } else {
        out[0] = (0xf0 | (cp >> 18)) as u8;
        out[1] = (0x80 | ((cp >> 12) & 0x3f)) as u8;
        out[2] = (0x80 | ((cp >> 6) & 0x3f)) as u8;
        out[3] = (0x80 | (cp & 0x3f)) as u8;
        4
    }
}
