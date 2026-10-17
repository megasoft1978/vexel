//! SVG path grammar: a `.` that directly follows a number with no separator
//! starts a new number (`1.2.3` is `1.2` then `.3`), in every command position.
#![allow(dead_code)]
mod support;

use vexel_base::Span;
use vexel_geometry::affine_identity;
use vexel_svg_path::{parse_path, Diagnostic, DiagnosticKind, ParseOptions, PathDocument};

fn parse(source: &[u8]) -> Result<PathDocument, Diagnostic> {
    parse_path(
        source,
        Span { start: 0, end: source.len() },
        affine_identity(),
        ParseOptions::default(),
    )
}

fn values(document: &PathDocument) -> Vec<u64> {
    document.numbers.iter().map(|number| number.value.to_bits()).collect()
}

fn geometry(document: &PathDocument) -> (Vec<[u64; 4]>, Vec<u8>) {
    let edges = document
        .edges
        .iter()
        .map(|edge| [edge.a.x.to_bits(), edge.a.y.to_bits(), edge.b.x.to_bits(), edge.b.y.to_bits()])
        .collect();
    let commands = document.segments.iter().map(|segment| segment.command).collect();
    (edges, commands)
}

fn assert_same_as(compact: &[u8], spaced: &[u8]) {
    let compact_doc = parse(compact).unwrap_or_else(|error| panic!("{compact:?}: {error:?}"));
    let spaced_doc = parse(spaced).unwrap_or_else(|error| panic!("{spaced:?}: {error:?}"));
    assert_eq!(values(&compact_doc), values(&spaced_doc), "{compact:?} numbers");
    assert_eq!(geometry(&compact_doc), geometry(&spaced_doc), "{compact:?} geometry");
}

fn assert_values(source: &[u8], expected: &[f64]) {
    let document = parse(source).unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
    let actual: Vec<f64> = document.numbers.iter().map(|number| number.value).collect();
    assert_eq!(actual.len(), expected.len(), "{source:?} count");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!((actual - expected).abs() < 1.0e-9, "{source:?} value {index}: {actual} != {expected}");
    }
}

#[test]
fn inkscape_and_illustrator_compact_dot_strings_match_spaced_forms() {
    assert_same_as(b"M0 0l-.1.21.1.55z", b"M0 0l-0.1 0.21 0.1 0.55z");
    assert_same_as(b"M1 2.5.6 7", b"M1 2.5 .6 7");
    assert_same_as(b"M0 0L1 2.5.6 7", b"M0 0L1 2.5 .6 7");
}

#[test]
fn compact_dot_splits_fraction_into_two_numbers() {
    assert_values(b"M0 0 L1.2.3", &[0.0, 0.0, 1.2, 0.3]);
    assert_values(b"M0 0 L.2.3.4.5", &[0.0, 0.0, 0.2, 0.3, 0.4, 0.5]);
    assert_values(b"M0 0 L-.1-.2", &[0.0, 0.0, -0.1, -0.2]);
    assert_values(b"M0 0 L1.2e1.5", &[0.0, 0.0, 12.0, 0.5]);
    assert_values(b"M0 0 L1e2.5", &[0.0, 0.0, 100.0, 0.5]);
    assert_values(b"M0 0 L1.5.5", &[0.0, 0.0, 1.5, 0.5]);
    assert_values(b"M0 0 L1. 2", &[0.0, 0.0, 1.0, 2.0]);
    assert_values(b"M0 0 L1..2", &[0.0, 0.0, 1.0, 0.2]);
}

#[test]
fn compact_dot_begins_an_implicit_repeat_after_a_complete_group() {
    assert_same_as(b"M0 0 L1.2.3 4 5", b"M0 0 L1.2 .3 4 5");
    assert_same_as(b"M0 0 L1.2.3.4.5.6.7", b"M0 0 L1.2 .3 .4 .5 .6 .7");
    assert_same_as(b"M0 0 L.2.3.4.5z", b"M0 0 L.2 .3 .4 .5z");
    assert_same_as(b"M0 0 L1.2.4Z", b"M0 0 L1.2 .4Z");
}

#[test]
fn compact_dot_after_arc_radius_begins_a_new_number() {
    assert_values(b"M0 0 A2.3.4 0 0 1 4 5", &[0.0, 0.0, 2.3, 0.4, 0.0, 0.0, 1.0, 4.0, 5.0]);
}

#[test]
fn malformed_compact_dot_forms_still_fail() {
    let malformed: &[&[u8]] = &[
        b"M0 0 L.",
        b"M0 0 L1 . 2",
        b"M0 0 L.2.3.4",
        b"M0 0 L1.2.3.",
        b"M0 0 L1.2.3.4",
        b"M0 0 L.2.3.4.5.6.",
        b"M0 0 L1e.5",
        b"M0 0 L1 2 3",
        b"M0 0 L1 2x",
        b"M0 0 L1 2.3.",
        b"M.4..6",
    ];
    for source in malformed {
        let error = parse(source).expect_err(&format!("{source:?} was accepted"));
        assert!(
            matches!(error.kind, DiagnosticKind::Path | DiagnosticKind::Unsupported),
            "{source:?}: {error:?}"
        );
    }
}

#[test]
fn incomplete_compact_pair_reports_the_group_error() {
    let error = parse(b"M0 0 L.2.3.4").unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Path);
    // The second pair is missing its y operand; the error is at end of input.
    assert_eq!(error.offset, 12);
}

#[test]
fn huge_compact_dot_counts_are_bounded_and_exact() {
    let mut even = b"M0 0 L".to_vec();
    for _ in 0..20_000 {
        even.extend_from_slice(b".1");
    }
    let document = parse(&even).unwrap();
    assert_eq!(document.numbers.len(), 20_002);
    assert_eq!(document.segments.len(), 10_001);

    let mut odd = b"M0 0 L".to_vec();
    for _ in 0..20_001 {
        odd.extend_from_slice(b".1");
    }
    assert_eq!(parse(&odd).unwrap_err().kind, DiagnosticKind::Path);

    let capped = ParseOptions { max_numbers: 19_999, ..ParseOptions::default() };
    let error = parse_path(&even, Span { start: 0, end: even.len() }, affine_identity(), capped).unwrap_err();
    assert_eq!(error.kind, DiagnosticKind::Limit);
}

/// Seeded generator that never emits a compact-dot form: every number that
/// follows another number is separated by whitespace or a comma, unless it
/// begins with a sign or a command letter.
struct Lcg(u64);
impl Lcg {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound
    }
    fn pick<'a>(&mut self, options: &[&'a [u8]]) -> &'a [u8] {
        options[self.below(options.len() as u64) as usize]
    }
}

fn push_number(rng: &mut Lcg, out: &mut Vec<u8>) {
    match rng.below(3) {
        0 => out.push(b'-'),
        1 => out.push(b'+'),
        _ => (),
    }
    let integer = rng.below(4);
    let has_fraction = rng.below(2) == 0;
    let fraction = rng.below(4);
    if integer == 0 && (!has_fraction || fraction == 0) {
        out.push(b'0');
    }
    for _ in 0..integer {
        out.push(b'0' + rng.below(10) as u8);
    }
    if has_fraction {
        out.push(b'.');
        for _ in 0..fraction {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
    if rng.below(6) == 0 {
        out.push(if rng.below(2) == 0 { b'e' } else { b'E' });
        match rng.below(3) {
            0 => out.push(b'-'),
            1 => out.push(b'+'),
            _ => (),
        }
        for _ in 0..=rng.below(2) {
            out.push(b'0' + rng.below(10) as u8);
        }
    }
}

fn arguments(command: u8) -> usize {
    match command.to_ascii_uppercase() {
        b'Z' => 0,
        b'H' | b'V' => 1,
        b'S' | b'Q' => 4,
        b'T' | b'M' | b'L' => 2,
        b'C' => 6,
        b'A' => 7,
        _ => 0,
    }
}

fn generate(seed: u64) -> Vec<u8> {
    const COMMANDS: &[u8] = b"MLCQSTAHVZmlcqstahvz";
    let mut rng = Lcg(seed);
    let mut out = Vec::new();
    if rng.below(4) == 0 {
        out.push(b' ');
    }
    let groups = 1 + rng.below(10);
    for group in 0..groups {
        let command = if group == 0 { b'M' } else { COMMANDS[rng.below(COMMANDS.len() as u64) as usize] };
        out.push(command);
        let arity = arguments(command);
        // Mostly well-formed groups so both accepted and rejected inputs occur.
        let count = if rng.below(8) == 0 {
            rng.below(2 * arity as u64 + 2) as usize
        } else {
            arity * (1 + rng.below(2) as usize)
        };
        for index in 0..count {
            let mut number = Vec::new();
            let arc_flag = command.to_ascii_uppercase() == b'A' && matches!(index % 7, 3 | 4);
            if arc_flag && rng.below(4) != 0 {
                number.push(b'0' + rng.below(2) as u8);
            } else {
                push_number(&mut rng, &mut number);
            }
            let separator: &[&[u8]] = &[b" ", b",", b" , ", b"", b"  "];
            let mut sep = rng.pick(separator);
            let after_number = out.last().is_some_and(|byte| !byte.is_ascii_alphabetic());
            if sep.is_empty() && after_number && matches!(number[0], b'0'..=b'9' | b'.') {
                sep = b" ";
            }
            out.extend_from_slice(sep);
            out.extend_from_slice(&number);
        }
        if rng.below(8) == 0 {
            out.push(b' ');
        }
    }
    out
}

/// A `.` that directly follows a number which already has a decimal point
/// is a compact-dot boundary.
fn has_compact_dot(bytes: &[u8]) -> bool {
    bytes.iter().enumerate().any(|(index, byte)| {
        if *byte != b'.' || index == 0 || !bytes[index - 1].is_ascii_digit() {
            return false;
        }
        let mut start = index;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        start > 0 && bytes[start - 1] == b'.'
    })
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Digest of input bytes and parse fingerprints for 4096 seeded inputs.
/// Recorded from the parser before the compact-dot change; any drift on a
/// compact-dot-free input changes it.
const PRE_CHANGE_DIGEST: u64 = 0x922d_0b63_cab7_3464;

#[test]
fn compact_dot_free_inputs_match_pre_change_parser_digest() {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut accepted = 0usize;
    for seed in 0..4096_u64 {
        let input = generate(seed);
        assert!(!has_compact_dot(&input), "generator emitted a compact-dot form: {input:?}");
        let result = parse(&input);
        if result.is_ok() {
            accepted += 1;
        }
        hash = fnv1a(&input, hash);
        hash = fnv1a(support::fingerprint(result).as_bytes(), hash);
    }
    assert!(accepted > 0 && accepted < 4096, "generator must mix accepted and rejected inputs: {accepted}");
    assert_eq!(hash, PRE_CHANGE_DIGEST, "actual digest {hash:#018x}");
}

#[test]
fn compact_dot_detector_flags_only_compact_forms() {
    assert!(has_compact_dot(b"M1.2.3"));
    assert!(has_compact_dot(b"M.5.6"));
    assert!(!has_compact_dot(b"M1.2 .3"));
    assert!(!has_compact_dot(b"M-.1-.2"));
    assert!(!has_compact_dot(b"M12.5,.5"));
}
