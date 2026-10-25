//! Authoring limits lifted for element_add: shapes beyond the canvas, long
//! path data, path presentation attributes, short hex colours, gradient
//! ellipses without a fill colour; oversize inputs stay rejected.

mod common;

use common::{Rig, no_grants};

fn rig() -> Rig {
    Rig::new(no_grants())
}

fn add(rig: &mut Rig, template: &str, attributes: &str, extra: &str) -> common::Reply {
    rig.doc_tool(
        "element_add",
        &format!(
            r#""parentId":"root","template":"{template}","attributes":{{{attributes}}}{extra}"#
        ),
    )
}

fn svg(rig: &mut Rig) -> String {
    rig.doc_tool("export_svg", "").ok().replace("\\\"", "\"")
}

#[test]
fn rect_and_ellipse_may_extend_past_the_canvas() {
    let mut rig = rig();
    add(
        &mut rig,
        "rect",
        r##""x":"-100","y":"-50","width":"2000","height":"900","fill":"#102030""##,
        "",
    )
    .ok();
    add(
        &mut rig,
        "ellipse",
        r##""cx":"600","cy":"700","rx":"900","ry":"300","fill":"#fa0""##,
        "",
    )
    .ok();
    let svg = svg(&mut rig);
    assert!(svg.contains(r#"width="2e3""#), "{svg}");
    // Beyond the coordinate bound is still refused.
    let reply = add(
        &mut rig,
        "rect",
        r##""x":"0","y":"0","width":"2000000","height":"10","fill":"#102030""##,
        "",
    );
    assert!(reply.is_error, "{}", reply.body);
}

#[test]
fn long_path_data_is_accepted_and_the_bound_is_enforced() {
    let mut rig = rig();
    let d = format!("M 0 0 {}", "L 1 1 ".repeat(5000));
    assert!(d.len() > 4096);
    add(
        &mut rig,
        "path",
        &format!(r##""d":"{d}","stroke":"#000""##),
        "",
    )
    .ok();
    let over = format!("M 0 0 {}", "L 1 1 ".repeat(45_000));
    assert!(over.len() > vexel_v1_document::MAX_PATH_DATA_BYTES);
    let reply = add(
        &mut rig,
        "path",
        &format!(r##""d":"{over}","stroke":"#000""##),
        "",
    );
    assert!(reply.is_error, "oversize path rejected");
    let segments = format!(
        "M 0 0 {}",
        "L 1 1 ".repeat(vexel_v1_document::MAX_PATH_SEGMENTS + 1)
    );
    assert!(segments.len() <= vexel_v1_document::MAX_PATH_DATA_BYTES);
    assert!(
        add(
            &mut rig,
            "path",
            &format!(r##""d":"{segments}","stroke":"#000""##),
            ""
        )
        .is_error
    );
}

#[test]
fn path_segment_bound_is_enforced_at_16384() {
    let mut rig = rig();
    // "M" counts as one segment: M + 16383 implicit H repeats = 16384 is the
    // accepted maximum. Implicit H keeps the argument small; an L-per-segment
    // string of 16383 segments (~98 KB) is refused before reaching the bound.
    let at_limit = format!("M 0 0 H{}", "1 ".repeat(16_383));
    assert!(at_limit.len() < vexel_v1_document::MAX_PATH_DATA_BYTES);
    add(
        &mut rig,
        "path",
        &format!(r##""d":"{at_limit}","stroke":"#000""##),
        "",
    )
    .ok();
    let over = format!("M 0 0 {}", "L 1 1 ".repeat(16_400));
    assert!(over.len() < vexel_v1_document::MAX_PATH_DATA_BYTES);
    let reply = add(
        &mut rig,
        "path",
        &format!(r##""d":"{over}","stroke":"#000""##),
        "",
    );
    assert!(reply.is_error, "over 16384 segments rejected");
}

#[test]
fn path_presentation_attributes_and_short_hex() {
    let mut rig = rig();
    add(
        &mut rig,
        "path",
        r##""d":"M 5 5 L 50 50","stroke":"#fa0","fill":"#000","stroke-width":"4","stroke-linecap":"round","stroke-linejoin":"bevel","fill-rule":"evenodd""##,
        "",
    )
    .ok();
    let svg = svg(&mut rig);
    for want in [
        r##"stroke="#ffaa00""##.to_owned(),
        r##"fill="#000000""##.to_owned(),
        r#"stroke-linecap="round""#.to_owned(),
        r#"stroke-linejoin="bevel""#.to_owned(),
        r#"fill-rule="evenodd""#.to_owned(),
    ] {
        assert!(svg.contains(&want), "{want} in {svg}");
    }
    let bad = add(
        &mut rig,
        "path",
        r##""d":"M 5 5 L 50 50","stroke":"#000","stroke-linecap":"wavy""##,
        "",
    );
    assert!(
        bad.is_error && bad.body.contains("stroke-linecap"),
        "{}",
        bad.body
    );
}

#[test]
fn gradient_ellipse_needs_no_fill_colour() {
    let mut rig = rig();
    let body = rig
        .doc_tool(
            "gradient_add",
            r#""kind":"radial","stops":"0:#ff0000:1,1:#0000ff:1""#,
        )
        .ok()
        .to_owned();
    let id = body
        .split("\"gradientId\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    add(
        &mut rig,
        "ellipse",
        r#""cx":"100","cy":"100","rx":"50","ry":"30""#,
        &format!(r#","gradientId":"{id}""#),
    )
    .ok();
    assert!(svg(&mut rig).contains(&format!("url(#{id})")));
    // Without a gradient the fill stays required.
    assert!(
        add(
            &mut rig,
            "ellipse",
            r#""cx":"1","cy":"1","rx":"5","ry":"5""#,
            ""
        )
        .is_error
    );
}

#[test]
fn element_group_wraps_up_to_1024_ids_and_rejects_more() {
    let mut rig = rig();
    // Batches of 205 adds keep under the per-minute request limit.
    let ids: Vec<String> = (0..1025).map(|i| format!("g{i}")).collect();
    for chunk in ids.chunks(205) {
        let ops: Vec<String> = chunk
            .iter()
            .map(|id| {
                format!(
                    r##"{{"tool":"element_add","arguments":{{"documentId":"preview-document","parentId":"root","id":"{id}","template":"rect","attributes":{{"x":"1","y":"1","width":"4","height":"4","fill":"#112233"}}}}}}"##
                )
            })
            .collect();
        rig.doc_tool("batch", &format!(r#""ops":[{}]"#, ops.join(",")))
            .ok();
    }
    let quoted: Vec<String> = ids.iter().map(|id| format!("\"{id}\"")).collect();
    let over = rig.doc_tool(
        "element_group",
        &format!(r#""elementIds":[{}]"#, quoted.join(",")),
    );
    assert!(over.is_error, "1025 ids rejected");
    assert!(over.body.contains("1-1024"), "{}", over.body);
    let ok = rig.doc_tool(
        "element_group",
        &format!(r#""elementIds":[{}]"#, quoted[..1024].join(",")),
    );
    assert!(ok.ok().contains("elementId"));
}

#[test]
fn fragment_with_an_undefined_filter_says_so() {
    let mut rig = rig();
    add(
        &mut rig,
        "rect",
        r##""x":"1","y":"1","width":"9","height":"9","fill":"#112233""##,
        "",
    )
    .ok();
    let before = rig.state();
    let reply = rig.doc_tool(
        "element_add_fragment",
        r##""parentId":"root","xml":"<rect id=\"r\" x=\"10\" y=\"10\" width=\"20\" height=\"20\" fill=\"#ff0000\" filter=\"url(#nope)\"/>""##,
    );
    assert!(reply.is_error);
    assert!(reply.body.contains("undefined filter"), "{}", reply.body);
    assert!(!reply.body.contains("mix-blend-mode"), "{}", reply.body);
    assert_eq!(rig.state().source, before.source, "edit undone");
}
