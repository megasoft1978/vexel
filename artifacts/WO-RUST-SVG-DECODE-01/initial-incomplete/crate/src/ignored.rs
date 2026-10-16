//! Ignored-feature diagnostics and fail-closed attribute classification.
use vexel_scene::{Diagnostic, ErrorKind, IgnoredKind, Result, SceneBuilder};
use vexel_xml::{TokenKind, XmlSpan, XmlToken};

pub fn ignored_kind_name(kind: IgnoredKind) -> &'static str {
    match kind {
        IgnoredKind::UnknownAttribute => "unknown-attribute",
        IgnoredKind::Meta => "meta",
        IgnoredKind::Doctype => "doctype",
        IgnoredKind::WritingMode => "writing-mode",
    }
}
fn eq(source: &[u8], span: XmlSpan, name: &[u8]) -> bool {
    span.slice(source) == Some(name)
}
fn ci_eq(source: &[u8], at: usize, name: &[u8]) -> bool {
    source
        .get(at..at.saturating_add(name.len()))
        .is_some_and(|s| {
            s.iter()
                .zip(name)
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        })
}
fn ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')
}
fn note(builder: &mut SceneBuilder, kind: IgnoredKind, offset: usize) -> Result<()> {
    builder.record_ignored(kind, offset)
}
fn scan_css(source: &[u8], span: XmlSpan, builder: &mut SceneBuilder) -> Result<()> {
    const NAME: &[u8] = b"writing-mode";
    let end = span.offset.checked_add(span.length).ok_or(Diagnostic {
        kind: ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic),
        offset: span.offset,
    })?;
    if end > source.len() {
        return Ok(());
    }
    for i in span.offset..=end.saturating_sub(NAME.len()) {
        if !ci_eq(source, i, NAME) || i > span.offset && ident(source[i - 1]) {
            continue;
        }
        let mut j = i + NAME.len();
        while j < end && matches!(source[j], b' ' | b'\t' | b'\r' | b'\n') {
            j += 1
        }
        if j < end && source[j] == b':' {
            note(builder, IgnoredKind::WritingMode, i)?
        }
    }
    Ok(())
}
/// Record document level meta/doctype/writing-mode notes once before traversal.
pub fn scan_ignored(
    source: &[u8],
    tokens: &[XmlToken],
    builder: &mut SceneBuilder,
    control: &mut dyn vexel_base::WorkControl,
) -> Result<()> {
    for (i, t) in tokens.iter().enumerate() {
        control
            .charge(vexel_base::WorkKind::TokenVisit, 1)
            .map_err(|s| Diagnostic {
                kind: match s {
                    vexel_base::Stop::Canceled => ErrorKind::Canceled,
                    vexel_base::Stop::Limit => ErrorKind::Limit(vexel_scene::LimitKind::Work),
                    vexel_base::Stop::Overflow => {
                        ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic)
                    }
                },
                offset: 0,
            })?;
        match t.kind {
            TokenKind::Doctype => note(builder, IgnoredKind::Doctype, t.source.offset)?,
            TokenKind::StartElement | TokenKind::EmptyElement => {
                if eq(source, t.local_name, b"meta") {
                    note(builder, IgnoredKind::Meta, t.source.offset)?
                }
                if t.kind == TokenKind::StartElement && vexel_svg_css::is_style_element(source, t) {
                    let mut j = i + 1;
                    while j < tokens.len()
                        && !(tokens[j].kind == TokenKind::EndElement && tokens[j].depth == t.depth)
                    {
                        if matches!(tokens[j].kind, TokenKind::Text | TokenKind::Cdata) {
                            scan_css(source, tokens[j].value, builder)?
                        }
                        j += 1
                    }
                }
            }
            TokenKind::Attribute if t.prefix.length == 0 => {
                if eq(source, t.local_name, b"writing-mode") {
                    note(builder, IgnoredKind::WritingMode, t.name.offset)?
                } else if eq(source, t.local_name, b"style") {
                    scan_css(source, t.value, builder)?
                }
            }
            _ => {}
        }
    }
    Ok(())
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
/// Validate render-affecting attributes and account unknown harmless attributes once.
pub fn traverse_attributes_supported(
    source: &[u8],
    tokens: &[XmlToken],
    element: usize,
    root: bool,
    visited: &mut [bool],
    builder: &mut SceneBuilder,
) -> Result<bool> {
    let Some(token) = tokens.get(element) else {
        return Ok(false);
    };
    if !vexel_svg_reference::attributes_supported(source, tokens, element, root, token.local_name) {
        // Report the first offending attribute offset using the exact single-attribute predicate.
        for (index, a) in tokens
            .iter()
            .enumerate()
            .skip(element + 1)
            .take_while(|(_, a)| a.kind == TokenKind::Attribute)
        {
            let pair = [*token, *a];
            if !vexel_svg_reference::attributes_supported(source, &pair, 0, root, token.local_name)
            {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: a.name.offset,
                });
            }
            if a.prefix.length != 0 || affects_rendering(source, a.local_name) {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: a.name.offset,
                });
            }
            if !eq(source, a.local_name, b"xmlns") && !eq(source, a.local_name, b"writing-mode") {
                if let Some(slot) = visited.get_mut(index) {
                    if !*slot {
                        *slot = true;
                        note(builder, IgnoredKind::UnknownAttribute, a.name.offset)?
                    }
                }
            }
        }
        return Ok(false);
    }
    Ok(true)
}
