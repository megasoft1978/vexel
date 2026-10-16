//! Layout unit tests against values recorded from the legacy C provider
//! (`platform_text.m` + `app_text_layout` glyph-local conversion), produced by a
//! throwaway harness built outside the repository on macOS 27.0.1. Fonts used:
//! Helvetica, Times, Menlo, Helvetica Neue (system default family).
#![cfg(target_os = "macos")]

use vexel_platform_apple::{
    MAX_FAMILIES, MAX_FAMILY_BYTES, MAX_TEXT_BYTES, TextCommand, TextLayout, TextLayoutError,
    TextRequest, TextStyle, TextVerb, text_layout,
};

const EPS: f64 = 1e-6;

fn req<'a>(text: &'a [u8], families: &'a [&'a [u8]]) -> TextRequest<'a> {
    TextRequest {
        text,
        font_families: families,
        size_points: 12.0,
        weight: 400.0,
        style: TextStyle::Normal,
        letter_spacing_points: 0.0,
    }
}

fn weighted_sum(layout: &TextLayout) -> f64 {
    let mut sum = 0.0;
    for command in &layout.commands {
        for (k, v) in command.values.iter().enumerate() {
            sum += v * (k + 1) as f64;
        }
    }
    sum
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPS
}

fn check(layout: &TextLayout, glyphs: usize, commands: usize, advance: f64, sum: f64) {
    assert_eq!(layout.glyphs.len(), glyphs);
    assert_eq!(layout.commands.len(), commands);
    assert!(
        close(layout.advance_width, advance),
        "{}",
        layout.advance_width
    );
    assert!(close(weighted_sum(layout), sum), "{}", weighted_sum(layout));
}

#[test]
fn empty_string_has_metrics_and_no_glyphs() {
    let layout = text_layout(&req(b"", &[b"Helvetica"])).unwrap();
    check(&layout, 0, 0, 0.0, 0.0);
    assert!(close(layout.ascent, 9.240234375));
    assert!(close(layout.descent, 2.759765625));
    assert!(close(layout.leading, 0.0));
}

#[test]
fn single_glyph_matches_the_recorded_command_stream() {
    let layout = text_layout(&req(b"A", &[b"Helvetica"])).unwrap();
    check(&layout, 1, 13, 8.00390625, 117.650390625);
    let glyph = layout.glyphs[0];
    assert_eq!(
        (glyph.glyph_id, glyph.command_start, glyph.command_count),
        (36, 0, 13)
    );
    assert!(close(glyph.advance, 8.00390625) && glyph.x == 0.0 && glyph.y == 0.0);
    let mv = |x, y| TextCommand {
        verb: TextVerb::Move,
        values: [x, y, 0.0, 0.0, 0.0, 0.0],
    };
    let ln = |x, y| TextCommand {
        verb: TextVerb::Line,
        values: [x, y, 0.0, 0.0, 0.0, 0.0],
    };
    let cl = TextCommand {
        verb: TextVerb::Close,
        values: [0.0; 6],
    };
    let expected = [
        mv(5.33203125, 3.52734375),
        ln(4.025390625, 7.330078125),
        ln(2.63671875, 3.52734375),
        cl,
        mv(3.416015625, 8.607421875),
        ln(4.734375, 8.607421875),
        ln(7.857421875, 0.0),
        ln(6.580078125, 0.0),
        ln(5.70703125, 2.578125),
        ln(2.302734375, 2.578125),
        ln(1.37109375, 0.0),
        ln(0.17578125, 0.0),
        cl,
    ];
    for (got, want) in layout.commands.iter().zip(expected) {
        assert_eq!(got.verb, want.verb);
        for k in 0..6 {
            assert!(close(got.values[k], want.values[k]), "{got:?} vs {want:?}");
        }
    }
}

#[test]
fn ligature_free_ascii_matches_recorded_advances_and_ids() {
    let layout = text_layout(&req(b"Hello World", &[b"Helvetica"])).unwrap();
    check(&layout, 11, 148, 62.015625, 6004.7861328125);
    let ids = [43, 72, 79, 79, 82, 3, 58, 82, 85, 79, 71];
    let advances = [
        8.666015625,
        6.673828125,
        2.666015625,
        2.666015625,
        6.673828125,
        3.333984375,
        11.326171875,
        6.673828125,
        3.99609375,
        2.666015625,
        6.673828125,
    ];
    let counts = [13, 26, 5, 5, 20, 0, 14, 20, 15, 5, 25];
    let mut x = 0.0;
    for (i, glyph) in layout.glyphs.iter().enumerate() {
        assert_eq!(glyph.glyph_id, ids[i]);
        assert!(close(glyph.advance, advances[i]) && close(glyph.x, x));
        assert_eq!(glyph.command_count, counts[i]);
        x += advances[i];
    }
}

#[test]
fn combining_mark_composes_to_one_glyph() {
    let layout = text_layout(&req("e\u{301}".as_bytes(), &[b"Helvetica"])).unwrap();
    check(&layout, 1, 31, 6.673828125, 1498.8154296875);
}

#[test]
fn missing_family_falls_back_as_the_c_does() {
    let layout = text_layout(&req(b"Hello", &[b"NoSuchFamilyVexelXYZ"])).unwrap();
    check(&layout, 5, 69, 27.345703125, 2750.4375);
}

#[test]
fn default_family_is_sans_serif() {
    let layout = text_layout(&req(b"Hello", &[])).unwrap();
    check(&layout, 5, 94, 27.324000000000005, 4375.1399999999976);
    assert!(close(layout.ascent, 11.4239501953125) && close(layout.leading, 0.33599853515625));
}

#[test]
fn weight_style_and_letter_spacing_match_the_recorded_layouts() {
    let mut request = req(b"Hello", &[b"Times"]);
    request.size_points = 18.0;
    request.weight = 700.0;
    request.style = TextStyle::Italic;
    check(
        &text_layout(&request).unwrap(),
        5,
        141,
        40.9921875,
        10051.42822265625,
    );

    let mut request = req(b"Hello", &[b"Menlo"]);
    request.letter_spacing_points = 1.5;
    check(
        &text_layout(&request).unwrap(),
        5,
        83,
        43.623046875,
        3167.5908203125,
    );
}

#[test]
fn zero_and_negative_size_are_typed_errors() {
    for size in [0.0, -3.0, f64::NAN, f64::INFINITY, 5000.0] {
        let mut request = req(b"A", &[b"Helvetica"]);
        request.size_points = size;
        assert_eq!(
            text_layout(&request).unwrap_err(),
            TextLayoutError::InvalidArgument
        );
    }
    let mut request = req(b"A", &[b"Helvetica"]);
    request.weight = 50.0;
    assert_eq!(
        text_layout(&request).unwrap_err(),
        TextLayoutError::InvalidArgument
    );
}

#[test]
fn over_limit_text_and_families_return_the_limit_error() {
    let big = vec![b'a'; MAX_TEXT_BYTES + 1];
    assert_eq!(
        text_layout(&req(&big, &[])).unwrap_err(),
        TextLayoutError::Limit
    );
    let many: Vec<&[u8]> = vec![b"Helvetica".as_slice(); MAX_FAMILIES + 1];
    assert_eq!(
        text_layout(&req(b"A", &many)).unwrap_err(),
        TextLayoutError::Limit
    );
    let long = vec![b'f'; MAX_FAMILY_BYTES + 1];
    assert_eq!(
        text_layout(&req(b"A", &[&long])).unwrap_err(),
        TextLayoutError::Limit
    );
}

#[test]
fn glyph_count_limit_is_typed() {
    // MAX_GLYPHS (16384) glyphs fit in MAX_TEXT_BYTES, so 20000 ASCII bytes
    // exceed the glyph limit while staying under the byte limit.
    let text = vec![b'W'; 20_000];
    assert_eq!(
        text_layout(&req(&text, &[b"Helvetica"])).unwrap_err(),
        TextLayoutError::Limit
    );
}

#[test]
fn invalid_utf8_nul_and_emoji_are_typed_errors_without_panic() {
    assert_eq!(
        text_layout(&req(b"A\xff", &[])).unwrap_err(),
        TextLayoutError::InvalidUtf8
    );
    assert_eq!(
        text_layout(&req(b"A\xc0\xaf", &[])).unwrap_err(),
        TextLayoutError::InvalidUtf8
    );
    assert_eq!(
        text_layout(&req(b"A", &[b"Hel\xffvetica"])).unwrap_err(),
        TextLayoutError::InvalidUtf8
    );
    assert_eq!(
        text_layout(&req(b"A", &[b"He\0lv"])).unwrap_err(),
        TextLayoutError::InvalidArgument
    );
    assert_eq!(
        text_layout(&req(b"A", &[b""])).unwrap_err(),
        TextLayoutError::InvalidArgument
    );
    assert_eq!(
        text_layout(&req("\u{1F600}".as_bytes(), &[])).unwrap_err(),
        TextLayoutError::Unsupported
    );
}
