//! Coarse performance gate over three deterministic synthetic workloads.
//!
//! Each case is timed (best of three) and must stay at or below `max_ms` from
//! `rust/ci/perf-baseline.json`. This is a regression tripwire, not a speed
//! claim. `VEXEL_PERF_PRINT=1` prints the measured values. The test never
//! writes the baseline file.
use std::time::Instant;
use vexel_app::compose::compose_svg;
use vexel_json::JsonDocument;
use vexel_protocol::{ProtocolBackend, ProtocolSession, ToolResponse, handle};
use vexel_xml::{ReaderStatus, XmlReader};

struct NoTools;
impl ProtocolBackend for NoTools {
    fn call_tool(&mut self, _: &JsonDocument<'_>, _: usize, _: &str) -> Option<ToolResponse> {
        None
    }
}

fn rect_document(rects: usize, target_bytes: usize) -> Vec<u8> {
    let mut svg = String::from("<svg width='512' height='512' xmlns='http://www.w3.org/2000/svg'>");
    let mut n = 0usize;
    while n < rects || svg.len() < target_bytes {
        let (x, y) = ((n * 37) % 480, (n * 91) % 480);
        let fill = format!("#{:06x}", (n * 2_654_435_761) & 0xff_ffff);
        svg.push_str(&format!(
            "<rect x='{x}' y='{y}' width='{}' height='{}' fill='{fill}'/>\n",
            8 + n % 24,
            8 + n % 17
        ));
        n += 1;
    }
    svg.push_str("</svg>");
    svg.into_bytes()
}

fn tokenize_one_mebibyte() {
    let source = rect_document(0, 1 << 20);
    let mut reader = XmlReader::new(&source).unwrap();
    let mut tags = 0usize;
    while reader.step(64 * 1024, |_| {
        tags += 1;
        true
    }) == ReaderStatus::Ok
    {}
    assert!(
        reader.progress().complete && tags > 10_000,
        "tokenize workload broke"
    );
}

fn compose_two_thousand_rects() {
    let source = rect_document(2000, 0);
    let frame = compose_svg(&source, 512, 512).expect("compose workload must succeed");
    assert_eq!((frame.width, frame.height), (512, 512));
}

fn compose_four_thousand_rects() {
    let source = rect_document(4000, 0);
    let frame = compose_svg(&source, 512, 512).expect("compose workload must succeed");
    assert_eq!((frame.width, frame.height), (512, 512));
}

fn thousand_tools_list_round_trips() {
    let init = br#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"perf","version":"1"}}}"#;
    let ready = br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let list = br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
    // The protocol rate-limits a session to 120 requests a minute, so the
    // 1000 round trips are spread over fresh sessions of 100 lists each.
    for _ in 0..10 {
        let mut session = ProtocolSession::default();
        handle(&mut session, init, Some(&mut NoTools)).unwrap();
        handle(&mut session, ready, Some(&mut NoTools)).unwrap();
        for _ in 0..100 {
            let reply = handle(&mut session, list, Some(&mut NoTools)).unwrap();
            assert!(reply.len() > 100, "tools/list reply is unexpectedly small");
        }
    }
}

fn best_of_three_ms(work: fn()) -> f64 {
    (0..3)
        .map(|_| {
            let start = Instant::now();
            work();
            start.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::INFINITY, f64::min)
}

fn max_ms(baseline: &JsonDocument<'_>, case: &str) -> u64 {
    let cases = baseline
        .object_get(baseline.root(), b"cases")
        .expect("cases object");
    let entry = baseline
        .object_get(cases, case.as_bytes())
        .unwrap_or_else(|| panic!("baseline has no case {case}"));
    let max = baseline
        .object_get(entry, b"max_ms")
        .unwrap_or_else(|| panic!("case {case} has no max_ms"));
    baseline
        .uint(max)
        .unwrap_or_else(|| panic!("case {case} max_ms is not an unsigned integer"))
}

#[test]
fn workloads_stay_within_baseline_max_ms() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ci/perf-baseline.json");
    let text = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let baseline = JsonDocument::parse(&text).expect("perf-baseline.json must be valid JSON");
    let print = std::env::var("VEXEL_PERF_PRINT").is_ok_and(|v| v == "1");
    let cases: [(&str, fn()); 4] = [
        ("xml_tokenize_1mib", tokenize_one_mebibyte),
        ("compose_2000_rects_512", compose_two_thousand_rects),
        ("compose_4000_rects_512", compose_four_thousand_rects),
        (
            "tools_list_1000_round_trips",
            thousand_tools_list_round_trips,
        ),
    ];
    let mut failures = Vec::new();
    for (name, work) in cases {
        let measured = best_of_three_ms(work);
        let limit = max_ms(&baseline, name);
        if print {
            println!("perf {name}: measured_ms={measured:.2} max_ms={limit}");
        }
        if measured > limit as f64 {
            failures.push(format!("{name}: {measured:.1} ms exceeds max_ms {limit}"));
        }
    }
    assert!(failures.is_empty(), "perf gate failed: {failures:?}");
}
