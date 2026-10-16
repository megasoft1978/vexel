use vexel_svg_css::{is_style_element, CssError, CssPaint, CssSheet, Property, MAX_BYTES, MAX_DECLARATIONS, MAX_RULES};
use vexel_xml::{tokenize_large, TokenKind, XmlError, XmlSpan};

fn parse_tokens(source: &[u8]) -> Vec<vexel_xml::XmlToken> {
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, XmlError::None);
    tokens
}

fn element(source: &[u8], tokens: &[vexel_xml::XmlToken], local: &[u8]) -> usize {
    tokens.iter().position(|token| {
        matches!(token.kind, TokenKind::StartElement | TokenKind::EmptyElement)
            && token.local_name.slice(source) == Some(local)
    }).expect("element")
}

fn value<'a>(paint: &CssPaint<'a>, property: Property) -> Option<&'a [u8]> {
    paint.value(property).map(|v| v.bytes)
}

#[test]
fn all_properties_cascade_and_source_spans_are_preserved() {
    let declarations = concat!(
        "fill:red;fill-rule:evenodd;stroke:blue;stroke-width:2;stroke-linecap:round;",
        "stroke-linejoin:bevel;stroke-miterlimit:5;stroke-dasharray:2 3;stroke-dashoffset:4;",
        "clip-path:none;stop-color:green;stop-opacity:1;fill-opacity:.5;stroke-opacity:.25;",
        "opacity:.75;visibility:hidden;display:inline;color:purple;font-family:serif;",
        "font-size:12px;font-weight:700;font-style:italic;text-anchor:middle;letter-spacing:1px;",
        "word-spacing:2px;filter:none;mask:none;mix-blend-mode:multiply;isolation:isolate;",
        "vector-effect:non-scaling-stroke;shape-rendering:crispEdges;paint-order:stroke fill;",
        "marker:none;marker-start:none;marker-mid:none;marker-end:none;mask-type:alpha;",
        "color-interpolation-filters:sRGB;clip-rule:evenodd"
    );
    let source = format!("<svg><style>rect{{{declarations}}}</style><rect/></svg>");
    let source = source.as_bytes();
    let tokens = parse_tokens(source);
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let paint = sheet.compute(rect, None).unwrap();
    let expected: [(Property, &[u8]); 39] = [
        (Property::Fill, b"red"), (Property::FillRule, b"evenodd"),
        (Property::Stroke, b"blue"), (Property::StrokeWidth, b"2"),
        (Property::StrokeLinecap, b"round"), (Property::StrokeLinejoin, b"bevel"),
        (Property::StrokeMiterlimit, b"5"), (Property::StrokeDasharray, b"2 3"),
        (Property::StrokeDashoffset, b"4"), (Property::ClipPath, b"none"),
        (Property::StopColor, b"green"), (Property::StopOpacity, b"1"),
        (Property::FillOpacity, b".5"), (Property::StrokeOpacity, b".25"),
        (Property::Opacity, b".75"), (Property::Visibility, b"hidden"),
        (Property::Display, b"inline"), (Property::Color, b"purple"),
        (Property::FontFamily, b"serif"), (Property::FontSize, b"12px"),
        (Property::FontWeight, b"700"), (Property::FontStyle, b"italic"),
        (Property::TextAnchor, b"middle"), (Property::LetterSpacing, b"1px"),
        (Property::WordSpacing, b"2px"), (Property::Filter, b"none"),
        (Property::Mask, b"none"), (Property::MixBlendMode, b"multiply"),
        (Property::Isolation, b"isolate"), (Property::VectorEffect, b"non-scaling-stroke"),
        (Property::ShapeRendering, b"crispEdges"), (Property::PaintOrder, b"stroke fill"),
        (Property::Marker, b"none"), (Property::MarkerStart, b"none"),
        (Property::MarkerMid, b"none"), (Property::MarkerEnd, b"none"),
        (Property::MaskType, b"alpha"), (Property::ColorInterpolationFilters, b"sRGB"),
        (Property::ClipRule, b"evenodd"),
    ];
    assert_eq!(Property::ALL.len(), expected.len());
    for (property, expected) in expected {
        assert_eq!(value(&paint, property), Some(expected), "wrong {:?}", property);
    }
    let found = paint.value(Property::Fill).unwrap();
    assert_eq!(found.bytes, b"red");
    assert_eq!(&source[found.span.offset..found.span.offset + found.span.length], found.bytes);
}

#[test]
fn original_c_css_fixture_semantics_are_preserved_exactly() {
    let source = b"<svg width=\"8\" height=\"8\"><style>/* cascade */\nrect, circle { FiLl: red; fill-rule: evenodd; stroke: /* leading */ blue /* trailing */; stroke-width: 2; stroke-linecap: round; stroke-linejoin: bevel; stroke-miterlimit: 5; stroke-dasharray: 2 3; stroke-dashoffset: 4; clip-rule:evenodd; }svg g rect.descendant { clip-path:none; stop-color:currentColor; stop-opacity:1; fill-opacity:50%; stroke-opacity:.25; opacity:.75; visibility:hidden; display:inline; color:hsl(240,100%,50%); }#target.paint { fill: green !important; } .other { fill: white; }</style><g><rect id=\"target\" class=\"paint otherish descendant\" fill=\"white\" style=\"fill: blue; stroke-dashoffset: 7\" width=\"2\" height=\"2\"/></g></svg>";
    let tokens = parse_tokens(source);
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let paint = sheet.compute(rect, None).unwrap();
    let expected: [(Property, &[u8]); 19] = [
        (Property::Fill, b"green"), (Property::FillRule, b"evenodd"),
        (Property::Stroke, b"blue"), (Property::StrokeWidth, b"2"),
        (Property::StrokeLinecap, b"round"), (Property::StrokeLinejoin, b"bevel"),
        (Property::StrokeMiterlimit, b"5"), (Property::StrokeDasharray, b"2 3"),
        (Property::StrokeDashoffset, b"7"), (Property::ClipPath, b"none"),
        (Property::StopColor, b"currentColor"), (Property::StopOpacity, b"1"),
        (Property::FillOpacity, b"50%"), (Property::StrokeOpacity, b".25"),
        (Property::Opacity, b".75"), (Property::Visibility, b"hidden"),
        (Property::Display, b"inline"), (Property::Color, b"hsl(240,100%,50%)"),
        (Property::ClipRule, b"evenodd"),
    ];
    for (property, expected) in expected {
        assert_eq!(value(&paint, property), Some(expected), "wrong {:?}", property);
    }
}

#[test]
fn equal_length_sources_cannot_cross_contaminate_sheet_or_parent_paint() {
    let source_a = b"<svg><style>g{fill:red} rect{fill:inherit}</style><g><rect/></g></svg>";
    let source_b = b"<svg><style>g{fill:blu} rect{fill:inherit}</style><g><rect/></g></svg>";
    assert_eq!(source_a.len(), source_b.len());
    let tokens_a = parse_tokens(source_a);
    let tokens_b = parse_tokens(source_b);
    let group_a = element(source_a, &tokens_a, b"g");
    let group_b = element(source_b, &tokens_b, b"g");
    let rect_a = element(source_a, &tokens_a, b"rect");
    let mut sheet_a = CssSheet::create(source_a, &tokens_a).unwrap();
    let mut sheet_b = CssSheet::create(source_b, &tokens_b).unwrap();
    let parent_a = sheet_a.compute(group_a, None).unwrap();
    let parent_b = sheet_b.compute(group_b, None).unwrap();
    assert_eq!(value(&parent_a, Property::Fill), Some(&b"red"[..]));
    assert_eq!(value(&parent_b, Property::Fill), Some(&b"blu"[..]));
    assert_eq!(sheet_a.compute(rect_a, Some(&parent_b)), Err(CssError::Limit));
    assert_eq!(value(&sheet_a.compute(rect_a, Some(&parent_a)).unwrap(), Property::Fill), Some(&b"red"[..]));
}

#[test]
fn cascade_order_specificity_inheritance_and_css_wide_keywords() {
    let source = b"<svg><style>rect{fill:red;stroke:blue} #x.k{fill:green!important} rect.k{stroke:black}</style><g fill='white' stroke='cyan'><rect id='x' class='k' fill='yellow' style='fill:purple!important;stroke:inherit'/></g></svg>";
    let tokens = parse_tokens(source);
    let group = element(source, &tokens, b"g");
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let parent = sheet.compute(group, None).unwrap();
    let paint = sheet.compute(rect, Some(&parent)).unwrap();
    assert_eq!(value(&paint, Property::Fill), Some(&b"purple"[..]));
    assert_eq!(value(&paint, Property::Stroke), Some(&b"cyan"[..]));
    assert_eq!(paint.value(Property::Opacity), None);

    let source = b"<svg><style>rect{fill:initial;stroke:unset}</style><g fill='white' stroke='red'><rect/></g></svg>";
    let tokens = parse_tokens(source);
    let group = element(source, &tokens, b"g");
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let parent = sheet.compute(group, None).unwrap();
    let paint = sheet.compute(rect, Some(&parent)).unwrap();
    assert_eq!(paint.value(Property::Fill), None);
    assert_eq!(value(&paint, Property::Stroke), Some(&b"red"[..]));
}

#[test]
fn selector_chains_compound_classes_namespaces_and_style_cdata() {
    let source = b"<svg xmlns:x='urn:x'><style><![CDATA[svg g rect.a.b{fill:blue} x:rect{stroke:red}]]></style><g><rect class='a b'/><x:rect class='a b'/></g></svg>";
    let tokens = parse_tokens(source);
    let style = tokens.iter().find(|token| token.kind == TokenKind::StartElement && is_style_element(source, token)).unwrap();
    assert_eq!(style.local_name.slice(source), Some(&b"style"[..]));
    let rect = element(source, &tokens, b"rect");
    let prefixed = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement && t.prefix.length > 0).unwrap();
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let paint = sheet.compute(rect, None).unwrap();
    assert_eq!(value(&paint, Property::Fill), Some(&b"blue"[..]));
    let prefixed_paint = sheet.compute(prefixed, None).unwrap();
    assert_eq!(prefixed_paint.value(Property::Stroke), None);
}

#[test]
fn cdata_styles_are_an_explicit_additive_compatibility_and_preserve_unknown_source() {
    let source = b"<svg><style><![CDATA[stop{unknown-inkscape-property: untouched}]]></style><stop/></svg>";
    let before = source.to_vec();
    let tokens = parse_tokens(source);
    let stop = element(source, &tokens, b"stop");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    assert_eq!(sheet.compute(stop, None), Err(CssError::Unsupported));
    assert_eq!(source, before.as_slice());
    assert_eq!(sheet.css_bytes(), b"rect{unknown-inkscape-property: untouched}".len());
}

#[test]
fn clip_rule_inherits_and_accepts_presentation_inline_and_stylesheet_sources() {
    let source = b"<svg><style>.even{clip-rule:evenodd}</style><g clip-rule='evenodd'><path/><path class='even' style='clip-rule:inherit'/></g></svg>";
    let tokens = parse_tokens(source);
    let group = element(source, &tokens, b"g");
    let paths: Vec<_> = tokens.iter().enumerate().filter_map(|(i, token)|
        (token.kind == TokenKind::EmptyElement && token.local_name.slice(source) == Some(&b"path"[..])).then_some(i)
    ).collect();
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let parent = sheet.compute(group, None).unwrap();
    for path in paths {
        let paint = sheet.compute(path, Some(&parent)).unwrap();
        assert_eq!(value(&paint, Property::ClipRule), Some(&b"evenodd"[..]));
    }
}

#[test]
fn unsupported_selectors_at_rules_malformed_rules_and_unknown_stop_style() {
    let source = b"<svg><style>@media screen{rect{fill:red}} @import url(x); rect:hover{fill:blue} rect > path{fill:green} rect{stroke:'a;b';font-family:'!important';fill:/*c*/red/*d*/}</style><rect/></svg>";
    let tokens = parse_tokens(source);
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    let paint = sheet.compute(rect, None).unwrap();
    assert_eq!(value(&paint, Property::Stroke), Some(&b"'a;b'"[..]));
    assert_eq!(value(&paint, Property::FontFamily), Some(&b"'!important'"[..]));
    assert_eq!(value(&paint, Property::Fill), Some(&b"red"[..]));

    let source = b"<svg><style>stop{unknown-property:red}</style><stop/></svg>";
    let tokens = parse_tokens(source);
    let stop = element(source, &tokens, b"stop");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    assert_eq!(sheet.compute(stop, None), Err(CssError::Unsupported));
}

#[test]
fn rule_match_quota_is_stateful_and_reaches_the_exact_limit() {
    let source = b"<svg><style>rect{fill:red}</style><rect/></svg>";
    let tokens = parse_tokens(source);
    let rect = element(source, &tokens, b"rect");
    let mut sheet = CssSheet::create(source, &tokens).unwrap();
    for _ in 0..vexel_svg_css::MAX_RULE_MATCHES {
        assert!(sheet.compute(rect, None).is_ok());
    }
    assert_eq!(sheet.compute(rect, None), Err(CssError::Limit));
}

#[test]
fn limits_and_unbounded_containing_svg_source() {
    let declarations = format!("{}", (0..MAX_DECLARATIONS + 1).map(|_| "fill:red;").collect::<String>());
    let source = format!("<svg><style>rect{{{declarations}}}</style><rect/></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert!(matches!(CssSheet::create(source.as_bytes(), &tokens), Err(CssError::Limit)));

    let padding = " ".repeat(16 * 1024 * 1024);
    let source = format!("<svg><!--{padding}--><style>rect{{fill:red}}</style><rect/></svg>");
    let (tokens, result) = tokenize_large(source.as_bytes());
    assert_eq!(result.diagnostic.category, XmlError::None);
    let rect = element(source.as_bytes(), &tokens, b"rect");
    let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
    assert_eq!(sheet.compute(rect, None).unwrap().value(Property::Fill).unwrap().bytes, b"red");
    assert!(source.len() > 16 * 1024 * 1024 && sheet.css_bytes() < MAX_BYTES);
}

#[test]
fn css_aggregate_and_rule_limits_are_exact() {
    let css = "rect{fill:red}".repeat(MAX_RULES);
    let source = format!("<svg><style>{css}</style><rect/></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert_eq!(CssSheet::create(source.as_bytes(), &tokens).unwrap().rule_count(), MAX_RULES);

    let declarations = "fill:red;".repeat(MAX_DECLARATIONS);
    let source = format!("<svg><style>rect{{{declarations}}}</style></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert_eq!(CssSheet::create(source.as_bytes(), &tokens).unwrap().rule_count(), 1);

    let css = format!("{}", (0..MAX_RULES + 1).map(|_| "rect{fill:red}").collect::<String>());
    let source = format!("<svg><style>{css}</style><rect/></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert!(matches!(CssSheet::create(source.as_bytes(), &tokens), Err(CssError::Limit)));

    let css = " ".repeat(MAX_BYTES);
    let source = format!("<svg><style>{css}</style></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert_eq!(CssSheet::create(source.as_bytes(), &tokens).unwrap().css_bytes(), MAX_BYTES);

    let css = " ".repeat(MAX_BYTES + 1);
    let source = format!("<svg><style>{css}</style></svg>");
    let tokens = parse_tokens(source.as_bytes());
    assert!(matches!(CssSheet::create(source.as_bytes(), &tokens), Err(CssError::Limit)));
}

#[test]
fn selector_term_and_class_bounds_reject_only_the_overflowing_selectors() {
    let classes = (0..17).map(|i| format!(".c{i}")).collect::<String>();
    let source = format!("<svg><style>g svg g rect{{fill:blue}} svg g svg g rect{{stroke:red}} rect{classes}{{fill:green}}</style><g><svg><g><rect class='{classes}'/></g></svg></g></svg>");
    let tokens = parse_tokens(source.as_bytes());
    let rect = element(source.as_bytes(), &tokens, b"rect");
    let mut sheet = CssSheet::create(source.as_bytes(), &tokens).unwrap();
    let paint = sheet.compute(rect, None).unwrap();
    assert_eq!(value(&paint, Property::Fill), Some(&b"blue"[..]));
    assert_eq!(paint.value(Property::Stroke), None);
}

#[test]
fn keyword_and_initial_paint_api_are_safe_for_invalid_spans() {
    let paint = CssPaint::initial();
    assert!(Property::ALL.iter().all(|property| paint.value(*property).is_none()));
    assert!(!vexel_svg_css::keyword_equals(b"red", XmlSpan { offset: usize::MAX, length: 1 }, b"red"));
}
