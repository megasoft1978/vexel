//! Decode batching differential (WO-RUST-PIPELINE-PERF-01 AC4): documents that
//! enable root batching and ones that disable it (style, nested g, use,
//! clipPath, pattern) must keep the exact shape count and frame hash that
//! base_sha d0ae379c produced. `VEXEL_BATCH_PRINT=1` prints the values.
mod pipeline_common;
use pipeline_common::{rects, rects_with_style, sha256_hex};
use vexel_app::compose::compose_document;

const HEAD: &str = "<svg width='256' height='256' viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>";

fn rect(n: usize) -> String {
    format!(
        "<rect x='{}' y='{}' width='{}' height='9' fill='#{:06x}'{{extra}}/>\n",
        (n * 37) % 240,
        (n * 91) % 240,
        6 + n % 11,
        (n * 2_654_435_761) & 0xff_ffff
    )
}

fn body(count: usize, extra: &str) -> String {
    (0..count)
        .map(|n| rect(n).replace("{extra}", extra))
        .collect()
}

fn nested_group() -> Vec<u8> {
    format!("{HEAD}<g><g opacity='0.8'>{}</g></g></svg>", body(200, "")).into_bytes()
}

fn use_elements() -> Vec<u8> {
    let mut svg =
        format!("{HEAD}<defs><rect id='r' width='12' height='7' fill='#336699'/></defs>\n");
    svg.push_str(&body(130, ""));
    for n in 0..70 {
        svg.push_str(&format!(
            "<use xlink:href='#r' x='{}' y='{}'/>\n",
            (n * 29) % 240,
            (n * 53) % 240
        ));
    }
    svg.push_str("</svg>");
    svg.into_bytes()
}

fn clip_path() -> Vec<u8> {
    format!(
        "{HEAD}<defs><clipPath id='c'><rect x='0' y='0' width='128' height='256'/></clipPath></defs>\n{}</svg>",
        body(200, " clip-path='url(#c)'")
    )
    .into_bytes()
}

fn pattern() -> Vec<u8> {
    let mut svg = format!(
        "{HEAD}<defs><pattern id='p' width='8' height='8' patternUnits='userSpaceOnUse'><rect width='4' height='4' fill='#cc3300'/></pattern></defs>\n"
    );
    for n in 0..200 {
        svg.push_str(&format!(
            "<rect x='{}' y='{}' width='{}' height='9' fill='url(#p)'/>\n",
            (n * 37) % 240,
            (n * 91) % 240,
            6 + n % 11
        ));
    }
    svg.push_str("</svg>");
    svg.into_bytes()
}

/// (name, shapes, frame sha256 at 128x128), captured at base_sha.
const EXPECTED: [(&str, usize, &str); 6] = [
    (
        "R200",
        200,
        "6cf022623c5b2636d4b12b12f36cd437bc2c30a68198ae6545fda0d934245765",
    ),
    (
        "R200_style",
        200,
        "6cf022623c5b2636d4b12b12f36cd437bc2c30a68198ae6545fda0d934245765",
    ),
    (
        "nested_g",
        200,
        "5fa6c27f70c217c854dd84efeba967854aba7ba007da6fc816bff8c7db95c5b0",
    ),
    (
        "use",
        200,
        "25a065e6688515eb73242630bf541e1b02a21ea84cb72a2fe8fbf2dd6ec952e7",
    ),
    (
        "clipPath",
        200,
        "ea99e05e715239bb1082fa7eec3b3966110de8a6eca36496bc116f936a68d4c8",
    ),
    (
        "pattern",
        200,
        "63e874987f98e710c40e70e8b2417f89d619dde7bd7f994e189fede936f82447",
    ),
];

#[test]
fn batching_documents_keep_scene_counts_and_frames() {
    let documents = [
        ("R200", rects(200)),
        ("R200_style", rects_with_style(200)),
        ("nested_g", nested_group()),
        ("use", use_elements()),
        ("clipPath", clip_path()),
        ("pattern", pattern()),
    ];
    let print = std::env::var("VEXEL_BATCH_PRINT").is_ok_and(|v| v == "1");
    for ((name, source), (expected_name, shapes, hash)) in documents.iter().zip(EXPECTED) {
        assert_eq!(*name, expected_name);
        let composed = compose_document(source, 128, 128, &|| false)
            .unwrap_or_else(|e| panic!("{name} must compose: {e:?}"));
        let actual = sha256_hex(&composed.frame.rgba);
        if print {
            println!("    (\"{name}\", {}, \"{actual}\"),", composed.shapes.len());
            continue;
        }
        assert_eq!(composed.shapes.len(), shapes, "{name} shape count");
        assert_eq!(actual, hash, "{name} frame hash");
    }
}
