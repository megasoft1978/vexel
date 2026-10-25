//! Names the cause when the renderer rejects a document after a fragment
//! insert: a `url(#id)` or `href="#id"` that names no element.

const GENERIC: &str =
    "element_add_fragment: the renderer cannot draw this construct; edit undone. See docs/mcp.md";

fn defined(source: &[u8], id: &[u8]) -> bool {
    [b'"', b'\''].iter().any(|quote| {
        let needle = [b" id=".as_slice(), &[*quote], id, &[*quote]].concat();
        source.windows(needle.len()).any(|part| part == needle)
    })
}

/// Attribute or property name that precedes the `url(` at `at`.
fn property_before(xml: &[u8], at: usize) -> &[u8] {
    let mut end = at;
    while end > 0 && matches!(xml[end - 1], b'=' | b':' | b'"' | b'\'' | b' ') {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (xml[start - 1].is_ascii_alphabetic() || xml[start - 1] == b'-') {
        start -= 1;
    }
    &xml[start..end]
}

fn undefined_message(property: &[u8]) -> &'static str {
    match property {
        b"filter" => {
            "element_add_fragment: undefined filter reference (url(#id) names no element in the document); define the <filter> first or include it in the fragment; edit undone"
        }
        b"mask" => {
            "element_add_fragment: undefined mask reference (url(#id) names no element in the document); define the <mask> first or include it in the fragment; edit undone"
        }
        b"clip-path" => {
            "element_add_fragment: undefined clip-path reference (url(#id) names no element in the document); define the <clipPath> first or include it in the fragment; edit undone"
        }
        b"fill" | b"stroke" => {
            "element_add_fragment: undefined paint reference (url(#id) names no gradient or pattern in the document); define it first or include it in the fragment; edit undone"
        }
        _ => {
            "element_add_fragment: undefined reference (a url(#id) names no element in the document); define the target first or include it in the fragment; edit undone"
        }
    }
}

/// Why `source` (the document with `xml` just inserted) does not render.
pub(super) fn diagnose(source: &[u8], xml: &[u8]) -> &'static str {
    let mut at = 0;
    while let Some(found) = xml[at..].windows(4).position(|w| w == b"url(") {
        let open = at + found;
        let mut start = open + 4;
        if matches!(xml.get(start), Some(b'\'' | b'"')) {
            start += 1;
        }
        at = start;
        if xml.get(start) != Some(&b'#') {
            continue;
        }
        start += 1;
        let end = xml[start..]
            .iter()
            .position(|b| matches!(b, b')' | b'\'' | b'"'))
            .map_or(xml.len(), |len| start + len);
        if !defined(source, &xml[start..end]) {
            return undefined_message(property_before(xml, open));
        }
        at = end;
    }
    for marker in [b"href=\"#".as_slice(), b"href='#"] {
        let mut at = 0;
        while let Some(found) = xml[at..].windows(marker.len()).position(|w| w == marker) {
            let start = at + found + marker.len();
            let end = xml[start..]
                .iter()
                .position(|b| matches!(b, b'\'' | b'"'))
                .map_or(xml.len(), |len| start + len);
            if !defined(source, &xml[start..end]) {
                return "element_add_fragment: undefined reference (href=\"#id\" names no element in the document); define the target first or include it in the fragment; edit undone";
            }
            at = end;
        }
    }
    GENERIC
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = br##"<svg><defs><filter id="f1"/></defs><rect id="r" filter="url(#f1)" mask="url(#m9)"/></svg>"##;

    #[test]
    fn names_the_undefined_reference_kind() {
        let msg = |xml: &str| diagnose(SRC, xml.as_bytes());
        assert!(msg(r##"<rect filter="url(#nope)"/>"##).contains("undefined filter"));
        assert!(msg(r##"<rect style="mask:url(#nope)"/>"##).contains("undefined mask"));
        assert!(msg(r##"<rect clip-path="url('#nope')"/>"##).contains("undefined clip-path"));
        assert!(msg(r##"<rect fill="url(#nope)"/>"##).contains("undefined paint"));
        assert!(msg(r##"<use href="#nope"/>"##).contains("undefined reference"));
        // Defined targets fall back to the generic text, which names no blend mode.
        let generic = msg(r##"<rect filter="url(#f1)"/>"##);
        assert!(!generic.contains("blend") && !generic.contains("undefined"));
    }
}
