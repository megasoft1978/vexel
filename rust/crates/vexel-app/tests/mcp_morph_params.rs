//! Path morph (`d`, `points`), animated filter parameters and gradient stops
//! over MCP: `render_png` at a key tick equals a static document drawn with
//! that key's value, byte for byte.

mod common;

use common::{Dir, Rig, policy_for};
use std::fs;

const A: &str = "M20 20 L100 20 L100 100 L20 100 Z";
const B: &str = "M40 40 L120 40 L120 120 L40 120 Z";
const C: &str = "m 10 90 l 40 -60 l 40 60 z";
const POLY: [&str; 2] = ["10,10 100,10 100,100", "30,30 120,30 120,120"];

fn svg(inner: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">{inner}</svg>"##
    )
}

fn open(dir: &Dir, name: &str, inner: &str) -> Rig {
    let path = dir.file(name);
    fs::write(&path, svg(inner)).unwrap();
    Rig::with_open(policy_for(&[&dir.0]), &path).unwrap()
}

fn png(rig: &mut Rig, dir: &Dir, name: &str, time: &str) -> Vec<u8> {
    let extra = if time.is_empty() {
        String::new()
    } else {
        format!(r#","time":{time}"#)
    };
    rig.doc_tool(
        "render_png",
        &format!(r#""path":"{}"{extra}"#, dir.text(name)),
    )
    .ok();
    fs::read(dir.file(name)).unwrap()
}

fn track(rig: &mut Rig, id: &str, target: &str, prop: &str, ty: &str, value: &str, keys: &str) {
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"{id}","targetId":"{target}","property":"{prop}","valueType":"{ty}","interpolation":"linear","value":{value},"keys":{keys}"#
        ),
    )
    .ok();
}

/// The expected frame: the same shapes with the key's value written in the
/// source, rendered through the same native timeline route (a constant
/// `fill-opacity` track on `target`) so only the animated value differs.
fn static_png(dir: &Dir, name: &str, inner: &str) -> Vec<u8> {
    let target = if inner.contains(r#"id="p""#) { "p" } else { "r" };
    let mut rig = open(dir, &format!("s_{name}.svg"), inner);
    track(&mut rig, "const", target, "fill-opacity", "scalar", "1", "[]");
    png(&mut rig, dir, name, "0")
}

#[test]
fn path_morph_key_ticks_match_static_renders() {
    let dir = Dir::new();
    let tpl = |d: &str| format!(r##"<path id="p" d="{d}" fill="#c33" stroke="#000"/>"##);
    let mut rig = open(&dir, "m.svg", &tpl(A));
    track(
        &mut rig,
        "morph",
        "p",
        "d",
        "path",
        &format!(r#""{A}""#),
        &format!(r#"[{{"tick":12,"value":"{B}"}},{{"tick":24,"value":"{A}"}}]"#),
    );
    for (n, (time, d)) in [("0", A), ("0.5", B), ("1", A)].into_iter().enumerate() {
        let got = png(&mut rig, &dir, &format!("g{n}.png"), time);
        assert_eq!(got, static_png(&dir, &format!("w{n}.png"), &tpl(d)), "tick {time}");
    }
    // Halfway between two keys is the exact per-coordinate midpoint.
    let mid = png(&mut rig, &dir, "mid.png", "0.25");
    let want = "M 30 30 L 110 30 L 110 110 L 30 110 Z";
    assert_eq!(mid, static_png(&dir, "midw.png", &tpl(want)));
}

#[test]
fn relative_commands_normalise_and_points_morph() {
    let dir = Dir::new();
    let mut rig = open(
        &dir,
        "r.svg",
        r##"<path id="p" d="M 0 0 L 1 1 L 2 0 Z" fill="#339"/><polygon id="g" points="1,1 2,2 3,1" fill="#393"/>"##,
    );
    track(&mut rig, "d", "p", "d", "path", r#""m 10 90 l 40 -60 l 40 60 z""#, r#"[{"tick":12,"value":"M 20 90 L 60 40 L 100 90 Z"}]"#);
    track(&mut rig, "pts", "g", "points", "path", &format!(r#""{}""#, POLY[0]), &format!(r#"[{{"tick":12,"value":"{}"}}]"#, POLY[1]));
    let want = static_png(
        &dir,
        "w.png",
        r##"<path id="p" d="M 20 90 L 60 40 L 100 90 Z" fill="#339"/><polygon id="g" points="30,30 120,30 120,120" fill="#393"/>"##,
    );
    assert_eq!(png(&mut rig, &dir, "g.png", "0.5"), want);
    let start = static_png(
        &dir,
        "s.png",
        &format!(r##"<path id="p" d="{C}" fill="#339"/><polygon id="g" points="{}" fill="#393"/>"##, POLY[0]),
    );
    assert_eq!(png(&mut rig, &dir, "t0.png", "0"), start);
}

#[test]
fn mismatched_structure_and_wrong_targets_are_rejected() {
    let dir = Dir::new();
    let mut rig = open(&dir, "e.svg", r##"<path id="p" d="M0 0 L9 9 Z"/><rect id="r" width="5" height="5"/>"##);
    track(&mut rig, "m", "p", "d", "path", r#""M0 0 L9 9 Z""#, "[]");
    let state = rig.state();
    for bad in ["M0 0 L9 9 L5 5 Z", "M0 0 Q9 9 5 5 Z", "M0 0 A5 5 0 0 0 9 9 Z", "L1 1", "nonsense"] {
        let reply = rig.doc_tool("native_timeline_set_property_key", &format!(r#""trackId":"m","tick":5,"value":"{bad}""#));
        assert!(reply.is_error, "{bad}: {}", reply.body);
    }
    assert_eq!(rig.state(), state, "rejected keys change nothing");
    let wrong = rig.doc_tool("native_timeline_add_property_track", r#""trackId":"x","targetId":"r","property":"d","valueType":"path","interpolation":"linear","value":"M0 0 L1 1""#);
    assert!(wrong.error().contains("<path>"), "{}", wrong.body);
    let bulk = rig.doc_tool("native_timeline_add_property_track", r#""trackId":"y","targetId":"p","property":"d","valueType":"path","interpolation":"linear","value":"M0 0 L9 9 Z","keys":[{"tick":3,"value":"M0 0 L9 9 L1 1 Z"}]"#);
    assert!(bulk.is_error, "{}", bulk.body);
    // Relative form of the same structure is accepted.
    let ok = rig.doc_tool("native_timeline_set_property_key", r#""trackId":"m","tick":5,"value":"m 1 1 l 3 3 z""#);
    ok.ok();
}

#[test]
fn filter_params_and_gradient_stops_match_static_renders() {
    let dir = Dir::new();
    let defs = |dev: &str, dx: &str, off: &str, color: &str| {
        format!(
            r##"<defs><filter id="f" x="-30%" y="-30%" width="160%" height="160%"><feOffset id="off" dx="{dx}" dy="0" result="o"/><feGaussianBlur id="blur" stdDeviation="{dev}"/></filter><linearGradient id="g"><stop id="s0" offset="{off}" stop-color="{color}" stop-opacity="1"/><stop id="s1" offset="1" stop-color="#0000ff"/></linearGradient></defs><rect id="r" x="30" y="30" width="60" height="60" fill="url(#g)" filter="url(#f)"/>"##
        )
    };
    let mut rig = open(&dir, "f.svg", &defs("1", "0", "0", "#ff0000"));
    track(&mut rig, "blur", "blur", "stdDeviation", "scalar", "1", r#"[{"tick":12,"value":6}]"#);
    track(&mut rig, "dx", "off", "dx", "scalar", "0", r#"[{"tick":12,"value":5}]"#);
    track(&mut rig, "off", "s0", "offset", "scalar", "0", r#"[{"tick":12,"value":0.6}]"#);
    track(&mut rig, "col", "s0", "stop-color", "rgba", "[1,0,0,1]", r#"[{"tick":12,"value":[0,1,0,1]}]"#);
    let want = static_png(&dir, "w.png", &defs("6", "5", "0.6", "#00ff00"));
    assert_eq!(png(&mut rig, &dir, "g.png", "0.5"), want);
    assert_eq!(png(&mut rig, &dir, "g0.png", "0"), static_png(&dir, "w0.png", &defs("1", "0", "0", "#ff0000")));
    // Offsets stay monotonic: s0 above s1's static 1 is clamped, not rejected.
    rig.doc_tool("native_timeline_set_property_key", r#""trackId":"off","tick":24,"value":1.7"#).ok();
    let clamped = png(&mut rig, &dir, "c.png", "1");
    assert_eq!(clamped, static_png(&dir, "cw.png", &defs("6", "5", "1", "#00ff00")));
    // Limits: blur above 32, wrong primitive.
    let bad = rig.doc_tool("native_timeline_set_property_key", r#""trackId":"blur","tick":30,"value":33"#);
    assert!(bad.is_error, "{}", bad.body);
    let wrong = rig.doc_tool("native_timeline_add_property_track", r#""trackId":"w","targetId":"r","property":"stdDeviation","valueType":"scalar","interpolation":"linear","value":1"#);
    assert!(wrong.error().contains("feGaussianBlur"), "{}", wrong.body);
}

