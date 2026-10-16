#![forbid(unsafe_code)]
//! Bounded, source-preserving CSS cascade for SVG XML token streams.

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
    Fill, FillRule, Stroke, StrokeWidth, StrokeLinecap, StrokeLinejoin,
    StrokeMiterlimit, StrokeDasharray, StrokeDashoffset, ClipPath, StopColor,
    StopOpacity, FillOpacity, StrokeOpacity, Opacity, Visibility, Display, Color,
    FontFamily, FontSize, FontWeight, FontStyle, TextAnchor, LetterSpacing,
    WordSpacing, Filter, Mask, MixBlendMode, Isolation, VectorEffect,
    ShapeRendering, PaintOrder, Marker, MarkerStart, MarkerMid, MarkerEnd,
    MaskType, ColorInterpolationFilters, ClipRule,
}

impl Property {
    pub const ALL: [Self; PROPERTY_COUNT] = [
        Self::Fill, Self::FillRule, Self::Stroke, Self::StrokeWidth, Self::StrokeLinecap,
        Self::StrokeLinejoin, Self::StrokeMiterlimit, Self::StrokeDasharray,
        Self::StrokeDashoffset, Self::ClipPath, Self::StopColor, Self::StopOpacity,
        Self::FillOpacity, Self::StrokeOpacity, Self::Opacity, Self::Visibility,
        Self::Display, Self::Color, Self::FontFamily, Self::FontSize, Self::FontWeight,
        Self::FontStyle, Self::TextAnchor, Self::LetterSpacing, Self::WordSpacing,
        Self::Filter, Self::Mask, Self::MixBlendMode, Self::Isolation, Self::VectorEffect,
        Self::ShapeRendering, Self::PaintOrder, Self::Marker, Self::MarkerStart,
        Self::MarkerMid, Self::MarkerEnd, Self::MaskType, Self::ColorInterpolationFilters,
        Self::ClipRule,
    ];

    fn name(self) -> &'static [u8] {
        PROPERTY_NAMES[self as usize]
    }
}

const PROPERTY_NAMES: [&[u8]; PROPERTY_COUNT] = [
    b"fill", b"fill-rule", b"stroke", b"stroke-width", b"stroke-linecap",
    b"stroke-linejoin", b"stroke-miterlimit", b"stroke-dasharray", b"stroke-dashoffset",
    b"clip-path", b"stop-color", b"stop-opacity", b"fill-opacity", b"stroke-opacity",
    b"opacity", b"visibility", b"display", b"color", b"font-family", b"font-size",
    b"font-weight", b"font-style", b"text-anchor", b"letter-spacing", b"word-spacing",
    b"filter", b"mask", b"mix-blend-mode", b"isolation", b"vector-effect",
    b"shape-rendering", b"paint-order", b"marker", b"marker-start", b"marker-mid",
    b"marker-end", b"mask-type", b"color-interpolation-filters", b"clip-rule",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssError { Limit, OutOfMemory, Unsupported }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssValue<'a> { pub span: XmlSpan, pub bytes: &'a [u8] }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssPaint<'a> {
    values: [Option<CssValue<'a>>; PROPERTY_COUNT],
    source: Option<&'a [u8]>,
}

impl<'a> CssPaint<'a> {
    pub const fn initial() -> Self { Self { values: [None; PROPERTY_COUNT], source: None } }
    pub fn value(&self, property: Property) -> Option<CssValue<'a>> { self.values[property as usize] }
    pub fn keyword_equals(&self, property: Property, keyword: &[u8]) -> bool {
        self.value(property).is_some_and(|value| value.bytes.eq_ignore_ascii_case(keyword))
    }
}

#[derive(Debug, Clone, Copy)]
struct Term { element: Option<XmlSpan>, id: Option<XmlSpan>, classes: [XmlSpan; MAX_CLASSES], class_count: usize, specificity: u32 }
impl Term { const EMPTY: Self = Self { element: None, id: None, classes: [XmlSpan { offset: 0, length: 0 }; MAX_CLASSES], class_count: 0, specificity: 0 }; }

#[derive(Debug, Clone, Copy)]
struct Selector { terms: [Term; MAX_SELECTOR_TERMS], count: usize, specificity: u32 }
impl Selector { const EMPTY: Self = Self { terms: [Term::EMPTY; MAX_SELECTOR_TERMS], count: 0, specificity: 0 }; }

#[derive(Debug, Clone, Copy)]
struct Declaration { property: Property, value: XmlSpan, important: bool }

struct Rule { selector: Selector, declarations: Vec<Declaration>, order: usize, unsupported: bool }

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
}

impl<'a> CssSheet<'a> {
    pub fn create(source: &'a [u8], tokens: &'a [XmlToken]) -> Result<Self, CssError> {
        let mut sheet = Self { source, tokens, rules: Vec::new(), declaration_count: 0, css_bytes: 0,
            match_tests: 0, selector_visits: 0, hit_limit: false };
        for (index, token) in tokens.iter().enumerate() {
            if token.kind != TokenKind::StartElement || !is_style_element(source, token) { continue; }
            let mut cursor = index + 1;
            while cursor < tokens.len() && tokens[cursor].kind != TokenKind::EndElement {
                let child = &tokens[cursor];
                if matches!(child.kind, TokenKind::Text | TokenKind::Cdata) {
                    let bytes = checked_slice(source, child.value)?;
                    sheet.css_bytes = sheet.css_bytes.checked_add(bytes.len()).ok_or(CssError::Limit)?;
                    if sheet.css_bytes > MAX_BYTES { return Err(CssError::Limit); }
                    sheet.parse_stylesheet(child.value.offset, child.value.offset + child.value.length)?;
                }
                cursor += 1;
            }
        }
        Ok(sheet)
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }
    pub fn css_bytes(&self) -> usize { self.css_bytes }

    /// Compute against the source and token stream retained by this sheet.
    /// Parent paints from another source are rejected using slice identity, with no source copy.
    pub fn compute(&mut self, element: usize, parent: Option<&CssPaint<'a>>) -> Result<CssPaint<'a>, CssError> {
        let source = self.source;
        let tokens = self.tokens;
        if parent.and_then(|paint| paint.source).is_some_and(|parent_source| !std::ptr::eq(parent_source, source)) {
            return Err(CssError::Limit);
        }
        let token = tokens.get(element).ok_or(CssError::Limit)?;
        if !is_element(token) { return Err(CssError::Limit); }
        let mut paint = parent.copied().unwrap_or_else(CssPaint::initial);
        paint.source = Some(source);
        for property in [Property::ClipPath, Property::Filter, Property::Mask, Property::MixBlendMode,
            Property::Isolation, Property::VectorEffect, Property::Opacity, Property::Display] {
            paint.values[property as usize] = None;
        }
        let mut winners: [Option<Winner>; PROPERTY_COUNT] = [None; PROPERTY_COUNT];
        let selector_visits = &mut self.selector_visits;
        let hit_limit = &mut self.hit_limit;
        for rule in &self.rules {
            if self.match_tests >= MAX_RULE_MATCHES { self.hit_limit = true; return Err(CssError::Limit); }
            self.match_tests += 1;
            if matches_selector(source, &rule.selector, tokens, element, selector_visits, hit_limit)? {
                if rule.unsupported && token.prefix.length == 0 && span_eq(source, token.local_name, b"stop") {
                    return Err(CssError::Unsupported);
                }
                for (i, declaration) in rule.declarations.iter().enumerate() {
                    offer(&mut winners, *declaration, rule.selector.specificity,
                          rule.order.saturating_mul(MAX_DECLARATIONS).saturating_add(i));
                }
            }
        }
        // Presentation attributes have zero specificity and precede stylesheet rules.
        for property in Property::ALL {
            if let Some(value) = attribute(source, tokens, element, property.name()) {
                offer(&mut winners, Declaration { property, value, important: false }, 0, 0);
            }
        }
        if let Some(style) = attribute(source, tokens, element, b"style") {
            if style.length > MAX_BYTES { return Err(CssError::Limit); }
            let mut inline = Vec::new();
            let _ = parse_declarations(source, style, &mut inline, &mut self.hit_limit)?;
            for (i, declaration) in inline.iter().enumerate() {
                offer(&mut winners, *declaration, 1_000_000, i);
            }
        }
        for property in Property::ALL {
            if let Some(winner) = winners[property as usize] {
                let value = checked_slice(source, winner.value)?;
                if keyword(value, b"inherit") || (keyword(value, b"unset") && inherited(property)) {
                    paint.values[property as usize] = parent.and_then(|p| p.value(property));
                } else if keyword(value, b"initial") || keyword(value, b"unset") {
                    paint.values[property as usize] = None;
                } else {
                    paint.values[property as usize] = Some(CssValue { span: winner.value, bytes: value });
                }
            }
        }
        Ok(paint)
    }

    fn parse_stylesheet(&mut self, start: usize, end: usize) -> Result<(), CssError> {
        let mut i = start;
        while i < end {
            skip_space_comments(self.source, &mut i, end);
            if i >= end { break; }
            if self.source[i] == b'@' {
                i = skip_at_rule(self.source, i, end);
                continue;
            }
            let selector_start = i;
            let Some(brace) = find_unquoted(self.source, i, end, b'{') else { break };
            let Some(close) = matching_brace(self.source, brace + 1, end) else { break };
            let mut declarations = Vec::new();
            let unsupported = parse_declarations(self.source, XmlSpan { offset: brace + 1, length: close - brace - 1 }, &mut declarations, &mut self.hit_limit)?;
            let mut part = selector_start;
            while part < brace {
                let comma = find_unquoted(self.source, part, brace, b',').unwrap_or(brace);
                if let Some(selector) = parse_selector(self.source, part, comma) {
                    if !declarations.is_empty() || unsupported {
                        let new_rules = self.rules.len().checked_add(1).ok_or(CssError::Limit)?;
                        if new_rules > MAX_RULES || declarations.len() > MAX_DECLARATIONS.saturating_sub(self.declaration_count) {
                            self.hit_limit = true; return Err(CssError::Limit);
                        }
                        self.rules.try_reserve(1).map_err(|_| CssError::OutOfMemory)?;
                        let mut copy = Vec::new();
                        copy.try_reserve_exact(declarations.len()).map_err(|_| CssError::OutOfMemory)?;
                        copy.extend_from_slice(&declarations);
                        self.declaration_count += copy.len();
                        self.rules.push(Rule { selector, declarations: copy, order: self.rules.len(), unsupported });
                    }
                }
                part = if comma < brace { comma + 1 } else { brace };
            }
            i = close + 1;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Winner { value: XmlSpan, important: bool, specificity: u32, order: usize }
fn offer(winners: &mut [Option<Winner>; PROPERTY_COUNT], declaration: Declaration, specificity: u32, order: usize) {
    let slot = &mut winners[declaration.property as usize];
    let wins = slot.is_none_or(|old| declaration.important > old.important ||
        (declaration.important == old.important && (specificity > old.specificity ||
        (specificity == old.specificity && order >= old.order))));
    if wins { *slot = Some(Winner { value: declaration.value, important: declaration.important, specificity, order }); }
}

pub fn is_style_element(source: &[u8], token: &XmlToken) -> bool {
    matches!(token.kind, TokenKind::StartElement | TokenKind::EmptyElement) &&
        token.prefix.length == 0 && span_eq(source, token.local_name, b"style")
}

pub fn keyword_equals(source: &[u8], span: XmlSpan, literal: &[u8]) -> bool {
    checked_slice(source, span).is_ok_and(|value| value.eq_ignore_ascii_case(literal))
}

fn is_element(token: &XmlToken) -> bool { matches!(token.kind, TokenKind::StartElement | TokenKind::EmptyElement) }
fn checked_slice(source: &[u8], span: XmlSpan) -> Result<&[u8], CssError> {
    span.slice(source).ok_or(CssError::Limit)
}
fn span_eq(source: &[u8], span: XmlSpan, bytes: &[u8]) -> bool { checked_slice(source, span).is_ok_and(|v| v == bytes) }
fn attribute(source: &[u8], tokens: &[XmlToken], element: usize, name: &[u8]) -> Option<XmlSpan> {
    let mut i = element.checked_add(1)?;
    while let Some(token) = tokens.get(i) {
        if token.kind != TokenKind::Attribute { break; }
        if token.prefix.length == 0 && span_eq(source, token.local_name, name) { return Some(token.value); }
        i += 1;
    }
    None
}

fn inherited(property: Property) -> bool {
    !matches!(property, Property::ClipPath | Property::Opacity | Property::Display | Property::Filter |
        Property::Mask | Property::MixBlendMode | Property::Isolation | Property::VectorEffect)
}
fn keyword(value: &[u8], word: &[u8]) -> bool { value.eq_ignore_ascii_case(word) }

fn is_space(c: u8) -> bool { matches!(c, b' ' | b'\t' | b'\r' | b'\n' | 0x0c) }
fn ident(c: u8) -> bool { c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-') || c >= 0x80 }
fn skip_space_comments(source: &[u8], i: &mut usize, end: usize) {
    loop {
        while *i < end && is_space(source[*i]) { *i += 1; }
        if *i + 1 >= end || source[*i..*i + 2] != *b"/*" { break; }
        *i = comment_end(source, *i, end);
    }
}
fn comment_end(source: &[u8], start: usize, end: usize) -> usize {
    let mut i = start.saturating_add(2);
    while i + 1 < end && source[i..i + 2] != *b"*/" { i += 1; }
    if i + 1 < end { i + 2 } else { end }
}
fn find_unquoted(source: &[u8], mut i: usize, end: usize, wanted: u8) -> Option<usize> {
    let mut quote = 0;
    while i < end {
        let c = source[i];
        if quote != 0 { if c == b'\\' { i = (i + 2).min(end); continue; } if c == quote { quote = 0; } }
        else if c == b'\'' || c == b'"' { quote = c; }
        else if i + 1 < end && source[i..i + 2] == *b"/*" { i = comment_end(source, i, end); continue; }
        else if c == wanted { return Some(i); }
        i += 1;
    }
    None
}
fn matching_brace(source: &[u8], mut i: usize, end: usize) -> Option<usize> {
    let mut depth = 1usize; let mut quote = 0;
    while i < end {
        let c = source[i];
        if quote != 0 { if c == b'\\' { i = (i + 2).min(end); continue; } if c == quote { quote = 0; } }
        else if c == b'\'' || c == b'"' { quote = c; }
        else if i + 1 < end && source[i..i + 2] == *b"/*" { i = comment_end(source, i, end); continue; }
        else if c == b'{' { depth = depth.checked_add(1)?; }
        else if c == b'}' { depth -= 1; if depth == 0 { return Some(i); } }
        i += 1;
    }
    None
}

fn skip_at_rule(source: &[u8], mut i: usize, end: usize) -> usize {
    let mut quote = 0; let mut parens = 0usize;
    while i < end {
        let c = source[i];
        if quote != 0 { if c == b'\\' { i = (i + 2).min(end); continue; } if c == quote { quote = 0; } }
        else if c == b'\'' || c == b'"' { quote = c; }
        else if i + 1 < end && source[i..i + 2] == *b"/*" { i = comment_end(source, i, end); continue; }
        else if c == b'(' { parens += 1; }
        else if c == b')' { parens = parens.saturating_sub(1); }
        else if parens == 0 && c == b';' { return i + 1; }
        else if parens == 0 && c == b'{' { return matching_brace(source, i + 1, end).map_or(end, |close| close + 1); }
        i += 1;
    }
    end
}

fn parse_declarations(source: &[u8], span: XmlSpan, output: &mut Vec<Declaration>, hit_limit: &mut bool) -> Result<bool, CssError> {
    let end = span.end().ok_or(CssError::Limit)?;
    checked_slice(source, span)?;
    let mut i = span.offset; let mut unsupported = false;
    while i < end {
        skip_space_comments(source, &mut i, end);
        if i >= end { break; }
        let start = i; let mut colon = None; let mut quote = 0; let mut parens = 0usize;
        while i < end {
            let c = source[i];
            if quote != 0 { if c == b'\\' { i = (i + 2).min(end); continue; } if c == quote { quote = 0; } }
            else if c == b'\'' || c == b'"' { quote = c; }
            else if i + 1 < end && source[i..i + 2] == *b"/*" { i = comment_end(source, i, end); continue; }
            else if c == b'(' { parens = parens.saturating_add(1); }
            else if c == b')' { parens = parens.saturating_sub(1); }
            else if c == b':' && colon.is_none() { colon = Some(i); }
            else if c == b';' && parens == 0 { break; }
            i += 1;
        }
        let segment_end = i;
        if i < end { i += 1; }
        let Some(colon) = colon.filter(|c| *c < segment_end) else { continue };
        let mut ns = start; let mut ne = colon;
        while ns < ne && is_space(source[ns]) { ns += 1; }
        while ne > ns && is_space(source[ne - 1]) { ne -= 1; }
        let mut vs = colon + 1; let mut ve = segment_end;
        trim_value(source, &mut vs, &mut ve);
        if ns == ne || vs == ve { continue; }
        let name = &source[ns..ne];
        let Some(property) = Property::ALL.iter().copied().find(|p| name.eq_ignore_ascii_case(p.name())) else { unsupported = true; continue };
        let (value_end, important) = strip_important(source, vs, ve);
        ve = value_end;
        if ve <= vs { continue; }
        if output.len() >= MAX_DECLARATIONS { *hit_limit = true; return Err(CssError::Limit); }
        output.try_reserve(1).map_err(|_| CssError::OutOfMemory)?;
        output.push(Declaration { property, value: XmlSpan { offset: vs, length: ve - vs }, important });
    }
    Ok(unsupported)
}
fn trim_value(source: &[u8], start: &mut usize, end: &mut usize) {
    loop { while *start < *end && is_space(source[*start]) { *start += 1; }
        if *start + 1 < *end && source[*start..*start + 2] == *b"/*" { *start = comment_end(source, *start, *end); } else { break; } }
    loop { while *end > *start && is_space(source[*end - 1]) { *end -= 1; }
        if *end >= *start + 2 && source[*end - 2..*end] == *b"*/" {
            let mut i = *end - 2; while i > *start && source[i - 1..i + 1] != *b"/*" { i -= 1; }
            if i > *start { *end = i - 1; } else { break; }
        } else { break; } }
}
fn strip_important(source: &[u8], start: usize, end: usize) -> (usize, bool) {
    let mut mark = end; while mark > start && is_space(source[mark - 1]) { mark -= 1; }
    let mut word = mark; while word > start && ident(source[word - 1]) { word -= 1; }
    let mut bang = word; while bang > start && is_space(source[bang - 1]) { bang -= 1; }
    if bang > start && source[bang - 1] == b'!' && source[word..mark].eq_ignore_ascii_case(b"important")
        && !inside_string(source, start, bang - 1) {
        let mut value_end = bang - 1; while value_end > start && is_space(source[value_end - 1]) { value_end -= 1; }
        (value_end, true)
    } else { (end, false) }
}
fn inside_string(source: &[u8], start: usize, end: usize) -> bool {
    let mut i = start; let mut quote = 0;
    while i < end {
        let c = source[i];
        if quote != 0 {
            if c == b'\\' { i = (i + 2).min(end); continue; }
            if c == quote { quote = 0; }
        } else if c == b'\'' || c == b'"' { quote = c; }
        else if i + 1 < end && source[i..i + 2] == *b"/*" { i = comment_end(source, i, end); continue; }
        i += 1;
    }
    quote != 0
}

fn parse_selector(source: &[u8], mut start: usize, mut end: usize) -> Option<Selector> {
    while end > start && is_space(source[end - 1]) { end -= 1; }
    while start < end && is_space(source[start]) { start += 1; }
    let mut result = Selector::EMPTY; let mut i = start;
    while i < end {
        if result.count == MAX_SELECTOR_TERMS { return None; }
        let mut term = Term::EMPTY;
        if source[i] != b'#' && source[i] != b'.' {
            let begin = i; while i < end && ident(source[i]) { i += 1; }
            if begin == i { return None; }
            term.element = Some(XmlSpan { offset: begin, length: i - begin }); term.specificity += 1;
        }
        while i < end && matches!(source[i], b'#' | b'.') {
            let marker = source[i]; i += 1; let begin = i; while i < end && ident(source[i]) { i += 1; }
            if begin == i { return None; }
            let span = XmlSpan { offset: begin, length: i - begin };
            if marker == b'#' && term.id.is_none() { term.id = Some(span); term.specificity += 10_000; }
            else if marker == b'.' && term.class_count < MAX_CLASSES { term.classes[term.class_count] = span; term.class_count += 1; term.specificity += 100; }
            else { return None; }
        }
        if term.element.is_none() && term.id.is_none() && term.class_count == 0 { return None; }
        result.specificity = result.specificity.checked_add(term.specificity)?;
        result.terms[result.count] = term; result.count += 1;
        let before = i; while i < end && is_space(source[i]) { i += 1; }
        if i < end && (i == before || matches!(source[i], b'>' | b'+' | b'~')) { return None; }
    }
    (result.count > 0).then_some(result)
}

fn matches_selector(source: &[u8], selector: &Selector, tokens: &[XmlToken], element: usize,
                    selector_visits: &mut usize, hit_limit: &mut bool) -> Result<bool, CssError> {
    if selector.count == 0 || !matches_term(source, &selector.terms[selector.count - 1], tokens, element) { return Ok(false); }
    let mut descendant = element;
    for term in (0..selector.count - 1).rev() {
        let depth = tokens[descendant].depth; let mut found = None;
        for candidate in (0..descendant).rev() {
            if !is_element(&tokens[candidate]) || tokens[candidate].depth >= depth { continue; }
            if *selector_visits >= MAX_RULE_MATCHES { *hit_limit = true; return Err(CssError::Limit); }
            *selector_visits += 1;
            if matches_term(source, &selector.terms[term], tokens, candidate) { found = Some(candidate); break; }
        }
        let Some(parent) = found else { return Ok(false) }; descendant = parent;
    }
    Ok(true)
}
fn matches_term(source: &[u8], term: &Term, tokens: &[XmlToken], element: usize) -> bool {
    let token = &tokens[element];
    if let Some(name) = term.element { if !span_eq(source, name, checked_slice(source, token.local_name).unwrap_or_default()) { return false; } }
    if let Some(id) = term.id {
        let Some(value) = attribute(source, tokens, element, b"id") else { return false };
        if !same_span(source, value, id) { return false; }
    }
    if term.class_count > 0 {
        let Some(value) = attribute(source, tokens, element, b"class") else { return false };
        for class in &term.classes[..term.class_count] { if !has_class(source, value, *class) { return false; } }
    }
    true
}
fn same_span(source: &[u8], a: XmlSpan, b: XmlSpan) -> bool { checked_slice(source, a).ok() == checked_slice(source, b).ok() }
fn has_class(source: &[u8], span: XmlSpan, wanted: XmlSpan) -> bool {
    let (Ok(value), Ok(class)) = (checked_slice(source, span), checked_slice(source, wanted)) else { return false };
    let mut i = 0; let mut count = 0;
    while i < value.len() { while i < value.len() && is_space(value[i]) { i += 1; } if i == value.len() { break; }
        count += 1; if count > 256 { return false; }
        let start = i; while i < value.len() && !is_space(value[i]) { i += 1; }
        if &value[start..i] == class { return true; }
    }
    false
}

