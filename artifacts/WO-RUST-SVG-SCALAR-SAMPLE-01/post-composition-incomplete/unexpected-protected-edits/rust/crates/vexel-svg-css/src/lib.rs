#![forbid(unsafe_code)]
//! Bounded, source-preserving CSS cascade for SVG XML token streams.

mod build;
#[cfg(test)]
mod capacity_tests;
mod compute;
mod control;
mod declarations;
mod matching;
mod scan;
mod selector;

pub use build::CssBuildSession;
pub use compute::CssComputeSession;
use vexel_base::{ControlledError, ControlledStep, WorkControl};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub const PROPERTY_COUNT: usize = 39;
pub const MAX_BYTES: usize = 1024 * 1024;
pub const MAX_RULES: usize = 4096;
pub const MAX_DECLARATIONS: usize = 16384;
pub const MAX_CLASSES: usize = 16;
pub const MAX_SELECTOR_TERMS: usize = 4;
pub const MAX_RULE_MATCHES: usize = 4_194_304;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Property {
    Fill,
    FillRule,
    Stroke,
    StrokeWidth,
    StrokeLinecap,
    StrokeLinejoin,
    StrokeMiterlimit,
    StrokeDasharray,
    StrokeDashoffset,
    ClipPath,
    StopColor,
    StopOpacity,
    FillOpacity,
    StrokeOpacity,
    Opacity,
    Visibility,
    Display,
    Color,
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    TextAnchor,
    LetterSpacing,
    WordSpacing,
    Filter,
    Mask,
    MixBlendMode,
    Isolation,
    VectorEffect,
    ShapeRendering,
    PaintOrder,
    Marker,
    MarkerStart,
    MarkerMid,
    MarkerEnd,
    MaskType,
    ColorInterpolationFilters,
    ClipRule,
}

impl Property {
    pub const ALL: [Self; PROPERTY_COUNT] = [
        Self::Fill,
        Self::FillRule,
        Self::Stroke,
        Self::StrokeWidth,
        Self::StrokeLinecap,
        Self::StrokeLinejoin,
        Self::StrokeMiterlimit,
        Self::StrokeDasharray,
        Self::StrokeDashoffset,
        Self::ClipPath,
        Self::StopColor,
        Self::StopOpacity,
        Self::FillOpacity,
        Self::StrokeOpacity,
        Self::Opacity,
        Self::Visibility,
        Self::Display,
        Self::Color,
        Self::FontFamily,
        Self::FontSize,
        Self::FontWeight,
        Self::FontStyle,
        Self::TextAnchor,
        Self::LetterSpacing,
        Self::WordSpacing,
        Self::Filter,
        Self::Mask,
        Self::MixBlendMode,
        Self::Isolation,
        Self::VectorEffect,
        Self::ShapeRendering,
        Self::PaintOrder,
        Self::Marker,
        Self::MarkerStart,
        Self::MarkerMid,
        Self::MarkerEnd,
        Self::MaskType,
        Self::ColorInterpolationFilters,
        Self::ClipRule,
    ];

    fn name(self) -> &'static [u8] {
        PROPERTY_NAMES[self as usize]
    }
}

const PROPERTY_NAMES: [&[u8]; PROPERTY_COUNT] = [
    b"fill",
    b"fill-rule",
    b"stroke",
    b"stroke-width",
    b"stroke-linecap",
    b"stroke-linejoin",
    b"stroke-miterlimit",
    b"stroke-dasharray",
    b"stroke-dashoffset",
    b"clip-path",
    b"stop-color",
    b"stop-opacity",
    b"fill-opacity",
    b"stroke-opacity",
    b"opacity",
    b"visibility",
    b"display",
    b"color",
    b"font-family",
    b"font-size",
    b"font-weight",
    b"font-style",
    b"text-anchor",
    b"letter-spacing",
    b"word-spacing",
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
    b"clip-rule",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssError {
    Limit,
    OutOfMemory,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssValue<'a> {
    pub span: XmlSpan,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssPaint<'a> {
    values: [Option<CssValue<'a>>; PROPERTY_COUNT],
    source: Option<&'a [u8]>,
}

impl<'a> CssPaint<'a> {
    pub const fn initial() -> Self {
        Self {
            values: [None; PROPERTY_COUNT],
            source: None,
        }
    }
    pub fn value(&self, property: Property) -> Option<CssValue<'a>> {
        self.values[property as usize]
    }
    pub fn keyword_equals(&self, property: Property, keyword: &[u8]) -> bool {
        self.value(property)
            .is_some_and(|value| value.bytes.eq_ignore_ascii_case(keyword))
    }
    /// Preserve the computed context while preventing a marker's contents from
    /// inheriting the host's marker declarations. Explicit child declarations
    /// still pass through the normal cascade. All retained spans borrow the
    /// original source, and the caller's parent style remains unchanged.
    pub fn without_inherited_markers(mut self) -> Self {
        for property in [
            Property::Marker,
            Property::MarkerStart,
            Property::MarkerMid,
            Property::MarkerEnd,
        ] {
            self.values[property as usize] = None;
        }
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct Term {
    element: Option<XmlSpan>,
    id: Option<XmlSpan>,
    classes: [XmlSpan; MAX_CLASSES],
    class_count: usize,
    specificity: u32,
}
impl Term {
    const EMPTY: Self = Self {
        element: None,
        id: None,
        classes: [XmlSpan {
            offset: 0,
            length: 0,
        }; MAX_CLASSES],
        class_count: 0,
        specificity: 0,
    };
}

#[derive(Debug, Clone, Copy)]
struct Selector {
    terms: [Term; MAX_SELECTOR_TERMS],
    count: usize,
    specificity: u32,
}
impl Selector {
    const EMPTY: Self = Self {
        terms: [Term::EMPTY; MAX_SELECTOR_TERMS],
        count: 0,
        specificity: 0,
    };
}

#[derive(Debug, Clone, Copy)]
struct Declaration {
    property: Property,
    value: XmlSpan,
    important: bool,
}

struct Rule {
    selector: Selector,
    declarations: Vec<Declaration>,
    order: usize,
    unsupported: bool,
}

/// A stylesheet borrows its immutable XML source. Rule values and selectors retain source spans.
pub struct CssSheet<'a> {
    source: &'a [u8],
    tokens: &'a [XmlToken],
    rules: Vec<Rule>,
    declaration_count: usize,
    css_bytes: usize,
    match_tests: usize,
    selector_visits: usize,
    hit_limit: bool,
    allocated_bytes: usize,
}

impl<'src> CssSheet<'src> {
    pub fn create(source: &'src [u8], tokens: &'src [XmlToken]) -> Result<Self, CssError> {
        Self::create_controlled(source, tokens, &mut control::NoStop).map_err(control::legacy_error)
    }

    pub fn create_controlled(
        source: &'src [u8],
        tokens: &'src [XmlToken],
        control: &mut dyn WorkControl,
    ) -> Result<Self, ControlledError<CssError>> {
        let mut session = CssBuildSession::new(source, tokens);
        loop {
            match session.step(256, control) {
                ControlledStep::Pending { .. } => {}
                _ => return session.finish(),
            }
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub fn css_bytes(&self) -> usize {
        self.css_bytes
    }

    /// Actual retained rule and nested declaration Vec capacities; borrowed XML is excluded.
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    /// The session copies the parent paint. Its temporary borrow cannot shorten source spans.
    pub fn begin_compute<'ctx>(
        &'ctx mut self,
        element: usize,
        parent: Option<&CssPaint<'src>>,
    ) -> CssComputeSession<'ctx, 'src> {
        CssComputeSession::new(self, element, parent.copied())
    }

    pub fn compute(
        &mut self,
        element: usize,
        parent: Option<&CssPaint<'src>>,
    ) -> Result<CssPaint<'src>, CssError> {
        self.compute_controlled(element, parent, &mut control::NoStop)
            .map_err(control::legacy_error)
    }

    pub fn compute_controlled(
        &mut self,
        element: usize,
        parent: Option<&CssPaint<'src>>,
        control: &mut dyn WorkControl,
    ) -> Result<CssPaint<'src>, ControlledError<CssError>> {
        let mut session = self.begin_compute(element, parent);
        loop {
            match session.step(256, control) {
                ControlledStep::Pending { .. } => {}
                _ => return session.finish(),
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Winner {
    value: XmlSpan,
    important: bool,
    specificity: u32,
    order: usize,
}
fn offer(
    winners: &mut [Option<Winner>; PROPERTY_COUNT],
    declaration: Declaration,
    specificity: u32,
    order: usize,
) {
    let slot = &mut winners[declaration.property as usize];
    let wins = slot.is_none_or(|old| {
        declaration.important > old.important
            || (declaration.important == old.important
                && (specificity > old.specificity
                    || (specificity == old.specificity && order >= old.order)))
    });
    if wins {
        *slot = Some(Winner {
            value: declaration.value,
            important: declaration.important,
            specificity,
            order,
        });
    }
}

pub fn is_style_element(source: &[u8], token: &XmlToken) -> bool {
    matches!(
        token.kind,
        TokenKind::StartElement | TokenKind::EmptyElement
    ) && token.prefix.length == 0
        && span_eq(source, token.local_name, b"style")
}

pub fn keyword_equals(source: &[u8], span: XmlSpan, literal: &[u8]) -> bool {
    checked_slice(source, span).is_ok_and(|value| value.eq_ignore_ascii_case(literal))
}
fn is_element(token: &XmlToken) -> bool {
    matches!(
        token.kind,
        TokenKind::StartElement | TokenKind::EmptyElement
    )
}
fn checked_slice(source: &[u8], span: XmlSpan) -> Result<&[u8], CssError> {
    span.slice(source).ok_or(CssError::Limit)
}
fn span_eq(source: &[u8], span: XmlSpan, bytes: &[u8]) -> bool {
    checked_slice(source, span).is_ok_and(|v| v == bytes)
}
fn inherited(property: Property) -> bool {
    !matches!(
        property,
        Property::ClipPath
            | Property::Opacity
            | Property::Display
            | Property::Filter
            | Property::Mask
            | Property::MixBlendMode
            | Property::Isolation
            | Property::VectorEffect
    )
}
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n' | 0x0c)
}
fn ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-') || c >= 0x80
}

fn reserve<T>(buffer: &mut Vec<T>, additional: usize, exact: bool) -> Result<(), CssError> {
    if exact {
        buffer.try_reserve_exact(additional)
    } else {
        buffer.try_reserve(additional)
    }
    .map_err(|_| CssError::OutOfMemory)
}
