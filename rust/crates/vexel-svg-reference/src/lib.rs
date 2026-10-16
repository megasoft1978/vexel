#![forbid(unsafe_code)]
//! Source-preserving SVG reference lookup over an accepted XML token stream.

use vexel_base::{ControlledError, ControlledStep, Stop, WorkControl, WorkKind};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

mod build;
mod find;
pub use build::ReferenceBuildSession;
pub use find::ReferenceFindSession;

pub const MAX_REFERENCE_IDS: usize = 65_536;
pub const REFERENCE_SLOT_CAPACITY: usize = 131_072;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceError {
    None,
    InvalidArgument,
    Unsupported,
    Limit,
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceDiagnostic {
    pub category: ReferenceError,
    pub offset: usize,
}

impl ReferenceDiagnostic {
    const fn new(category: ReferenceError, offset: usize) -> Self {
        Self { category, offset }
    }
}

pub type ReferenceResult<T> = Result<T, ReferenceDiagnostic>;

/// An ID table that borrows its immutable source and tokens. IDs are compared
/// as their exact source bytes; entities are neither decoded nor rewritten.
#[derive(Debug)]
pub struct ReferenceIndex<'a> {
    source: &'a [u8],
    tokens: &'a [XmlToken],
    slots: Vec<u32>,
    id_count: usize,
    lookup_probes: usize,
}

impl<'a> ReferenceIndex<'a> {
    /// Creates the fixed-capacity index. Empty documents allocate no table.
    pub fn create(source: &'a [u8], tokens: &'a [XmlToken]) -> ReferenceResult<Self> {
        let mut control = NoStop;
        let mut session = ReferenceBuildSession::new(source, tokens);
        loop {
            match session.step(256, &mut control) {
                ControlledStep::Pending { .. } => {}
                ControlledStep::Complete(()) => return session.finish().map_err(original_error),
                ControlledStep::Failed(error) => return Err(controlled_original(error)),
            }
        }
    }

    pub fn create_controlled(
        source: &'a [u8],
        tokens: &'a [XmlToken],
        control: &mut dyn WorkControl,
    ) -> Result<Self, ControlledError<ReferenceDiagnostic>> {
        let mut session = ReferenceBuildSession::new(source, tokens);
        loop {
            match session.step(256, control) {
                ControlledStep::Pending { .. } => {}
                ControlledStep::Complete(()) => return session.finish(),
                ControlledStep::Failed(error) => return Err(error),
            }
        }
    }

    /// Finds an exact raw ID and counts each occupied or terminating slot probe.
    pub fn find(&mut self, id: XmlSpan) -> Option<usize> {
        let mut session = self.begin_find(id);
        let mut control = NoStop;
        loop {
            match session.step(256, &mut control) {
                ControlledStep::Pending { .. } => {}
                ControlledStep::Complete(found) => return found,
                ControlledStep::Failed(_) => return None,
            }
        }
    }

    pub fn find_controlled(
        &mut self,
        id: XmlSpan,
        control: &mut dyn WorkControl,
    ) -> Result<Option<usize>, ControlledError<ReferenceDiagnostic>> {
        let mut session = self.begin_find(id);
        loop {
            match session.step(256, control) {
                ControlledStep::Pending { .. } => {}
                ControlledStep::Complete(found) => return Ok(found),
                ControlledStep::Failed(error) => return Err(error),
            }
        }
    }

    pub fn begin_find(&mut self, id: XmlSpan) -> ReferenceFindSession<'_, 'a> {
        ReferenceFindSession::new(self, id)
    }

    pub fn id_count(&self) -> usize {
        self.id_count
    }

    /// Bytes in the actual allocated table, zero for an empty index.
    pub fn storage_bytes(&self) -> usize {
        self.slots
            .capacity()
            .checked_mul(core::mem::size_of::<u32>())
            .unwrap_or(0)
    }

    pub fn lookup_probes(&self) -> usize {
        self.lookup_probes
    }

    pub fn allocated_bytes(&self) -> usize {
        self.slots
            .capacity()
            .checked_mul(core::mem::size_of::<u32>())
            .unwrap_or(0)
    }
}

fn original_error(error: ControlledError<ReferenceDiagnostic>) -> ReferenceDiagnostic {
    match error {
        ControlledError::Original(d) => d,
        _ => ReferenceDiagnostic::new(ReferenceError::OutOfMemory, 0),
    }
}

fn controlled_original(error: ControlledError<ReferenceDiagnostic>) -> ReferenceDiagnostic {
    original_error(error)
}

struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

/// Finds the matching close token for a supported, nonempty definition node.
pub fn definition_end(source: &[u8], tokens: &[XmlToken], start: usize) -> Option<usize> {
    validate_tokens(source, tokens).ok()?;
    let token = tokens.get(start)?;
    if !is_element(token.kind)
        || token.prefix.length != 0
        || !one_of(
            span_bytes(source, token.local_name)?,
            &[b"defs", b"symbol", b"marker", b"clipPath"],
        )
    {
        return None;
    }
    if token.kind == TokenKind::EmptyElement {
        return Some(start);
    }
    tokens
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(i, close)| {
            (close.kind == TokenKind::EndElement && close.depth == token.depth).then_some(i)
        })
}

pub fn attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    root: bool,
    kind: XmlSpan,
) -> bool {
    if validate_tokens(source, tokens).is_err() {
        return false;
    }
    attributes_supported_inner(source, tokens, element, root, kind)
}

pub fn xlink_is_bound(source: &[u8], tokens: &[XmlToken]) -> bool {
    if validate_tokens(source, tokens).is_err() {
        return false;
    }
    for (i, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::StartElement || token.depth != 0 {
            continue;
        }
        for attr in attributes(tokens, i) {
            if span_eq(source, attr.name, b"xmlns:xlink")
                && span_eq(source, attr.value, b"http://www.w3.org/1999/xlink")
            {
                return true;
            }
        }
        return false;
    }
    false
}

pub fn use_attributes_supported(source: &[u8], tokens: &[XmlToken], element: usize) -> bool {
    if validate_tokens(source, tokens).is_err() {
        return false;
    }
    for attr in attributes(tokens, element) {
        let name = attr.local_name;
        if attr.prefix.length != 0 {
            if !span_eq(source, attr.prefix, b"xlink")
                || !span_eq(source, name, b"href")
                || !xlink_is_bound(source, tokens)
            {
                return false;
            }
            continue;
        }
        if !one_of(span_bytes(source, name).unwrap_or_default(), USE_ALLOWED) {
            return false;
        }
    }
    tokens.get(element).is_some_and(|t| is_element(t.kind))
}

const COMMON: &[&[u8]] = &[
    b"fill",
    b"fill-rule",
    b"transform",
    b"clip-path",
    b"filter",
    b"mask",
    b"mix-blend-mode",
    b"isolation",
    b"vector-effect",
    b"shape-rendering",
    b"paint-order",
    b"marker",
    b"marker-start",
    b"marker-mid",
    b"marker-end",
    b"mask-type",
    b"color-interpolation-filters",
    b"systemLanguage",
    b"requiredFeatures",
    b"requiredExtensions",
    b"fill-opacity",
    b"stroke-opacity",
    b"opacity",
    b"visibility",
    b"display",
    b"color",
    b"stroke",
    b"stroke-width",
    b"stroke-linecap",
    b"stroke-linejoin",
    b"stroke-miterlimit",
    b"stroke-dasharray",
    b"stroke-dashoffset",
    b"font-family",
    b"font-size",
    b"font-weight",
    b"font-style",
    b"text-anchor",
    b"letter-spacing",
    b"word-spacing",
];
const META: &[&[u8]] = &[b"id", b"class", b"style"];
const USE_ALLOWED: &[&[u8]] = &[
    b"href",
    b"id",
    b"class",
    b"style",
    b"x",
    b"y",
    b"width",
    b"height",
    b"preserveAspectRatio",
    b"fill",
    b"fill-rule",
    b"fill-opacity",
    b"stroke-opacity",
    b"opacity",
    b"visibility",
    b"display",
    b"color",
    b"transform",
    b"clip-path",
    b"filter",
    b"mask",
    b"mix-blend-mode",
    b"isolation",
    b"vector-effect",
    b"shape-rendering",
    b"paint-order",
    b"marker",
    b"marker-start",
    b"marker-mid",
    b"marker-end",
    b"mask-type",
    b"color-interpolation-filters",
    b"stroke",
    b"stroke-width",
    b"stroke-linecap",
    b"stroke-linejoin",
    b"stroke-miterlimit",
    b"stroke-dasharray",
    b"stroke-dashoffset",
    b"font-family",
    b"font-size",
    b"font-weight",
    b"font-style",
    b"text-anchor",
    b"letter-spacing",
    b"word-spacing",
];

fn attributes_supported_inner(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    root: bool,
    kind: XmlSpan,
) -> bool {
    let Some(element_token) = tokens.get(element) else {
        return false;
    };
    if !is_element(element_token.kind) || span_bytes(source, kind).is_none() {
        return false;
    }
    for attr in attributes(tokens, element) {
        let n = attr.local_name;
        let namespace_declaration = root && span_eq(source, attr.prefix, b"xmlns");
        let xlink_href = span_eq(source, attr.prefix, b"xlink") && span_eq(source, n, b"href");
        let common = matches_any(source, n, COMMON);
        let shape = shape_attribute(source, n, kind);
        let xml_space = span_eq(source, attr.prefix, b"xml")
            && span_eq(source, n, b"space")
            && one_of(
                span_bytes(source, kind).unwrap_or_default(),
                &[b"text", b"tspan"],
            );
        if attr.prefix.length != 0
            && !namespace_declaration
            && !xml_space
            && !(xlink_href
                && one_of(
                    span_bytes(source, kind).unwrap_or_default(),
                    &[b"image", b"a"],
                ))
        {
            if common || shape || xlink_href {
                return false;
            }
            continue;
        }
        let k = span_bytes(source, kind).unwrap_or_default();
        let metadata = matches_any(source, n, META);
        let ok = if root {
            metadata
                || common
                || span_eq(source, n, b"xmlns")
                || namespace_declaration
                || one_of(
                    span_bytes(source, n).unwrap_or_default(),
                    &[
                        b"version",
                        b"width",
                        b"height",
                        b"viewBox",
                        b"preserveAspectRatio",
                    ],
                )
        } else if k == b"g" || k == b"switch" {
            metadata || common
        } else if k == b"a" {
            metadata || common || span_eq(source, n, b"href")
        } else if k == b"svg" {
            metadata
                || common
                || one_of(
                    span_bytes(source, n).unwrap_or_default(),
                    &[
                        b"x",
                        b"y",
                        b"width",
                        b"height",
                        b"viewBox",
                        b"preserveAspectRatio",
                    ],
                )
        } else {
            metadata || common || shape || xml_space
        };
        if !ok {
            return false;
        }
    }
    true
}

fn shape_attribute(source: &[u8], name: XmlSpan, kind: XmlSpan) -> bool {
    let (n, k) = (
        span_bytes(source, name).unwrap_or_default(),
        span_bytes(source, kind).unwrap_or_default(),
    );
    if k == b"rect" {
        one_of(n, &[b"x", b"y", b"width", b"height", b"rx", b"ry"])
    } else if k == b"path" {
        n == b"d"
    } else if k == b"circle" {
        one_of(n, &[b"cx", b"cy", b"r"])
    } else if k == b"ellipse" {
        one_of(n, &[b"cx", b"cy", b"rx", b"ry"])
    } else if k == b"line" {
        one_of(n, &[b"x1", b"y1", b"x2", b"y2"])
    } else if k == b"polygon" || k == b"polyline" {
        n == b"points"
    } else if k == b"image" {
        one_of(
            n,
            &[
                b"x",
                b"y",
                b"width",
                b"height",
                b"href",
                b"preserveAspectRatio",
            ],
        )
    } else if k == b"text" || k == b"tspan" {
        one_of(n, &[b"x", b"y", b"dx", b"dy"])
    } else {
        false
    }
}

fn validate_tokens(source: &[u8], tokens: &[XmlToken]) -> ReferenceResult<()> {
    for token in tokens {
        for span in [
            token.source,
            token.name,
            token.value,
            token.prefix,
            token.local_name,
        ] {
            if span_bytes(source, span).is_none() {
                return Err(ReferenceDiagnostic::new(
                    ReferenceError::InvalidArgument,
                    span.offset,
                ));
            }
        }
    }
    Ok(())
}

fn attributes(tokens: &[XmlToken], element: usize) -> impl Iterator<Item = &XmlToken> {
    let begin = element.saturating_add(1);
    tokens
        .get(begin..)
        .unwrap_or_default()
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
}

// Retained reference helpers document the pre-control lookup/hash arithmetic.
#[allow(dead_code)]
fn attr_value(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    attributes(tokens, element)
        .find(|attr| span_eq(source, attr.local_name, name) && attr.prefix.length == 0)
        .map(|attr| attr.value)
}

fn is_element(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::StartElement | TokenKind::EmptyElement)
}
fn span_bytes(source: &[u8], span: XmlSpan) -> Option<&[u8]> {
    let end = span.offset.checked_add(span.length)?;
    source.get(span.offset..end)
}
fn span_eq(source: &[u8], span: XmlSpan, bytes: &[u8]) -> bool {
    span_bytes(source, span) == Some(bytes)
}
fn one_of(bytes: &[u8], choices: &[&[u8]]) -> bool {
    choices.contains(&bytes)
}
fn matches_any(source: &[u8], span: XmlSpan, choices: &[&[u8]]) -> bool {
    span_bytes(source, span).is_some_and(|bytes| one_of(bytes, choices))
}
#[allow(dead_code)]
fn spans_equal(source: &[u8], left: XmlSpan, right: XmlSpan) -> ReferenceResult<bool> {
    let left = span_bytes(source, left)
        .ok_or_else(|| ReferenceDiagnostic::new(ReferenceError::InvalidArgument, left.offset))?;
    let right = span_bytes(source, right)
        .ok_or_else(|| ReferenceDiagnostic::new(ReferenceError::InvalidArgument, right.offset))?;
    Ok(left == right)
}
#[allow(dead_code)]
fn span_hash(source: &[u8], span: XmlSpan) -> ReferenceResult<u64> {
    Ok(hash_bytes(span_bytes(source, span).ok_or_else(|| {
        ReferenceDiagnostic::new(ReferenceError::InvalidArgument, span.offset)
    })?))
}
#[allow(dead_code)]
fn hash_bytes(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(1_469_598_103_934_665_603u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(1_099_511_628_211)
        })
}

fn retain_exact_slot_candidate(candidate: Vec<u32>) -> Option<Vec<u32>> {
    (candidate.capacity() == REFERENCE_SLOT_CAPACITY).then_some(candidate)
}

#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn oversized_successful_reservation_is_rejected() {
        let mut candidate = Vec::new();
        candidate.try_reserve_exact(REFERENCE_SLOT_CAPACITY + 1).unwrap();
        assert!(candidate.capacity() > REFERENCE_SLOT_CAPACITY);
        assert!(retain_exact_slot_candidate(candidate).is_none());
    }
}
