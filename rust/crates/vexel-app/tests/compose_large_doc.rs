//! Large-source behaviour of the compose pipeline: the single source cap,
//! its typed refusal and cancellation during decode of a multi-MiB source.
use std::cell::Cell;
use vexel_app::compose::{ComposeError, MAX_SVG_BYTES, compose_document, compose_svg};

const MIB: usize = 1024 * 1024;

/// A valid SVG of exactly `target` bytes: `rects` small rects, then padding
/// inside an XML comment so the element count stays within decode budgets.
fn svg_of_size(target: usize, rects: usize) -> Vec<u8> {
    let mut svg = String::from("<svg width='64' height='64' xmlns='http://www.w3.org/2000/svg'>");
    for n in 0..rects {
        let (x, y) = ((n * 7) % 56, (n * 13) % 56);
        svg.push_str(&format!(
            "<rect x='{x}' y='{y}' width='{}' height='4' fill='#{:06x}'/>\n",
            2 + n % 5,
            (n * 2_654_435_761) & 0xff_ffff
        ));
    }
    let tail = "</svg>";
    svg.push_str("<!--");
    assert!(svg.len() + "-->".len() + tail.len() < target);
    let pad = target - svg.len() - "-->".len() - tail.len();
    svg.extend(std::iter::repeat_n('x', pad));
    svg.push_str("-->");
    svg.push_str(tail);
    assert_eq!(svg.len(), target);
    svg.into_bytes()
}

#[test]
fn cap_constant_is_32_mib() {
    assert_eq!(MAX_SVG_BYTES, 32 * MIB);
}

#[test]
fn nine_mib_source_is_not_refused() {
    let svg = svg_of_size(9 * MIB, 2000);
    assert!(compose_svg(&svg, 64, 64).is_ok());
}

#[test]
fn thirty_mib_source_is_not_refused() {
    let svg = svg_of_size(30 * MIB, 2000);
    assert!(compose_svg(&svg, 64, 64).is_ok());
}

/// `rects` rects whose attributes are separated by padding spaces, so every
/// byte beyond 16 MiB belongs to real elements rather than one comment.
fn spaced_rects(rects: usize, pad: usize) -> Vec<u8> {
    let mut svg = String::from("<svg width='64' height='64' xmlns='http://www.w3.org/2000/svg'>");
    let spaces = " ".repeat(pad);
    for n in 0..rects {
        svg.push_str(&format!(
            "<rect x='{}' y='{}' width='3' height='3'{spaces}fill='#336699'/>\n",
            n % 60,
            (n / 60) % 60,
        ));
    }
    svg.push_str("</svg>");
    svg.into_bytes()
}

#[test]
fn rect_documents_across_old_8_and_16_mib_caps_compose() {
    for (rects, pad, min_len) in [(1000, 9000, 8 * MIB), (1000, 19000, 16 * MIB)] {
        let svg = spaced_rects(rects, pad);
        assert!(svg.len() > min_len, "{}", svg.len());
        assert!(svg.len() <= MAX_SVG_BYTES);
        assert!(compose_svg(&svg, 64, 64).is_ok(), "{} bytes", svg.len());
    }
}

/// Many tiny tokens: the lexical token storage, not the source size, was the
/// hidden 16 MiB site (about 4 bytes of token storage per source byte).
#[test]
fn token_dense_source_is_not_refused_by_token_storage() {
    let mut svg = String::from("<svg width='8' height='8' xmlns='http://www.w3.org/2000/svg'>");
    while svg.len() < 6 * MIB {
        svg.push_str("<desc a='1' b='2' c='3' d='4'/>\n");
    }
    svg.push_str("<rect width='4' height='4'/></svg>");
    assert!(svg.len() < 8 * MIB);
    let result = compose_svg(svg.as_bytes(), 8, 8);
    assert!(result.is_ok(), "{:?}", result.map(|f| f.width));
}

#[test]
fn thirty_three_mib_source_is_typed_too_large() {
    let svg = vec![b' '; 33 * MIB];
    assert_eq!(compose_svg(&svg, 10, 10), Err(ComposeError::LimitDecode));
    let at_cap_plus_one = vec![b' '; MAX_SVG_BYTES + 1];
    assert_eq!(
        compose_svg(&at_cap_plus_one, 10, 10),
        Err(ComposeError::LimitDecode)
    );
}

#[test]
fn immediate_cancel_stops_decode_of_large_source() {
    let svg = svg_of_size(12 * MIB, 2000);
    let polls = Cell::new(0u32);
    let cancel = || {
        polls.set(polls.get() + 1);
        true
    };
    let result = compose_document(&svg, 64, 64, &cancel);
    assert!(matches!(result, Err(ComposeError::Cancelled)));
    assert_eq!(polls.get(), 1, "decode must stop at the first checkpoint");
}
