//! One canonical source/token owner for ephemeral CSS/reference state. Lower
//! modules query it through methods instead of replacing its borrowed tables.
use crate::types::{invalid, stopped};
use crate::*;
use vexel_base::ControlledError;
use vexel_xml::{TokenKind, XmlSpan, XmlToken};
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
        if self.cache.source.as_ref().is_some_and(|source| {
            !std::sync::Arc::ptr_eq(source, self.shared.source_owner())
        }) {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Bind a fresh cache before storing any resolved source-dependent data.
    /// Arc cloning shares the original allocation; storage accounting never
    /// counts its source bytes a second time.
    pub fn bind_cache(&mut self) -> Result<()> {
        self.validate()?;
        if self.cache.source.is_none() {
            if !self.cache.entries.is_empty()
                || !self.cache.slots.is_empty()
                || !self.cache.stops.is_empty()
            {
                return Err(invalid(0));
            }
            self.cache.source = Some(self.shared.source_owner().clone());
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
        self.builder.scene(self.scene)?;
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
