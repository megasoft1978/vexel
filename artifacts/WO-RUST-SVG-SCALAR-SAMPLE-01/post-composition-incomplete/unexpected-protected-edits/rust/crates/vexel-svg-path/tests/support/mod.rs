use vexel_base::Span;
use vexel_geometry::{Affine, affine_identity};
use vexel_svg_path::{Diagnostic, ParseOptions, PathDocument};

pub struct Case {
    pub name: String,
    pub source: Vec<u8>,
    pub span: Span,
    pub transform: Affine,
    pub options: ParseOptions,
}

impl Case {
    fn new(name: &str, source: &[u8]) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            span: Span {
                start: 0,
                end: source.len(),
            },
            transform: affine_identity(),
            options: ParseOptions::default(),
        }
    }
}

/// Every authored path fixture from the twelve original acceptance tests, plus
/// grammar/numeric/error-precedence probes captured before the parser rewrite.
pub fn cases() -> Vec<Case> {
    let groups: &[(&str, &[&[u8]])] = &[
        (
            "commands",
            &[b"M1 2 L3 4 h2 v-1 C6 5 7 6 8 7 s2 2 3 3 Q12 12 14 10 t2 0 A2 3 0 0 1 19 12 z"],
        ),
        ("repeats", &[b"M1 1 4 1 4 4 M8 8 l2 0 z"]),
        (
            "smooth",
            &[
                b"M0 0 C1 0 2 0 3 0 S5 1 6 0 L7 0 S8 1 9 0",
                b"M0 0 Q2 2 4 0 T8 0 L9 0 T11 0",
            ],
        ),
        (
            "arcs",
            &[
                b"M0 0 A0 3 0 0 1 4 5",
                b"M2 2 A3 4 45 1 1 2 2",
                b"M0 0 A1 1 0 0 1 10 0",
                b"M0 0 A1 1 0 2 0 4 4",
                b"M0 0 A-1 1 0 0 0 4 4",
                b"M0 0 A1 1 0 0.0 0 4 4",
            ],
        ),
        (
            "numbers",
            &[
                b"M-.4.6",
                b"M1e2 -2.E-1 L+.5E+2 .2e1",
                b"M0 0L1-2+3-4",
                b"M.4..6",
                b"M0 0 S1 2 3",
                b"M999999 0 l2 0",
                b"M1e 0",
                b"M1e+ 0",
                b"M1e999 0",
                b"M1e-999 0",
                b"M-0 +0",
                b"M0.1234567890123456789012345 0",
                b"M99999999999999999999999999999999999999e-35 0",
                b"M1.e1 .1e1",
                b"M+ 1",
                b"M. 1",
                b"M1e1000000000000000000000000 0",
            ],
        ),
        (
            "grammar",
            &[
                b"",
                b" \t\r\n",
                b"L0 0",
                b"M0 0X",
                b"M0 0 Z 1 2",
                b"M0 0zZ",
                b"m1 1 2 2 -1-1 h2 3 v4 -1 c1 0 2 1 3 2 s1 2 3 4 q1 2 3 4 t2 3 a4 5 30 011 2 z",
                b"M0 0 L.2.3.4.5",
                b"M0 0\x0cL1 2",
                b"M0 0 L1",
                b"M0 0 H",
                b"M0 0 A1 1 0 001 2",
                b"M0 0 A1 1 0 +0 1 2 3",
                b"M0 0 A1 1 0 01-2-3",
                b"M0 0 C0 0 0 0 0 0 C1 1 2 2 3 3",
                b"M0 0 M1 1 z M2 2",
            ],
        ),
        (
            "commas",
            &[
                b",M0 0",
                b"M,0 0",
                b"M0,0",
                b"M0 , \t0 L1,2",
                b"M0 0,",
                b"M0 0 , \t L1 2",
                b"M0,,0",
                b"M0 , , 0",
                b"X ,",
                b"Mbad L1 2,",
                b"M0 0, Z",
                b"M0 , -0",
                b"M0. , .1",
            ],
        ),
    ];
    let mut out = Vec::new();
    for (group, sources) in groups {
        for (index, source) in sources.iter().enumerate() {
            out.push(Case::new(&format!("{group}-{index}"), source));
        }
    }
    let mut compact = Case::new(
        "compact-source-spans",
        b"prefix M-.4.6A2 3 0 011.2.4 suffix",
    );
    compact.span = Span {
        start: 7,
        end: compact.source.len() - 7,
    };
    out.push(compact);
    let mut malformed = Case::new("malformed-absolute", b"long source: M0 0 S1 2 3");
    malformed.span.start = 13;
    out.push(malformed);
    let mut offset = Case::new("malformed-offset-101", &vec![b' '; 101]);
    offset.source.extend_from_slice(b"M.4..6");
    offset.span = Span {
        start: 101,
        end: 107,
    };
    out.push(offset);
    let mut bad = Case::new("invalid-transform-offset", &vec![b' '; 20]);
    bad.source.extend_from_slice(b"M0 0");
    bad.span = Span { start: 20, end: 24 };
    bad.transform.a = f64::NAN;
    out.push(bad);
    let mut bad_span = Case::new("invalid-span", b"abc");
    bad_span.span = Span { start: 2, end: 4 };
    out.push(bad_span);
    for (name, options) in [
        (
            "command-cap",
            ParseOptions {
                max_commands: 1,
                ..ParseOptions::default()
            },
        ),
        (
            "edge-cap",
            ParseOptions {
                max_edges: 1,
                ..ParseOptions::default()
            },
        ),
        (
            "number-cap",
            ParseOptions {
                max_numbers: 1,
                ..ParseOptions::default()
            },
        ),
        (
            "zero-command-cap",
            ParseOptions {
                max_commands: 0,
                ..ParseOptions::default()
            },
        ),
        (
            "zero-edge-cap",
            ParseOptions {
                max_edges: 0,
                ..ParseOptions::default()
            },
        ),
        (
            "clamped-caps",
            ParseOptions {
                max_commands: usize::MAX,
                max_edges: usize::MAX,
                max_numbers: usize::MAX,
            },
        ),
    ] {
        let mut case = Case::new(name, b"M0 0L1 1");
        case.options = options;
        out.push(case);
    }
    for (index, transform) in [
        affine_identity(),
        Affine {
            a: 4.0,
            d: 4.0,
            ..affine_identity()
        },
        Affine {
            a: 1.0,
            b: 0.5,
            c: -0.75,
            d: 1.0,
            ..affine_identity()
        },
    ]
    .iter()
    .copied()
    .enumerate()
    {
        let mut curve = Case::new(
            &format!("transform-curve-{index}"),
            b"M0 0 C0 10 10 10 10 0",
        );
        curve.transform = transform;
        out.push(curve);
        let mut wrapper = Case::new(&format!("wrapper-curve-{index}"), b"M1 2 C2 9 8 -1 10 3");
        wrapper.transform = transform;
        out.push(wrapper);
    }
    let mut translation = Case::new("translation", b"M1 2L3 4");
    translation.transform.e = 12.0;
    translation.transform.f = -5.0;
    out.push(translation);
    out
}

/// Bitwise floats make signed zero, operation order, ties and provenance part
/// of the frozen evidence, rather than accepting a numeric tolerance.
pub fn fingerprint(result: Result<PathDocument, Diagnostic>) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    match result {
        Err(error) => writeln!(text, "ERROR {:?} {}", error.kind, error.offset).unwrap(),
        Ok(path) => {
            writeln!(text, "OK").unwrap();
            for edge in path.edges {
                writeln!(
                    text,
                    "E {:016x} {:016x} {:016x} {:016x}",
                    edge.a.x.to_bits(),
                    edge.a.y.to_bits(),
                    edge.b.x.to_bits(),
                    edge.b.y.to_bits()
                )
                .unwrap();
            }
            for subpath in path.subpaths {
                writeln!(
                    text,
                    "P {} {} {}",
                    subpath.first_edge, subpath.edge_count, subpath.closed
                )
                .unwrap();
            }
            for segment in path.segments {
                writeln!(
                    text,
                    "S {} {} {} {:?} {} {} {} {}",
                    segment.command,
                    segment.source.start,
                    segment.source.end,
                    segment.command_offset,
                    segment.first_number,
                    segment.number_count,
                    segment.first_edge,
                    segment.edge_count
                )
                .unwrap();
            }
            for number in path.numbers {
                writeln!(
                    text,
                    "N {} {} {:016x}",
                    number.source.start,
                    number.source.end,
                    number.value.to_bits()
                )
                .unwrap();
            }
        }
    }
    text
}
