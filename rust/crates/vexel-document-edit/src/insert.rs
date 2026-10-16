use crate::{
    alloc,
    cache::parse_tokens,
    ops::{checked_name, encode_attribute, encode_text},
    query::decode_entities,
    *,
};
use std::borrow::Cow;
use vexel_xml::TokenKind;
pub const XML_FRAGMENT_LIMIT: usize = MAX_FRAGMENT_BYTES;
const TEMPLATE_ATTRIBUTE_LIMIT: usize = 128;
const AUTO_ID_ATTEMPTS: usize = 4096;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TemplateKind {
    Rect,
    Circle,
    Ellipse,
    Line,
    Path,
    Group,
    Text,
}
impl TemplateKind {
    fn tag(self) -> &'static [u8] {
        match self {
            Self::Rect => b"rect",
            Self::Circle => b"circle",
            Self::Ellipse => b"ellipse",
            Self::Line => b"line",
            Self::Path => b"path",
            Self::Group => b"g",
            Self::Text => b"text",
        }
    }
    pub const fn action(self) -> HistoryAction {
        match self {
            Self::Rect => HistoryAction::AddRectangle,
            Self::Circle => HistoryAction::AddCircle,
            Self::Ellipse => HistoryAction::AddEllipse,
            Self::Line => HistoryAction::AddLine,
            Self::Path => HistoryAction::AddPath,
            Self::Group => HistoryAction::AddGroup,
            Self::Text => HistoryAction::AddText,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemplateAttribute<'a> {
    pub name: &'a [u8],
    pub value: &'a [u8],
}
struct Prepared<'a> {
    bytes: Cow<'a, [u8]>,
    root_offset: usize,
    generated_id: bool,
    next_id: u64,
}
pub fn graphics_tag(tag: &[u8]) -> bool {
    let start = tag.iter().rposition(|b| *b == b':').map_or(0, |n| n + 1);
    matches!(
        &tag[start..],
        b"a" | b"circle"
            | b"ellipse"
            | b"image"
            | b"line"
            | b"path"
            | b"polygon"
            | b"polyline"
            | b"rect"
            | b"text"
            | b"use"
            | b"g"
    )
}
impl Editor {
    fn prepare_fragment<'a>(&self, fragment: &'a [u8]) -> Result<Prepared<'a>, EditError> {
        if fragment.is_empty() || fragment.len() > XML_FRAGMENT_LIMIT {
            return Err(EditError::InvalidArgument);
        }
        let mut wrapped = alloc::vector(fragment.len().checked_add(15).ok_or(EditError::Limit)?)?;
        wrapped.extend_from_slice(b"<probe>");
        wrapped.extend_from_slice(fragment);
        wrapped.extend_from_slice(b"</probe>");
        let tokens = parse_tokens(&wrapped).map_err(|(e, _)| {
            if e == EditError::OutOfMemory {
                e
            } else {
                EditError::MalformedXml
            }
        })?;
        let mut roots = tokens.iter().enumerate().filter(|(_, t)| {
            t.depth == 1 && matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
        });
        let (index, root) = roots.next().ok_or(EditError::InvalidOperation)?;
        if roots.next().is_some() {
            return Err(EditError::InvalidOperation);
        }
        let root_offset = root
            .source
            .offset
            .checked_sub(7)
            .filter(|n| *n < fragment.len())
            .ok_or(EditError::InvalidOperation)?;
        let tag = root
            .name
            .slice(&wrapped)
            .ok_or(EditError::InvalidOperation)?;
        let attrs = tokens[index + 1..]
            .iter()
            .take_while(|t| t.kind == TokenKind::Attribute);
        let id = attrs
            .into_iter()
            .find(|t| t.name.slice(&wrapped) == Some(b"id"));
        if let Some(id) = id {
            let raw = id
                .value
                .slice(&wrapped)
                .ok_or(EditError::InvalidOperation)?;
            let mut decoded = alloc::vector(raw.len().max(1))?;
            decode_entities(raw, &mut decoded)?;
            if self.id_exists(&decoded)? {
                return Err(EditError::DuplicateId);
            }
        }
        let mut prepared = Prepared {
            bytes: Cow::Borrowed(fragment),
            root_offset,
            generated_id: false,
            next_id: self.next_auto_id,
        };
        // C policy checks only the top-level graphic. Nested source and IDs are retained verbatim.
        if !graphics_tag(tag) || id.is_some() {
            return Ok(prepared);
        }
        let mut id = [0u8; 48];
        let mut id_len = 0usize;
        let mut available = false;
        for _ in 0..AUTO_ID_ATTEMPTS {
            if prepared.next_id == u64::MAX {
                return Err(EditError::Limit);
            }
            id_len = object_id(prepared.next_id, &mut id);
            prepared.next_id += 1;
            if !self.id_exists(&id[..id_len])? {
                available = true;
                break;
            }
        }
        if !available {
            return Err(EditError::Limit);
        }
        let open_end = root
            .source
            .end()
            .ok_or(EditError::Limit)?
            .checked_sub(7)
            .ok_or(EditError::InvalidOperation)?;
        let at = if root.kind == TokenKind::EmptyElement {
            open_end - 2
        } else {
            open_end - 1
        };
        let total = fragment
            .len()
            .checked_add(id_len)
            .and_then(|n| n.checked_add(6))
            .ok_or(EditError::Limit)?;
        if total > XML_FRAGMENT_LIMIT {
            return Err(EditError::Limit);
        }
        let mut bytes = alloc::vector(total)?;
        bytes.extend_from_slice(&fragment[..at]);
        bytes.extend_from_slice(b" id=\"");
        bytes.extend_from_slice(&id[..id_len]);
        bytes.push(b'"');
        bytes.extend_from_slice(&fragment[at..]);
        prepared.bytes = Cow::Owned(bytes);
        prepared.generated_id = true;
        Ok(prepared)
    }
    pub fn insert_template(
        &mut self,
        parent: DocumentHandle,
        kind: TemplateKind,
        attributes: &[TemplateAttribute<'_>],
    ) -> Result<DocumentHandle, EditError> {
        if attributes.len() > TEMPLATE_ATTRIBUTE_LIMIT {
            return Err(EditError::InvalidArgument);
        }
        let info = self.handle_index(parent)?.node(parent)?;
        let mut markup = alloc::vector(XML_FRAGMENT_LIMIT)?;
        markup.push(b'<');
        markup.extend_from_slice(kind.tag());
        append_attributes(&mut markup, attributes)?;
        bounded_append(&mut markup, b"/>")?;
        let prepared = self.prepare_fragment(&markup)?;
        self.insert_prepared(parent, info, prepared, kind.action(), true)
    }
    pub fn insert_xml_fragment(
        &mut self,
        parent: DocumentHandle,
        fragment: &[u8],
    ) -> Result<DocumentHandle, EditError> {
        if fragment.is_empty() {
            return Err(EditError::InvalidArgument);
        }
        if fragment.len() > XML_FRAGMENT_LIMIT {
            return Err(EditError::Limit);
        }
        let info = self.handle_index(parent)?.node(parent)?;
        let prepared = self.prepare_fragment(fragment)?;
        self.insert_prepared(parent, info, prepared, HistoryAction::AddShape, false)
    }
    pub fn insert_text_element(
        &mut self,
        parent: DocumentHandle,
        attributes: &[TemplateAttribute<'_>],
        text: &[u8],
    ) -> Result<DocumentHandle, EditError> {
        if attributes.len() > TEMPLATE_ATTRIBUTE_LIMIT {
            return Err(EditError::Limit);
        }
        let mut markup = alloc::vector(XML_FRAGMENT_LIMIT)?;
        markup.extend_from_slice(b"<text");
        append_attributes(&mut markup, attributes)?;
        // Match bounded text_fragment_append before committing one command.
        bounded_append(&mut markup, b">")?;
        let encoded = encode_text(text)?;
        bounded_append(&mut markup, &encoded)?;
        bounded_append(&mut markup, b"</text>")?;
        let info = self.handle_index(parent)?.node(parent)?;
        let prepared = self.prepare_fragment(&markup)?;
        self.insert_prepared(parent, info, prepared, HistoryAction::AddText, false)
    }
    fn insert_prepared(
        &mut self,
        parent: DocumentHandle,
        info: crate::cache::NodeInfo,
        prepared: Prepared<'_>,
        action: HistoryAction,
        template: bool,
    ) -> Result<DocumentHandle, EditError> {
        let (mut offset, mut remove) = (info.close.offset, 0usize);
        let mut replacement = Cow::Borrowed(prepared.bytes.as_ref());
        if info.empty {
            let cache = self.index()?;
            let name = info
                .name
                .slice(&cache.source)
                .ok_or(EditError::StaleHandle)?;
            let total = prepared
                .bytes
                .len()
                .checked_add(name.len())
                .and_then(|n| n.checked_add(4))
                .ok_or(EditError::Limit)?;
            if total
                > if template {
                    XML_FRAGMENT_LIMIT
                } else {
                    XML_FRAGMENT_LIMIT + 4096
                }
            {
                return Err(EditError::Limit);
            }
            let mut bytes = alloc::vector(total)?;
            bytes.push(b'>');
            bytes.extend_from_slice(&prepared.bytes);
            bytes.extend_from_slice(b"</");
            bytes.extend_from_slice(name);
            bytes.push(b'>');
            replacement = Cow::Owned(bytes);
            offset = info.open.offset + info.open.length - 2;
            remove = 2;
        }
        let returned = offset
            .checked_add(usize::from(info.empty))
            .and_then(|n| n.checked_add(prepared.root_offset))
            .ok_or(EditError::Limit)?;
        let handle = self.apply_returning(
            info.span,
            offset,
            remove,
            &replacement,
            false,
            action.label(),
            parent.slot,
            returned,
        )?;
        if prepared.generated_id {
            self.next_auto_id = prepared.next_id;
        }
        Ok(handle)
    }
}
fn bounded_append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), EditError> {
    if out.len() > XML_FRAGMENT_LIMIT || bytes.len() > XML_FRAGMENT_LIMIT - out.len() {
        return Err(EditError::Limit);
    }
    out.extend_from_slice(bytes);
    Ok(())
}
fn append_attributes(
    out: &mut Vec<u8>,
    attributes: &[TemplateAttribute<'_>],
) -> Result<(), EditError> {
    for (i, attr) in attributes.iter().enumerate() {
        checked_name(attr.name)?;
        if attributes[..i].iter().any(|a| a.name == attr.name) {
            return Err(EditError::DuplicateAttribute);
        }
        let encoded = encode_attribute(attr.value, b'"')?;
        let addition = attr
            .name
            .len()
            .checked_add(encoded.len())
            .and_then(|n| n.checked_add(4))
            .ok_or(EditError::Limit)?;
        if addition > XML_FRAGMENT_LIMIT - out.len() {
            return Err(EditError::Limit);
        }
        bounded_append(out, b" ")?;
        bounded_append(out, attr.name)?;
        bounded_append(out, b"=\"")?;
        bounded_append(out, &encoded)?;
        bounded_append(out, b"\"")?;
    }
    Ok(())
}
fn object_id(value: u64, output: &mut [u8; 48]) -> usize {
    let prefix = b"vexel-object-";
    output[..prefix.len()].copy_from_slice(prefix);
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    let mut value = value;
    loop {
        digits[count] = b'0' + (value % 10) as u8;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for i in 0..count {
        output[prefix.len() + i] = digits[count - 1 - i];
    }
    prefix.len() + count
}
