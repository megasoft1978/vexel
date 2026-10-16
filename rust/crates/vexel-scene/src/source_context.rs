//! One canonical source/token owner for ephemeral CSS/reference state. Lower
//! modules query it through methods instead of replacing its borrowed tables.
use crate::shape_geometry::GeometryProof;
use crate::types::{invalid, stopped};
use crate::*;
use vexel_base::ControlledError;
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

/// Source-backed proof that a validated `<path d="…">` supplied geometry.
/// Its private fields prevent callers from pairing a token index with arbitrary
/// bytes or an unrelated attribute span.
#[derive(Debug, Clone, Copy)]
pub struct CanonicalPathSource<'src> {
    source: &'src SourceOwner,
    pub(crate) proof: GeometryProof,
}
impl CanonicalPathSource<'_> {
    pub(crate) fn belongs_to(&self, source: &SourceOwner) -> bool {
        std::sync::Arc::ptr_eq(self.source, source)
    }
}
pub struct DecodeSource<'src> {
    source: &'src SourceOwner,
    tokens: &'src [XmlToken],
    css: vexel_svg_css::CssSheet<'src>,
    references: vexel_svg_reference::ReferenceIndex<'src>,
}
impl<'src> DecodeSource<'src> {
    pub fn new(document: &'src LexicalDocument, control: &mut dyn WorkControl) -> Result<Self> {
        control.checkpoint().map_err(stopped)?;
        let source = document.source_owner();
        let tokens = document.tokens();
        let css = vexel_svg_css::CssSheet::create_controlled(source, tokens, control)
            .map_err(css_error)?;
        let references =
            vexel_svg_reference::ReferenceIndex::create_controlled(source, tokens, control)
                .map_err(reference_error)?;
        Ok(Self {
            source,
            tokens,
            css,
            references,
        })
    }
    pub fn source_owner(&self) -> &'src SourceOwner {
        self.source
    }
    pub fn source(&self) -> &'src [u8] {
        self.source
    }
    pub fn tokens(&self) -> &'src [XmlToken] {
        self.tokens
    }
    pub fn element(
        &self,
        id: ElementId,
        end: usize,
        control: &mut dyn WorkControl,
    ) -> Result<ElementView<'src>> {
        let view = ElementView {
            source: self.source,
            tokens: self.tokens,
            element: id,
            end,
        };
        view.validate(control)?;
        Ok(view)
    }
    /// Mint canonical path provenance from this exact lexical document. The
    /// direct unprefixed `d` attribute and full element range are retained so
    /// later scene validation can distinguish referenced geometry from its
    /// independent use-host SourceMap annotation.
    pub fn canonical_path_source(
        &self,
        element: ElementView<'src>,
        control: &mut dyn WorkControl,
    ) -> Result<CanonicalPathSource<'src>> {
        element.validate(control)?;
        if element.source.as_ptr() != self.source.as_slice().as_ptr()
            || element.source.len() != self.source.len()
            || !std::ptr::eq(element.tokens.as_ptr(), self.tokens.as_ptr())
            || element.tokens.len() != self.tokens.len()
        {
            return Err(invalid(0));
        }
        let first = self
            .tokens
            .get(element.element.0)
            .ok_or_else(|| invalid(0))?;
        if !first
            .local_name
            .slice(self.source)
            .is_some_and(|v| v == b"path")
            || !matches!(
                first.kind,
                TokenKind::StartElement | TokenKind::EmptyElement
            )
        {
            return Err(invalid(first.name.offset));
        }
        let mut path_data = None;
        for attribute in self.tokens.iter().skip(element.element.0 + 1) {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            if attribute.kind != TokenKind::Attribute {
                break;
            }
            if attribute.name.slice(self.source).is_some_and(|v| v == b"d") {
                path_data = Some(attribute.value);
                break;
            }
        }
        let path_data = path_data.ok_or_else(|| invalid(first.name.offset))?;
        let path_end = add(path_data.offset, path_data.length)?;
        let element_end = if first.kind == TokenKind::EmptyElement {
            add(first.source.offset, first.source.length)?
        } else {
            let mut close = None;
            for token in self
                .tokens
                .iter()
                .take(element.end)
                .skip(element.element.0 + 1)
            {
                control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
                if token.kind == TokenKind::EndElement && token.depth == first.depth {
                    close = Some(add(token.source.offset, token.source.length)?);
                    break;
                }
            }
            close.ok_or_else(|| invalid(first.name.offset))?
        };
        let element_range = Span {
            start: first.source.offset,
            end: element_end,
        };
        let path_data = Span {
            start: path_data.offset,
            end: path_end,
        };
        if path_data.start < element_range.start
            || path_data.end > element_range.end
            || path_data.slice(self.source).is_none()
        {
            return Err(invalid(path_data.start));
        }
        Ok(CanonicalPathSource {
            source: self.source,
            proof: GeometryProof {
                element: element.element,
                element_range,
                path_data,
            },
        })
    }
    pub fn compute_style(
        &mut self,
        element: ElementId,
        parent: Option<&CssPaint<'src>>,
        control: &mut dyn WorkControl,
    ) -> Result<CssPaint<'src>> {
        if !self
            .tokens
            .get(element.0)
            .is_some_and(|t| matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement))
        {
            return Err(invalid(0));
        }
        if let Some(parent) = parent {
            for property in vexel_svg_css::Property::ALL {
                control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
                if let Some(value) = parent.value(property) {
                    if value.span.slice(self.source) != Some(value.bytes) {
                        return Err(invalid(value.span.offset));
                    }
                }
            }
        }
        self.css
            .compute_controlled(element.0, parent, control)
            .map_err(css_error)
    }
    pub fn find_reference(
        &mut self,
        id: XmlSpan,
        control: &mut dyn WorkControl,
    ) -> Result<Option<ElementId>> {
        if id.slice(self.source).is_none() {
            return Err(invalid(id.offset));
        }
        self.references
            .find_controlled(id, control)
            .map(|value| value.map(ElementId))
            .map_err(reference_error)
    }
    pub fn css_storage_bytes(&self) -> usize {
        self.css.allocated_bytes()
    }
    pub fn reference_storage_bytes(&self) -> usize {
        self.references.storage_bytes()
    }
    /// Number of actual hash-table slot probes performed by this owned index.
    pub fn reference_lookup_probes(&self) -> usize {
        self.references.lookup_probes()
    }
}
fn css_error(error: ControlledError<vexel_svg_css::CssError>) -> Diagnostic {
    use vexel_svg_css::CssError;
    let kind = match error {
        ControlledError::Original(CssError::Limit) => ErrorKind::Limit(LimitKind::PaintVisits),
        ControlledError::Original(CssError::OutOfMemory) | ControlledError::OutOfMemory => {
            ErrorKind::OutOfMemory
        }
        ControlledError::Original(CssError::Unsupported) => ErrorKind::Unsupported,
        ControlledError::Canceled => ErrorKind::Canceled,
        ControlledError::Limit => ErrorKind::Limit(LimitKind::Work),
        ControlledError::Overflow => ErrorKind::Limit(LimitKind::Arithmetic),
    };
    Diagnostic { kind, offset: 0 }
}
fn reference_error(error: ControlledError<vexel_svg_reference::ReferenceDiagnostic>) -> Diagnostic {
    use vexel_svg_reference::ReferenceError;
    match error {
        ControlledError::Original(e) => Diagnostic {
            kind: match e.category {
                ReferenceError::None | ReferenceError::InvalidArgument => {
                    ErrorKind::InvalidArgument
                }
                ReferenceError::Unsupported => ErrorKind::Unsupported,
                ReferenceError::Limit => ErrorKind::Limit(LimitKind::References),
                ReferenceError::OutOfMemory => ErrorKind::OutOfMemory,
            },
            offset: e.offset,
        },
        ControlledError::Canceled => stopped(Stop::Canceled),
        ControlledError::Limit => stopped(Stop::Limit),
        ControlledError::Overflow => stopped(Stop::Overflow),
        ControlledError::OutOfMemory => Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        },
    }
}
impl<'src, 'ctx> ResolveContext<'src, 'ctx> {
    pub fn validate(&self) -> Result<()> {
        if !std::sync::Arc::ptr_eq(self.shared.source_owner(), self.ledger.source_owner()) {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Validate owned cached entries against the same immutable source. Source
    /// witnesses share the original Arc allocation and keep it alive; equal
    /// source bytes in a different allocation do not authorize a cache hit.
    pub fn validate_cache(&self, control: &mut dyn WorkControl) -> Result<()> {
        self.validate()?;
        control.checkpoint().map_err(stopped)?;
        self.cache.storage_bytes_controlled(control)?;
        for entry in &self.cache.entries {
            control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
            if !std::sync::Arc::ptr_eq(&entry.source, self.shared.source_owner())
                || !self
                    .shared
                    .tokens()
                    .get(entry.definition.0)
                    .is_some_and(|token| {
                        matches!(
                            token.kind,
                            TokenKind::StartElement | TokenKind::EmptyElement
                        )
                    })
                || !entry
                    .context
                    .viewport
                    .into_iter()
                    .all(|v| v.is_finite() && v > 0.)
                || !vexel_geometry::affine_valid(entry.context.user_inverse)
                || !entry.context.bounds.into_iter().all(f64::is_finite)
                || entry.context.bounds[0] > entry.context.bounds[2]
                || entry.context.bounds[1] > entry.context.bounds[3]
                || entry.stop_range != entry.paint.stops
            {
                return Err(invalid(0));
            }
            let stops = entry
                .stop_range
                .slice(&entry.stops)
                .ok_or_else(|| invalid(0))?;
            let mut previous = 0.;
            for stop in stops {
                control.charge(WorkKind::PaintVisit, 1).map_err(stopped)?;
                if !stop.offset.is_finite()
                    || !(0.0..=1.0).contains(&stop.offset)
                    || stop.offset < previous
                {
                    return Err(invalid(0));
                }
                previous = stop.offset;
            }
        }
        for slot in &self.cache.slots {
            control
                .charge(WorkKind::ReferenceProbe, 1)
                .map_err(stopped)?;
            if slot.is_some_and(|index| index >= self.cache.entries.len()) {
                return Err(invalid(0));
            }
        }
        Ok(())
    }
}
impl<'src, 'ctx> DefinitionContext<'src, 'ctx> {
    pub fn validate(&self) -> Result<()> {
        if !std::sync::Arc::ptr_eq(self.shared.source_owner(), self.ledger.source_owner())
            || !std::sync::Arc::ptr_eq(self.shared.source_owner(), self.builder.source_owner())
        {
            return Err(invalid(0));
        }
        self.builder.traversal_scene(self.scene)?;
        Ok(())
    }
}

impl std::fmt::Debug for DecodeSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodeSource")
            .field("source_bytes", &self.source.len())
            .field("tokens", &self.tokens.len())
            .field("css_storage", &self.css_storage_bytes())
            .field("reference_storage", &self.reference_storage_bytes())
            .finish()
    }
}
