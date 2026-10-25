//! 600 elements + 100 tracks: `scene_apply` against the old path (`batch`
//! of `element_add` entries, then `native_timeline_tracks_set`).
//! Run: `cargo test -p vexel-app --release --test mcp_scene_bench -- --ignored --nocapture`

mod common;

use common::{Rig, no_grants};
use std::time::Instant;

const ELEMENTS: usize = 600;
const TRACKS: usize = 100;

fn rect(i: usize) -> String {
    format!(
        r##"{{"template":"rect","id":"r{i}","attributes":{{"x":"{}","y":"{}","width":"10","height":"10","fill":"#336699"}}}}"##,
        (i % 30) * 15,
        (i / 30) * 15
    )
}

fn tracks() -> String {
    (0..TRACKS)
        .map(|i| {
            format!(
                r#"{{"trackId":"t{i}","targetId":"r{i}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1,"keys":[{{"tick":12,"value":0.5}},{{"tick":24,"value":1}}]}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
#[ignore]
fn scene_apply_against_batch_and_tracks_set() {
    let mut old = Rig::new(no_grants());
    let start = Instant::now();
    for chunk in (0..ELEMENTS).collect::<Vec<_>>().chunks(200) {
        let ops: Vec<String> = chunk
            .iter()
            .map(|i| {
                let e = rect(*i);
                let attrs = &e[e.find("\"attributes\"").unwrap()..e.len() - 1];
                format!(
                    r#"{{"tool":"element_add","arguments":{{"parentId":"root","template":"rect","id":"r{i}",{attrs}}}}}"#
                )
            })
            .collect();
        old.doc_tool("batch", &format!(r#""ops":[{}]"#, ops.join(",")))
            .ok();
    }
    let elements_ms = start.elapsed().as_secs_f64() * 1000.0;
    old.doc_tool(
        "native_timeline_tracks_set",
        &format!(r#""tracks":[{}]"#, tracks()),
    )
    .ok();
    let old_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut new = Rig::new(no_grants());
    let elements: Vec<String> = (0..ELEMENTS).map(rect).collect();
    let start = Instant::now();
    new.doc_tool(
        "scene_apply",
        &format!(
            r#""elements":[{}],"tracks":[{}]"#,
            elements.join(","),
            tracks()
        ),
    )
    .ok();
    let new_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "old path: {old_ms:.0} ms ({elements_ms:.0} ms elements in 3 batch calls + tracks_set), scene_apply: {new_ms:.0} ms"
    );
    assert_eq!(
        old.app.source_bytes().unwrap().len(),
        new.app.source_bytes().unwrap().len()
    );
}
