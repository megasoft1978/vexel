//! The preview `batch` tool through the real JSON-RPC layer: one undo step,
//! atomic rollback, partial non-atomic runs, rejections and a timing note.

mod common;

use common::{Rig, no_grants};
use std::time::Instant;

const DOC: &str = "preview-document";

fn add_op(index: u32, id: &str) -> String {
    let id = if id.is_empty() {
        String::new()
    } else {
        format!(r#","id":"{id}""#)
    };
    format!(
        r##"{{"tool":"element_add","arguments":{{"documentId":"{DOC}","parentId":"root","template":"rect","attributes":{{"x":"{}","y":"5","width":"10","height":"10","fill":"#336699"}}{id}}}}}"##,
        index % 400
    )
}

fn track_op(track: &str, target: &str) -> String {
    format!(
        r#"{{"tool":"native_timeline_add_property_track","arguments":{{"documentId":"{DOC}","trackId":"{track}","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1}}}}"#
    )
}

fn batch(rig: &mut Rig, atomic: Option<bool>, ops: &[String]) -> common::Reply {
    let atomic = atomic.map_or(String::new(), |a| format!(r#""atomic":{a},"#));
    rig.tool(
        "batch",
        &format!(
            r#"{{"documentId":"{DOC}",{atomic}"ops":[{}]}}"#,
            ops.join(",")
        ),
    )
}

fn undo(rig: &mut Rig) {
    rig.doc_tool("undo", "").ok();
}

#[test]
fn batch_is_listed() {
    let mut rig = Rig::new(no_grants());
    assert!(rig.list_tools().iter().any(|n| n == "batch"));
}

#[test]
fn fifty_elements_and_tracks_are_one_undo_step() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let mut ops: Vec<String> = (0..50).map(|i| add_op(i, &format!("box-{i}"))).collect();
    // Later ops reference ids created earlier in the same batch.
    ops.push(track_op("fade-a", "box-0"));
    ops.push(track_op("fade-b", "box-49"));
    let reply = batch(&mut rig, None, &ops);
    let body = reply.ok();
    assert!(body.contains(r#""ok":true"#), "{body}");
    assert_eq!(body.matches(r#""tool":"element_add""#).count(), 50);
    let after = rig.state();
    assert_ne!(after.source, before.source);
    assert!(rig.app.document().history_count() > 0);
    let tracks = rig.doc_tool("native_timeline_get", "");
    assert!(tracks.ok().contains("fade-a"));

    undo(&mut rig);
    assert_eq!(rig.state().source, before.source, "one undo reverts all");
    assert!(!rig.app.can_undo());
    rig.doc_tool("redo", "").ok();
    assert_eq!(rig.state().source, after.source);
}

#[test]
fn atomic_failure_at_op_7_rolls_everything_back() {
    let mut rig = Rig::new(no_grants());
    rig.add_rect(1, 1, "#112233");
    let before = rig.state();
    let mut ops: Vec<String> = (0..6).map(|i| add_op(i, "")).collect();
    ops.push(format!(
        r#"{{"tool":"element_delete","arguments":{{"documentId":"{DOC}","elementId":"vexel-object-999"}}}}"#
    ));
    ops.extend((7..10).map(|i| add_op(i, "")));
    let reply = batch(&mut rig, Some(true), &ops);
    let body = reply.ok();
    assert!(body.contains(r#""ok":false"#), "{body}");
    assert!(body.contains(r#""failedIndex":6"#), "{body}");
    assert_eq!(body.matches(r#""tool":"element_add""#).count(), 6, "{body}");
    let after = rig.state();
    assert_eq!(after.source, before.source, "document bytes identical");
    assert!(!rig.app.can_redo());
    assert!(rig.app.can_undo(), "earlier history survives");
    undo(&mut rig);
    assert!(!rig.app.can_undo());
}

#[test]
fn atomic_rollback_also_reverts_timeline_payload() {
    let mut rig = Rig::new(no_grants());
    let id = rig.add_rect(1, 1, "#112233");
    let before_timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    let bad = format!(
        r#"{{"tool":"element_delete","arguments":{{"documentId":"{DOC}","elementId":"vexel-object-999"}}}}"#
    );
    let ops = vec![add_op(0, "extra"), track_op("fade-z", &id), bad];
    let reply = batch(&mut rig, None, &ops);
    assert!(reply.ok().contains(r#""failedIndex":2"#));
    let after_timeline = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(after_timeline, before_timeline);
    assert!(!after_timeline.contains("fade-z"));
}

#[test]
fn non_atomic_continues_past_failures_in_one_undo_step() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let bad = format!(
        r#"{{"tool":"element_delete","arguments":{{"documentId":"{DOC}","elementId":"vexel-object-999"}}}}"#
    );
    let ops = vec![add_op(0, ""), bad, add_op(1, "")];
    let reply = batch(&mut rig, Some(false), &ops);
    let body = reply.ok();
    assert!(body.contains(r#""ok":false"#), "{body}");
    assert!(body.contains(r#""failedCount":1"#), "{body}");
    let source = String::from_utf8(rig.state().source).unwrap();
    assert!(source.contains("vexel-object-1") && source.contains("vexel-object-2"));
    undo(&mut rig);
    assert_eq!(rig.state().source, before.source);
}

#[test]
fn rejects_nested_batch_file_tools_and_bad_shapes() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let nested = format!(
        r#"{{"tool":"batch","arguments":{{"documentId":"{DOC}","ops":[{}]}}}}"#,
        add_op(0, "")
    );
    assert!(batch(&mut rig, None, &[nested]).is_error);
    let save_as = format!(
        r#"{{"tool":"save_as","arguments":{{"documentId":"{DOC}","path":"/tmp/x.vexel"}}}}"#
    );
    assert!(batch(&mut rig, None, &[save_as]).is_error);
    let ops: Vec<String> = (0..257).map(|i| add_op(i, "")).collect();
    assert!(batch(&mut rig, None, &ops).is_error);
    assert!(batch(&mut rig, None, &[]).is_error);
    let unknown = format!(r#"{{"tool":"no_such_tool","arguments":{{"documentId":"{DOC}"}}}}"#);
    assert!(batch(&mut rig, None, &[unknown]).is_error);
    // A malformed op later in the batch fails fast, naming its index, and
    // nothing before it is applied.
    let malformed = format!(
        r#"{{"tool":"attribute_set","arguments":{{"documentId":"{DOC}","elementId":"vexel-object-1","name":"bogus","value":"1"}}}}"#
    );
    let reply = batch(&mut rig, None, &[add_op(0, ""), add_op(1, ""), malformed]);
    assert!(reply.error().contains("batch op 2"), "{}", reply.body);
    assert_eq!(rig.state(), before);
}

#[test]
fn batch_is_not_slower_than_single_calls() {
    let single = {
        let mut rig = Rig::new(no_grants());
        let start = Instant::now();
        for i in 0..100 {
            rig.tool(
                "element_add",
                &format!(
                    r##"{{"documentId":"{DOC}","parentId":"root","template":"rect","attributes":{{"x":"{i}","y":"5","width":"10","height":"10","fill":"#336699"}}}}"##
                ),
            )
            .ok();
        }
        start.elapsed()
    };
    let batched = {
        let mut rig = Rig::new(no_grants());
        let ops: Vec<String> = (0..100).map(|i| add_op(i, "")).collect();
        let start = Instant::now();
        batch(&mut rig, None, &ops).ok();
        start.elapsed()
    };
    println!("100 element_add: {single:?} as 100 calls, {batched:?} as one batch");
    assert!(batched <= single, "{batched:?} > {single:?}");
}

fn bare_add(index: u32) -> String {
    format!(
        r##"{{"tool":"element_add","arguments":{{"parentId":"root","template":"rect","attributes":{{"x":"{index}","y":"5","width":"10","height":"10","fill":"#336699"}}}}}}"##
    )
}

#[test]
fn ops_without_document_id_inherit_the_batch_document() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let ops: Vec<String> = (0..3).map(bare_add).collect();
    let body = batch(&mut rig, None, &ops).ok().to_owned();
    assert!(body.contains(r#""ok":true"#), "{body}");
    assert_eq!(body.matches(r#""tool":"element_add""#).count(), 3, "{body}");
    undo(&mut rig);
    assert_eq!(rig.state().source, before.source, "one undo reverts all");

    // Atomic: a failing op after inherited ops rolls them all back.
    let bad = r#"{"tool":"element_delete","arguments":{"elementId":"vexel-object-999"}}"#;
    let reply = batch(
        &mut rig,
        Some(true),
        &[bare_add(0), bare_add(1), bad.to_owned()],
    );
    assert!(reply.ok().contains(r#""failedIndex":2"#), "{}", reply.body);
    assert_eq!(rig.state().source, before.source);
}

#[test]
fn mismatched_op_document_id_is_rejected_with_op_index() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let other = r#"{"tool":"element_delete","arguments":{"documentId":"other-doc","elementId":"vexel-object-1"}}"#;
    let reply = batch(&mut rig, None, &[bare_add(0), other.to_owned()]);
    let error = reply.error();
    assert!(error.contains("batch op 1 (element_delete)"), "{error}");
    assert!(error.contains("documentId differs"), "{error}");
    assert_eq!(rig.state(), before);
}

#[test]
fn op_errors_name_index_tool_and_argument() {
    let mut rig = Rig::new(no_grants());
    let before = rig.state();
    let cases = [
        (
            r#"{"tool":"element_add","arguments":{"parentId":"root"}}"#,
            "batch op 1 (element_add): missing required argument 'template'",
        ),
        (
            r#"{"tool":"no_such_tool","arguments":{}}"#,
            "batch op 1 (no_such_tool): unknown tool",
        ),
        (
            r#"{"tool":"element_add","arguments":[]}"#,
            "batch op 1 (element_add): 'arguments' must be an object",
        ),
        (
            r#"{"tool":"element_add","arguments":{"parentId":"root","template":"rect"},"id":7}"#,
            "batch op 1: unexpected key 'id'",
        ),
        (
            r#"{"tool":"element_add","arguments":{"parentId":"root","template":"rect","bogus":"1"}}"#,
            "batch op 1 (element_add): unexpected argument 'bogus'",
        ),
    ];
    for (op, expected) in cases {
        let reply = batch(&mut rig, None, &[bare_add(0), op.to_owned()]);
        let error = reply.error();
        assert!(error.contains(expected), "{error}");
    }
    assert_eq!(rig.state(), before);
}
