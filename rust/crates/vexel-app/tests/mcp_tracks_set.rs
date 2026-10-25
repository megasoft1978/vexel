//! `native_timeline_tracks_set`: many tracks and keys in one atomic, undoable
//! edit that matches the per-track tools, names the failing track by index,
//! rolls back inside an atomic batch and rejects malformed or oversize input.

mod common;

use common::{Dir, Rig, policy_for};

fn rig(count: usize) -> (Dir, Rig, Vec<String>) {
    let dir = Dir::new();
    let mut rig = Rig::new(policy_for(&[&dir.0]));
    let ids = (0..count)
        .map(|i| rig.add_rect((i as u32 % 32) * 15, (i as u32 / 32) * 15, "#336699"))
        .collect();
    (dir, rig, ids)
}

fn entry(id: &str, target: &str, keys: &str) -> String {
    format!(
        r#"{{"trackId":"{id}","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1,"keys":[{keys}]}}"#
    )
}

const KEYS: &str = r#"{"tick":9,"value":0.5},{"tick":3,"value":0.25,"interp":"ease_in_out"},{"tick":9,"value":0.75}"#;

fn set(rig: &mut Rig, tracks: &[String]) -> common::Reply {
    rig.doc_tool(
        "native_timeline_tracks_set",
        &format!(r#""tracks":[{}]"#, tracks.join(",")),
    )
}

#[test]
fn matches_per_track_calls_and_is_one_undo_step() {
    let (_dir, mut rig, ids) = rig(2);
    let reply = set(
        &mut rig,
        &[entry("a", &ids[0], KEYS), entry("b", &ids[1], "")],
    );
    assert!(reply.ok().contains("\"tracks\":2"), "{}", reply.body);
    let bulk = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    rig.doc_tool("native_timeline_undo", "").ok();
    let gone = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert!(!gone.contains("\"a\"") && !gone.contains("\"b\""), "{gone}");

    for (id, target, keys) in [("a", &ids[0], true), ("b", &ids[1], false)] {
        rig.doc_tool(
            "native_timeline_add_property_track",
            &format!(
                r#""trackId":"{id}","targetId":"{target}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
            ),
        )
        .ok();
        if keys {
            for (tick, value, ease) in [
                (9, "0.5", ""),
                (3, "0.25", r#","ease":"ease_in_out""#),
                (9, "0.75", ""),
            ] {
                rig.doc_tool(
                    "native_timeline_set_property_key",
                    &format!(r#""trackId":"{id}","tick":{tick},"value":{value}{ease}"#),
                )
                .ok();
            }
        }
    }
    let single = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(bulk, single);
}

#[test]
fn rejection_names_the_track_and_changes_nothing() {
    let (_dir, mut rig, ids) = rig(3);
    set(&mut rig, &[entry("keep", &ids[0], "")]).ok();
    let state = rig.state();
    let bad_target = entry("x", "no-such-element", "");
    let cases = [
        (vec![entry("ok", &ids[1], ""), bad_target], "tracks[1]"),
        (
            vec![entry("ok", &ids[1], ""), entry("keep", &ids[2], "")],
            "tracks[1]",
        ),
        (
            vec![
                entry("ok", &ids[1], ""),
                entry("dup", &ids[2], ""),
                entry("dup", &ids[2], ""),
            ],
            "tracks[2]",
        ),
        (
            vec![entry("late", &ids[1], r#"{"tick":900,"value":1}"#)],
            "tracks[0]",
        ),
        (
            vec![
                entry("ok", &ids[1], ""),
                entry("bad", &ids[2], r#"{"tick":2,"value":[1,2]}"#),
            ],
            "tracks[1]",
        ),
        (vec![entry("bad id!", &ids[1], "")], "tracks[0]"),
    ];
    for (tracks, expect) in cases {
        let tracks = tracks;
        let reply = set(&mut rig, &tracks);
        assert!(
            reply.error().contains(expect),
            "{expect}: {} for {tracks:?}",
            reply.body
        );
        assert_eq!(rig.state(), state, "a rejected call changes nothing");
    }
}

#[test]
fn malformed_and_oversize_requests_are_rejected() {
    let (_dir, mut rig, ids) = rig(1);
    let state = rig.state();
    let good = entry("t", &ids[0], "");
    let many: Vec<String> = (0..257)
        .map(|i| entry(&format!("t{i}"), &ids[0], ""))
        .collect();
    let keys = vec![r#"{"tick":1,"value":1}"#; 257].join(",");
    let bodies = [
        r#""tracks":[]"#.to_owned(),
        r#""tracks":"nope""#.to_owned(),
        r#""tracks":{}"#.to_owned(),
        r#""tracks":[1]"#.to_owned(),
        r#""tracks":[[]]"#.to_owned(),
        format!(r#""tracks":[{}]"#, many.join(",")),
        format!(r#""tracks":[{}]"#, entry("big", &ids[0], &keys)),
        format!(r#""tracks":[{}],"extra":1"#, good),
        format!(
            r#""tracks":[{}]"#,
            good.replace("\"keys\":[]", "\"documentId\":\"x\"")
        ),
        format!(r#""tracks":[{}]"#, good.replace("\"trackId\":\"t\",", "")),
        format!(
            r#""tracks":[{}]"#,
            good.replace("\"value\":1", "\"value\":\"junk\"")
        ),
        format!(
            r#""tracks":[{}]"#,
            good.replace("\"keys\":[]", "\"keys\":{}")
        ),
    ];
    for body in bodies {
        let reply = rig.doc_tool("native_timeline_tracks_set", &body);
        assert!(reply.is_error, "accepted {}", &body[..body.len().min(120)]);
        assert_eq!(rig.state(), state);
    }
    let missing = rig.tool(
        "native_timeline_tracks_set",
        r#"{"documentId":"preview-document"}"#,
    );
    assert!(missing.is_error);
}

#[test]
fn rolls_back_inside_an_atomic_batch() {
    let (_dir, mut rig, ids) = rig(2);
    let state = rig.state();
    let ops = format!(
        r#"{{"tool":"native_timeline_tracks_set","arguments":{{"tracks":[{}]}}}},{{"tool":"native_timeline_remove_track","arguments":{{"trackId":"missing"}}}}"#,
        entry("a", &ids[0], KEYS)
    );
    let reply = rig.tool(
        "batch",
        &format!(r#"{{"documentId":"preview-document","ops":[{ops}]}}"#),
    );
    assert!(reply.ok().contains("\"failedIndex\":1"), "{}", reply.body);
    assert_eq!(rig.state(), state);
    let ok = rig.tool(
        "batch",
        &format!(
            r#"{{"documentId":"preview-document","ops":[{{"tool":"native_timeline_tracks_set","arguments":{{"tracks":[{}]}}}}]}}"#,
            entry("a", &ids[0], KEYS)
        ),
    );
    assert!(ok.ok().contains("\"ok\":true"), "{}", ok.body);
}

#[test]
fn flushes_a_full_scene_in_one_call() {
    let (_dir, mut rig, ids) = rig(120);
    let tracks: Vec<String> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let keys: Vec<String> = (1..=12)
                .map(|k| format!(r#"{{"tick":{k},"value":[{i}.5,2,{k}.5,1,1,10,10]}}"#))
                .collect();
            format!(
                r#"{{"trackId":"m{i}","targetId":"{id}","property":"transform","valueType":"trs","interpolation":"cubic","value":[0,0,0,1,1,10,10],"keys":[{}]}}"#,
                keys.join(",")
            )
        })
        .collect();
    let start = std::time::Instant::now();
    let reply = set(&mut rig, &tracks);
    assert!(reply.ok().contains("\"tracks\":120"), "{}", reply.body);
    assert!(
        start.elapsed().as_secs_f64() < 20.0,
        "120 tracks took {:?}",
        start.elapsed()
    );
    let get = rig.doc_tool("native_timeline_get", "").ok().to_owned();
    assert_eq!(get.matches("\"kind\":\"property\"").count(), 120);
}
