//! One root-owned ignored-feature sink shared by all expansion scenes.
use crate::source::{charge, element_name};
use vexel_scene::{
    Diagnostic, ErrorKind, Ignored, IgnoredKind, LimitKind, Result, SceneBuilder, WorkControl,
    WorkKind,
};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};
#[derive(Debug, Default)]
pub struct IgnoredSink {
    pub records: [Ignored; 4],
    background_input: Option<bool>,
}
impl IgnoredSink {
    /// `enable-background` only affects rendering through the deprecated
    /// `BackgroundImage`/`BackgroundAlpha` filter inputs; without either in the
    /// document it is inert. Scanned once per document.
    fn uses_background_input(&mut self, source: &[u8]) -> bool {
        *self.background_input.get_or_insert_with(|| {
            [&b"BackgroundImage"[..], b"BackgroundAlpha"]
                .iter()
                .any(|n| source.windows(n.len()).any(|w| w == *n))
        })
    }
    pub fn note(&mut self, kind: IgnoredKind, offset: usize) -> Result<()> {
        let r = &mut self.records[kind as usize];
        r.count = r.count.checked_add(1).ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset,
        })?;
        r.first_offset = Some(r.first_offset.map_or(offset, |old| old.min(offset)));
        Ok(())
    }
    pub fn publish(&self, builder: &mut SceneBuilder, control: &mut dyn WorkControl) -> Result<()> {
        for kind in [
            IgnoredKind::UnknownAttribute,
            IgnoredKind::Meta,
            IgnoredKind::Doctype,
            IgnoredKind::WritingMode,
        ] {
            let r = self.records[kind as usize];
            for _ in 0..r.count {
                charge(
                    control,
                    WorkKind::TokenVisit,
                    1,
                    r.first_offset.unwrap_or(0),
                )?;
                builder.record_ignored(kind, r.first_offset.unwrap_or(0))?;
            }
        }
        Ok(())
    }
    pub fn scan(
        &mut self,
        source: &[u8],
        tokens: &[XmlToken],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        for (i, t) in tokens.iter().enumerate() {
            charge(control, WorkKind::TokenVisit, 1, t.source.offset)?;
            match t.kind {
                TokenKind::Doctype => self.note(IgnoredKind::Doctype, t.source.offset)?,
                TokenKind::StartElement | TokenKind::EmptyElement => {
                    if element_name(source, t, b"meta") {
                        self.note(IgnoredKind::Meta, t.source.offset)?
                    }
                    if t.kind == TokenKind::StartElement
                        && vexel_svg_css::is_style_element(source, t)
                    {
                        for x in tokens.iter().skip(i + 1).take_while(|x| {
                            !(x.kind == TokenKind::EndElement && x.depth == t.depth)
                        }) {
                            charge(control, WorkKind::TokenVisit, 1, x.source.offset)?;
                            if matches!(x.kind, TokenKind::Text | TokenKind::Cdata) {
                                self.scan_css(source, x.value, control)?
                            }
                        }
                    }
                }
                TokenKind::Attribute if t.prefix.length == 0 => {
                    if eq(source, t.local_name, b"writing-mode") {
                        self.note(IgnoredKind::WritingMode, t.name.offset)?
                    } else if eq(source, t.local_name, b"style") {
                        self.scan_css(source, t.value, control)?
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn scan_css(
        &mut self,
        source: &[u8],
        span: XmlSpan,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        const NAME: &[u8] = b"writing-mode";
        let Some(raw) = span.slice(source) else {
            return Ok(());
        };
        if raw.len() < NAME.len() {
            return Ok(());
        }
        for i in 0..=raw.len() - NAME.len() {
            charge(control, WorkKind::SourceRead, 1, span.offset + i)?;
            if !raw[i..i + NAME.len()].eq_ignore_ascii_case(NAME) || i > 0 && ident(raw[i - 1]) {
                continue;
            }
            let mut j = i + NAME.len();
            while j < raw.len() && matches!(raw[j], b' ' | b'\t' | b'\r' | b'\n') {
                charge(control, WorkKind::SourceRead, 1, span.offset + j)?;
                j += 1
            }
            if raw.get(j) == Some(&b':') {
                self.note(IgnoredKind::WritingMode, span.offset + i)?
            }
        }
        Ok(())
    }
}
pub fn ignored_kind_name(kind: IgnoredKind) -> &'static str {
    match kind {
        IgnoredKind::UnknownAttribute => "unknown-attribute",
        IgnoredKind::Meta => "meta",
        IgnoredKind::Doctype => "doctype",
        IgnoredKind::WritingMode => "writing-mode",
    }
}
fn ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')
}
fn eq(source: &[u8], span: XmlSpan, name: &[u8]) -> bool {
    span.slice(source) == Some(name)
}
pub fn scan_ignored(
    source: &[u8],
    tokens: &[XmlToken],
    builder: &mut SceneBuilder,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let mut sink = IgnoredSink::default();
    sink.scan(source, tokens, control)?;
    sink.publish(builder, control)
}
fn affects_rendering(source: &[u8], name: XmlSpan) -> bool {
    const KNOWN: &[&[u8]] = &[
        b"alignment-baseline",
        b"baseline-shift",
        b"clip",
        b"color-interpolation",
        b"color-rendering",
        b"cursor",
        b"direction",
        b"dominant-baseline",
        b"enable-background",
        b"flood-color",
        b"flood-opacity",
        b"font",
        b"font-size-adjust",
        b"font-stretch",
        b"font-variant",
        b"glyph-orientation-horizontal",
        b"glyph-orientation-vertical",
        b"image-rendering",
        b"kerning",
        b"lighting-color",
        b"overflow",
        b"stop-color",
        b"stop-opacity",
        b"text-decoration",
        b"text-rendering",
        b"unicode-bidi",
        b"transform-origin",
        b"transform-box",
        b"d",
        b"x",
        b"y",
        b"width",
        b"height",
        b"rx",
        b"ry",
        b"cx",
        b"cy",
        b"r",
        b"x1",
        b"y1",
        b"x2",
        b"y2",
        b"points",
        b"dx",
        b"dy",
        b"rotate",
        b"textLength",
        b"lengthAdjust",
        b"href",
        b"viewBox",
        b"pathLength",
        b"preserveAspectRatio",
        b"clipPathUnits",
        b"maskUnits",
        b"maskContentUnits",
        b"patternUnits",
        b"patternContentUnits",
        b"patternTransform",
        b"gradientUnits",
        b"gradientTransform",
        b"offset",
        b"requiredFeatures",
        b"requiredExtensions",
        b"systemLanguage",
    ];
    KNOWN.iter().any(|literal| eq(source, name, literal))
}
pub fn attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    root: bool,
    visited: &mut [bool],
    sink: &mut IgnoredSink,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(t) = tokens.get(element) else {
        return Ok(false);
    };
    let attr_end = element
        + 1
        + tokens
            .iter()
            .skip(element + 1)
            .take_while(|a| a.kind == TokenKind::Attribute)
            .count();
    let window = tokens
        .get(element..attr_end)
        .ok_or_else(|| crate::invalid(t.name.offset))?;
    let bindings = Bindings::resolve(source, tokens, element, control)?;
    let foreign = |p: XmlSpan| bindings.is_foreign(source, p);
    if vexel_svg_reference::attributes_supported_with(
        source,
        window,
        0,
        root,
        t.local_name,
        &foreign,
    ) {
        return Ok(true);
    }
    for (i, a) in tokens
        .iter()
        .enumerate()
        .skip(element + 1)
        .take_while(|(_, a)| a.kind == TokenKind::Attribute)
    {
        charge(control, WorkKind::TokenVisit, 1, a.name.offset)?;
        let pair = [*t, *a];
        if vexel_svg_reference::attributes_supported_with(
            source,
            &pair,
            0,
            root,
            t.local_name,
            &foreign,
        ) {
            // A foreign-prefixed attribute that the SVG rules would reject is ignored and noted once.
            if a.prefix.length != 0
                && !vexel_svg_reference::attributes_supported(source, &pair, 0, root, t.local_name)
            {
                let first = visited.get_mut(i).ok_or(Diagnostic {
                    kind: ErrorKind::InvalidArgument,
                    offset: a.name.offset,
                })?;
                if !*first {
                    *first = true;
                    sink.note(IgnoredKind::UnknownAttribute, a.name.offset)?
                }
            }
            continue;
        }
        if a.prefix.length != 0 {
            return Ok(false);
        }
        let writing = eq(source, a.local_name, b"writing-mode");
        let inert_background =
            eq(source, a.local_name, b"enable-background") && !sink.uses_background_input(source);
        if !writing && !inert_background && affects_rendering(source, a.local_name) {
            return Ok(false);
        }
        if eq(source, a.local_name, b"xmlns") {
            continue;
        }
        if !writing {
            let first = visited.get_mut(i).ok_or(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: a.name.offset,
            })?;
            if !*first {
                *first = true;
                sink.note(IgnoredKind::UnknownAttribute, a.name.offset)?
            }
        }
    }
    Ok(true)
}

/// Prefix bindings in scope for one element: each distinct attribute prefix with whether its
/// nearest `xmlns:prefix` declaration binds a non-SVG namespace. Unbound prefixes are not foreign.
struct Bindings(Vec<(XmlSpan, bool)>);

impl Bindings {
    fn resolve(
        source: &[u8],
        tokens: &[XmlToken],
        element: usize,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        let mut out: Vec<(XmlSpan, bool)> = Vec::new();
        for a in tokens
            .iter()
            .skip(element + 1)
            .take_while(|a| a.kind == TokenKind::Attribute)
        {
            if a.prefix.length == 0 {
                continue;
            }
            let Some(p) = a.prefix.slice(source) else {
                continue;
            };
            if p == b"xmlns" || p == b"xml" || out.iter().any(|(q, _)| q.slice(source) == Some(p)) {
                continue;
            }
            let foreign = nearest_binding_is_foreign(source, tokens, element, p, control)?;
            out.push((a.prefix, foreign));
        }
        Ok(Self(out))
    }
    fn is_foreign(&self, source: &[u8], prefix: XmlSpan) -> bool {
        let Some(p) = prefix.slice(source) else {
            return false;
        };
        self.0
            .iter()
            .any(|(q, foreign)| *foreign && q.slice(source) == Some(p))
    }
}

/// Nearest in-scope `xmlns:prefix` declaration: the element itself first, then ancestors
/// (nearest first). Walks back by depth, so each ancestor start tag is visited once.
fn nearest_binding_is_foreign(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    prefix: &[u8],
    control: &mut dyn WorkControl,
) -> Result<bool> {
    let Some(own) = tokens.get(element) else {
        return Ok(false);
    };
    if let Some(decision) = declared_binding(source, tokens, element, prefix) {
        return Ok(decision);
    }
    let mut level = own.depth;
    for i in (0..element).rev() {
        let t = tokens[i];
        charge(control, WorkKind::TokenVisit, 1, t.name.offset)?;
        if t.kind != TokenKind::StartElement || t.depth >= level {
            continue;
        }
        level = t.depth;
        if let Some(decision) = declared_binding(source, tokens, i, prefix) {
            return Ok(decision);
        }
        if level == 0 {
            break;
        }
    }
    Ok(false)
}

/// Some(true) when the start tag at `element` declares `xmlns:prefix` to a non-SVG namespace,
/// Some(false) for an SVG or unusable (empty, entity-bearing) URI, None when not declared there.
fn declared_binding(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    prefix: &[u8],
) -> Option<bool> {
    tokens
        .iter()
        .skip(element + 1)
        .take_while(|a| a.kind == TokenKind::Attribute)
        .find(|a| {
            a.prefix.slice(source) == Some(b"xmlns") && a.local_name.slice(source) == Some(prefix)
        })
        .map(|a| match a.value.slice(source) {
            Some(uri) if !uri.is_empty() && !uri.contains(&b'&') => {
                uri != b"http://www.w3.org/2000/svg"
            }
            _ => false,
        })
}

pub fn traverse_attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    root: bool,
    visited: &mut [bool],
    builder: &mut SceneBuilder,
) -> Result<bool> {
    let mut sink = IgnoredSink::default();
    let okay = attributes_supported(
        source,
        tokens,
        element,
        root,
        visited,
        &mut sink,
        &mut NoStop,
    )?;
    sink.publish(builder, &mut NoStop)?;
    Ok(okay)
}
struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
}
