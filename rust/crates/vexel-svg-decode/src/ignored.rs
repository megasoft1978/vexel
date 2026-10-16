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
}
impl IgnoredSink {
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
    if vexel_svg_reference::attributes_supported(source, window, 0, root, t.local_name) {
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
        if vexel_svg_reference::attributes_supported(source, &pair, 0, root, t.local_name) {
            continue;
        }
        if a.prefix.length != 0 {
            return Ok(false);
        }
        let writing = eq(source, a.local_name, b"writing-mode");
        if !writing && affects_rendering(source, a.local_name) {
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
