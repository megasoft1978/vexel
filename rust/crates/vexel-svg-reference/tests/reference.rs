use vexel_svg_reference::{
    attributes_supported, definition_end, use_attributes_supported, xlink_is_bound,
    ReferenceDiagnostic, ReferenceError, ReferenceIndex, MAX_REFERENCE_IDS,
    REFERENCE_SLOT_CAPACITY,
};
use vexel_xml::{tokenize_large, TokenKind, XmlSpan, XmlToken};

fn parsed(source: &[u8]) -> Vec<XmlToken> {
    let (tokens, result) = tokenize_large(source);
    assert_eq!(result.diagnostic.category, vexel_xml::XmlError::None, "{result:?}");
    tokens
}

fn attr<'a>(source: &'a [u8], tokens: &[XmlToken], element: usize, local: &[u8]) -> XmlSpan {
    tokens[element + 1..].iter().take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.local_name.slice(source) == Some(local) && t.prefix.length == 0)
        .expect("expected attribute").value
}

#[test]
fn empty_index_has_no_slot_storage_and_lookup_is_a_miss() {
    let source = b"<svg/>";
    let tokens = parsed(source);
    let mut index = ReferenceIndex::create(source, &tokens).unwrap();
    assert_eq!(index.id_count(), 0);
    assert_eq!(index.storage_bytes(), 0);
    assert_eq!(index.lookup_probes(), 0);
    assert_eq!(index.find(XmlSpan { offset: 0, length: 1 }), None);
    assert_eq!(index.lookup_probes(), 0);
}

#[test]
fn indexed_raw_ids_duplicate_detection_and_probe_accounting_match_contract() {
    let source = b"<svg><g id='a&amp;b'/><rect id='a&#38;b'/></svg>";
    let tokens = parsed(source);
    let raw = attr(source, &tokens, 1, b"id");
    let other = attr(source, &tokens, 3, b"id");
    let mut index = ReferenceIndex::create(source, &tokens).unwrap();
    assert_eq!(index.id_count(), 2);
    assert_eq!(index.storage_bytes(), REFERENCE_SLOT_CAPACITY * core::mem::size_of::<u32>());
    assert_eq!(index.find(raw), Some(1));
    assert_eq!(index.find(other), Some(3));
    let before_miss = index.lookup_probes();
    assert_eq!(index.find(XmlSpan { offset: 0, length: 4 }), None);
    assert!(index.lookup_probes() > before_miss);
    assert_eq!(index.find(XmlSpan { offset: usize::MAX, length: 1 }), None);
    assert_eq!(index.find(XmlSpan { offset: raw.offset, length: 0 }), None);

    assert_ne!(raw.slice(source), other.slice(source));
}

#[test]
fn empty_and_duplicate_ids_fail_without_publishing_a_partial_index() {
    for (source, expected, offset) in [
        (&b"<svg><g id=''/></svg>"[..], ReferenceError::Unsupported, 12usize),
        (&b"<svg><g id='x'/><g id='x'/></svg>"[..], ReferenceError::Unsupported, 23usize),
    ] {
        let tokens = parsed(source);
        let result = ReferenceIndex::create(source, &tokens);
        let error = result.unwrap_err();
        assert_eq!(error.category, expected);
        assert_eq!(error.offset, offset);
    }
}

#[test]
fn exact_hash_collisions_probe_until_the_raw_key_matches() {
    let mut by_bucket = std::collections::BTreeMap::<usize, Vec<String>>::new();
    let mut collision = None;
    for n in 0..100_000usize {
        let id = format!("key{n}");
        let bucket = id.bytes().fold(1_469_598_103_934_665_603u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        }) as usize & (REFERENCE_SLOT_CAPACITY - 1);
        let ids = by_bucket.entry(bucket).or_default();
        ids.push(id);
        if ids.len() == 3 { collision = Some(ids.clone()); break; }
    }
    let ids = collision.expect("pigeonhole search finds a bucket with three keys");
    let mut source = String::from("<svg>");
    for id in &ids { source.push_str(&format!("<g id='{id}'/>")); }
    source.push_str("</svg>");
    let tokens = parsed(source.as_bytes());
    let spans: Vec<_> = [1usize, 3, 5].iter().map(|&element| attr(source.as_bytes(), &tokens, element, b"id")).collect();
    let mut index = ReferenceIndex::create(source.as_bytes(), &tokens).unwrap();
    for (expected, span) in [1usize, 3, 5].into_iter().zip(spans) {
        assert_eq!(index.find(span), Some(expected));
    }
    assert!(index.lookup_probes() >= 6);
}

#[test]
fn id_cap_accepts_exactly_the_limit_and_rejects_the_next_identifier() {
    let mut source = String::from("<svg>");
    for n in 0..MAX_REFERENCE_IDS { source.push_str(&format!("<g id='i{n}'/>")); }
    source.push_str("</svg>");
    let tokens = parsed(source.as_bytes());
    let index = ReferenceIndex::create(source.as_bytes(), &tokens).unwrap();
    assert_eq!(index.id_count(), MAX_REFERENCE_IDS);
    assert_eq!(index.storage_bytes(), REFERENCE_SLOT_CAPACITY * size_of::<u32>());

    source.insert_str(source.len() - b"</svg>".len(), "<g id='overflow'/>");
    let tokens = parsed(source.as_bytes());
    let error = ReferenceIndex::create(source.as_bytes(), &tokens).unwrap_err();
    assert_eq!(error.category, ReferenceError::Limit);
    assert!(error.offset < source.len());
}

#[test]
fn malformed_token_spans_are_rejected_before_index_creation_or_helpers_read_them() {
    let source = b"<svg><g id='x'/></svg>";
    let mut tokens = parsed(source);
    let attr_index = tokens.iter().position(|token| token.kind == TokenKind::Attribute).unwrap();
    tokens[attr_index].value = XmlSpan { offset: usize::MAX, length: 1 };
    assert_eq!(ReferenceIndex::create(source, &tokens).unwrap_err(), ReferenceDiagnostic {
        category: ReferenceError::InvalidArgument, offset: usize::MAX,
    });
    assert_eq!(definition_end(source, &tokens, 0), None);
    assert!(!xlink_is_bound(source, &tokens));
    assert!(!use_attributes_supported(source, &tokens, 1));
}

#[test]
fn definition_end_matches_supported_containers_and_depth() {
    let source = b"<svg><defs><g><defs/><symbol><path/></symbol></g></defs><marker/><a/></svg>";
    let tokens = parsed(source);
    let outer = tokens.iter().position(|t| t.kind == TokenKind::StartElement && t.local_name.slice(source) == Some(b"defs")).unwrap();
    let close = tokens.iter().position(|t| t.kind == TokenKind::EndElement && t.local_name.slice(source) == Some(b"defs")).unwrap();
    assert_eq!(definition_end(source, &tokens, outer), Some(close));
    let empty = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement && t.local_name.slice(source) == Some(b"marker")).unwrap();
    assert_eq!(definition_end(source, &tokens, empty), Some(empty));
    let ordinary = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement && t.local_name.slice(source) == Some(b"a")).unwrap();
    assert_eq!(definition_end(source, &tokens, ordinary), None);
}

#[test]
fn attribute_allowlists_preserve_root_shape_namespace_and_foreign_rules() {
    let source = b"<svg xmlns:xlink='http://www.w3.org/1999/xlink' width='8' foreign:label='kept'><rect x='1' width='2' foreign:label='kept'/><path x='1'/></svg>";
    let tokens = parsed(source);
    let root = tokens.iter().position(|t| t.kind == TokenKind::StartElement).unwrap();
    let rect = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement && t.local_name.slice(source) == Some(b"rect")).unwrap();
    let path = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement && t.local_name.slice(source) == Some(b"path")).unwrap();
    let rect_kind = tokens[rect].local_name;
    let path_kind = tokens[path].local_name;
    assert!(attributes_supported(source, &tokens, root, true, tokens[root].local_name));
    assert!(attributes_supported(source, &tokens, rect, false, rect_kind));
    assert!(!attributes_supported(source, &tokens, path, false, path_kind));

    let bad = b"<svg><rect svg:width='2'/></svg>";
    let bad_tokens = parsed(bad);
    let rect = bad_tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
    assert!(!attributes_supported(bad, &bad_tokens, rect, false, bad_tokens[rect].local_name));
}

#[test]
fn every_legacy_common_and_use_attribute_name_remains_supported() {
    let common = [
        "fill", "fill-rule", "transform", "clip-path", "filter", "mask",
        "mix-blend-mode", "isolation", "vector-effect", "shape-rendering",
        "paint-order", "marker", "marker-start", "marker-mid", "marker-end",
        "mask-type", "color-interpolation-filters", "systemLanguage",
        "requiredFeatures", "requiredExtensions", "fill-opacity", "stroke-opacity",
        "opacity", "visibility", "display", "color", "stroke", "stroke-width",
        "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-dasharray",
        "stroke-dashoffset", "font-family", "font-size", "font-weight", "font-style",
        "text-anchor", "letter-spacing", "word-spacing",
    ];
    let root_only = ["xmlns", "version", "width", "height", "viewBox", "preserveAspectRatio"];
    let use_allowed = [
        "href", "id", "class", "style", "x", "y", "width", "height",
        "preserveAspectRatio", "fill", "fill-rule", "fill-opacity", "stroke-opacity",
        "opacity", "visibility", "display", "color", "transform", "clip-path",
        "filter", "mask", "mix-blend-mode", "isolation", "vector-effect",
        "shape-rendering", "paint-order", "marker", "marker-start", "marker-mid",
        "marker-end", "mask-type", "color-interpolation-filters", "stroke",
        "stroke-width", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit",
        "stroke-dasharray", "stroke-dashoffset", "font-family", "font-size",
        "font-weight", "font-style", "text-anchor", "letter-spacing", "word-spacing",
    ];
    for name in common {
        let source = format!("<svg {name}='v'/>");
        let tokens = parsed(source.as_bytes());
        let root = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
        assert!(attributes_supported(source.as_bytes(), &tokens, root, true, tokens[root].local_name), "root {name}");
    }
    for name in root_only {
        let source = format!("<svg {name}='v'/>");
        let tokens = parsed(source.as_bytes());
        let root = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
        assert!(attributes_supported(source.as_bytes(), &tokens, root, true, tokens[root].local_name), "root {name}");
    }
    for name in use_allowed {
        let source = format!("<svg><use {name}='v'/></svg>");
        let tokens = parsed(source.as_bytes());
        let use_index = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
        assert!(use_attributes_supported(source.as_bytes(), &tokens, use_index), "use {name}");
    }
    for name in ["id", "class", "style"] {
        let source = format!("<svg {name}='v'><use {name}='v'/></svg>");
        let tokens = parsed(source.as_bytes());
        let root = tokens.iter().position(|t| t.kind == TokenKind::StartElement).unwrap();
        let use_index = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
        assert!(attributes_supported(source.as_bytes(), &tokens, root, true, tokens[root].local_name));
        assert!(use_attributes_supported(source.as_bytes(), &tokens, use_index));
    }
}

#[test]
fn xlink_binding_uses_root_namespace_and_use_allowlist_is_complete() {
    let good = b"<svg xmlns:xlink='http://www.w3.org/1999/xlink'><use xlink:href='#x' transform='translate(1 2)'/></svg>";
    let tokens = parsed(good);
    let use_index = tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
    assert!(xlink_is_bound(good, &tokens));
    assert!(use_attributes_supported(good, &tokens, use_index));

    let wrong_uri = b"<svg xmlns:xlink='urn:wrong'><use xlink:href='#x'/></svg>";
    let wrong_tokens = parsed(wrong_uri);
    let use_index = wrong_tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
    assert!(!xlink_is_bound(wrong_uri, &wrong_tokens));
    assert!(!use_attributes_supported(wrong_uri, &wrong_tokens, use_index));

    let nested_binding = b"<svg><g xmlns:xlink='http://www.w3.org/1999/xlink'><use xlink:href='#x'/></g></svg>";
    let nested_tokens = parsed(nested_binding);
    let nested_use = nested_tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
    assert!(!xlink_is_bound(nested_binding, &nested_tokens));
    assert!(!use_attributes_supported(nested_binding, &nested_tokens, nested_use));

    let bad_attr = b"<svg><use href='#x' onload='bad'/></svg>";
    let bad_tokens = parsed(bad_attr);
    let use_index = bad_tokens.iter().position(|t| t.kind == TokenKind::EmptyElement).unwrap();
    assert!(!use_attributes_supported(bad_attr, &bad_tokens, use_index));
}

fn many_ids(count: usize) -> String {
    let mut source = String::from("<svg>");
    for n in 0..count { source.push_str(&format!("<g id='i{n}'/>")); }
    source.push_str("</svg>");
    source
}

#[test]
fn corpus_measured_id_counts_are_indexed() {
    // WO-RUST-SCENE-LIMITS-01: chuaattractormodified has 100,088 id attributes and
    // ideogram-human-chromosome-20 has 144,217; both were refused at 65,536.
    for count in [100_088usize, 144_217] {
        let source = many_ids(count);
        let tokens = parsed(source.as_bytes());
        let index = ReferenceIndex::create(source.as_bytes(), &tokens).unwrap();
        assert_eq!(index.id_count(), count);
    }
}

#[test]
fn id_table_geometry_is_a_bounded_power_of_two() {
    assert!(REFERENCE_SLOT_CAPACITY.is_power_of_two());
    assert!(REFERENCE_SLOT_CAPACITY >= 2 * MAX_REFERENCE_IDS);
    // Worst-case table allocation: one u32 per slot, never more.
    assert!(REFERENCE_SLOT_CAPACITY * size_of::<u32>() <= 2 * 1024 * 1024);
}

#[test]
fn adversarial_id_overflow_is_refused_in_the_counting_phase() {
    // N+1 ids fail before the slot table is allocated.
    let source = many_ids(MAX_REFERENCE_IDS + 1);
    let tokens = parsed(source.as_bytes());
    let error = ReferenceIndex::create(source.as_bytes(), &tokens).unwrap_err();
    assert_eq!(error.category, ReferenceError::Limit);
}
