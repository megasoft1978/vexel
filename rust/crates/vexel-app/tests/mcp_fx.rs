//! `effect_apply` through MCP: every preset validates, applies in one undoable
//! edit, changes the render at three ticks, removes cleanly by idPrefix and
//! refuses hostile parameters without touching the document.

mod common;

use common::{Rig, element_id, no_grants};
use vexel_app::animation::native_export::NativeExportSource;

/// White page with a red "hero" group and a blue shape, 512x512.
fn scene() -> (Rig, String) {
    let mut rig = Rig::new(no_grants());
    let add = |rig: &mut Rig, attrs: &str| {
        element_id(
            rig.doc_tool(
                "element_add",
                &format!(r#""parentId":"root","template":"rect","attributes":{{{attrs}}}"#),
            )
            .ok(),
        )
    };
    add(
        &mut rig,
        r##""x":"0","y":"0","width":"512","height":"512","fill":"#808080""##,
    );
    let hero = add(
        &mut rig,
        r##""x":"180","y":"180","width":"150","height":"150","fill":"#cc2222""##,
    );
    let hero = element_id(
        rig.doc_tool("element_group", &format!(r#""elementId":"{hero}""#))
            .ok(),
    );
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"keep","targetId":"{hero}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
    add(
        &mut rig,
        r##""x":"40","y":"60","width":"60","height":"60","fill":"#2244cc""##,
    );
    (rig, hero)
}

fn source(rig: &Rig) -> String {
    String::from_utf8(rig.app.source_bytes().unwrap()).unwrap()
}

fn frame(rig: &Rig, tick: u32) -> Vec<u8> {
    let native = NativeExportSource::from_document(&rig.app).unwrap();
    native
        .render_still_at(f64::from(tick) / 24.0, Ok)
        .unwrap()
        .3
}

/// Pixels whose RGBA differ.
fn diff(a: &[u8], b: &[u8]) -> usize {
    a.chunks(4).zip(b.chunks(4)).filter(|(x, y)| x != y).count()
}

fn timeline(rig: &mut Rig) -> String {
    rig.doc_tool("native_timeline_get", "").ok().to_owned()
}

/// (preset, arguments after the preset, whether the target is `hero`)
const CASES: &[(&str, &str, bool)] = &[
    ("speed_lines", r#""count":16,"seed":7"#, false),
    ("glow", r#""radius":10,"intensity":0.9"#, true),
    ("chromatic_split", r#""amount":12,"angle":20"#, true),
    ("film_grain", r#""intensity":0.8,"seed":3"#, false),
    ("halftone", r#""size":10,"dot":0.2,"dotTo":0.8"#, false),
    ("vhs_scanlines", r#""pitch":6,"jitter":4"#, false),
    (
        "motion_blur_dir",
        r#""amount":30,"angle":0,"copies":5"#,
        true,
    ),
    (
        "smear",
        r#""start":2,"ticks":12,"stretch":2,"trail":2"#,
        true,
    ),
    ("vignette", r#""intensity":0.9"#, false),
    ("flash", r#""start":2,"attack":1,"falloff":12"#, false),
];

fn args(preset: &str, extra: &str, hero: Option<&str>) -> String {
    let target = hero.map_or(String::new(), |h| format!(r#","target":"{h}""#));
    format!(r#""preset":"{preset}"{target},{extra}"#)
}

#[test]
fn every_preset_applies_changes_the_render_undoes_and_removes() {
    for (preset, extra, targeted) in CASES {
        let (mut rig, hero) = scene();
        let before = source(&rig);
        let base: Vec<Vec<u8>> = [3, 6, 9].iter().map(|t| frame(&rig, *t)).collect();
        let reply = rig
            .doc_tool(
                "effect_apply",
                &args(preset, extra, targeted.then_some(&hero)),
            )
            .ok()
            .to_owned();
        assert!(reply.contains("\"tracks\":"), "{preset}: {reply}");
        let prefix = format!("fx-{preset}-1");
        assert!(
            reply.contains(&format!("\"idPrefix\":\"{prefix}\"")),
            "{reply}"
        );
        assert!(
            timeline(&mut rig).contains(&format!("{prefix}-")),
            "{preset}"
        );
        let shown: Vec<Vec<u8>> = [3, 6, 9].iter().map(|t| frame(&rig, *t)).collect();
        for (i, (a, b)) in base.iter().zip(&shown).enumerate() {
            let changed = diff(a, b);
            assert!(
                changed >= 200,
                "{preset}: tick {} changed only {changed} pixels",
                3 + i * 3
            );
        }
        // Deterministic: the same call on a fresh document gives the same source.
        let (mut again, hero2) = scene();
        again
            .doc_tool(
                "effect_apply",
                &args(preset, extra, targeted.then_some(&hero2)),
            )
            .ok();
        assert_eq!(source(&again), source(&rig), "{preset}: not deterministic");
        assert_eq!(frame(&again, 6), shown[1], "{preset}: render differs");
        // Remove by prefix restores the scene and drops every track.
        let applied = source(&rig);
        rig.doc_tool(
            "effect_apply",
            &format!(r#""preset":"{preset}","idPrefix":"{prefix}","remove":true"#),
        )
        .ok();
        assert!(!timeline(&mut rig).contains("fx-"), "{preset}: tracks left");
        assert_eq!(
            diff(&base[1], &frame(&rig, 6)),
            0,
            "{preset}: render after remove"
        );
        if *targeted {
            assert_eq!(source(&rig), before, "{preset}: source after remove");
        }
        // One undo brings the effect back, a second removes the removal's
        // inverse: undo twice from here is the original document.
        rig.doc_tool("undo", "").ok();
        assert_eq!(source(&rig), applied, "{preset}: undo of remove");
        rig.doc_tool("undo", "").ok();
        assert_eq!(source(&rig), before, "{preset}: undo of apply is exact");
        assert!(
            !timeline(&mut rig).contains("fx-"),
            "{preset}: tracks after undo"
        );
    }
}

#[test]
fn scene_wide_filter_effect_wraps_and_unwraps_the_content() {
    let (mut rig, _) = scene();
    let base = frame(&rig, 6);
    rig.doc_tool("effect_apply", r#""preset":"glow","radius":12"#)
        .ok();
    assert!(source(&rig).contains("fx-glow-1-scene"));
    assert!(diff(&base, &frame(&rig, 6)) > 200);
    rig.doc_tool(
        "effect_apply",
        r#""preset":"glow","idPrefix":"fx-glow-1","remove":true"#,
    )
    .ok();
    assert!(!source(&rig).contains("fx-glow-1"));
    assert_eq!(diff(&base, &frame(&rig, 6)), 0);
}

#[test]
fn prefixes_are_unique_per_call_and_effects_stack() {
    let (mut rig, hero) = scene();
    for n in 1..=2 {
        let reply = rig
            .doc_tool(
                "effect_apply",
                &format!(r#""preset":"vignette","target":"{hero}""#),
            )
            .ok()
            .to_owned();
        assert!(reply.contains(&format!("fx-vignette-{n}")), "{reply}");
    }
    let reuse = rig.doc_tool(
        "effect_apply",
        r#""preset":"flash","idPrefix":"fx-vignette-1""#,
    );
    assert!(reuse.error().contains("idPrefix"), "{}", reuse.body);
}

#[test]
fn hostile_parameters_are_rejected_without_changing_the_document() {
    let (mut rig, hero) = scene();
    rig.doc_tool("effect_apply", r#""preset":"vignette""#).ok();
    let state = rig.state();
    let timeline_before = timeline(&mut rig);
    let bad: &[String] = &[
        r#""preset":"speed_lines","count":1000"#.into(),
        r#""preset":"speed_lines","count":0"#.into(),
        r#""preset":"glow","radius":99"#.into(),
        r#""preset":"glow","radius":-1"#.into(),
        r#""preset":"film_grain","seed":1.5"#.into(),
        r#""preset":"film_grain","seed":-1"#.into(),
        r#""preset":"film_grain","frames":99"#.into(),
        r#""preset":"glow","count":3"#.into(),
        r#""preset":"glow","bogus":3"#.into(),
        r#""preset":"glow","radius":"8""#.into(),
        r#""preset":"nope""#.into(),
        r#""preset":"glow","blend":"multiply""#.into(),
        r#""preset":"flash","color":"red""#.into(),
        r##""preset":"flash","color":"#ff00""##.into(),
        r#""preset":"speed_lines","mode":"spiral""#.into(),
        r#""preset":"glow","idPrefix":"a\"b""#.into(),
        r#""preset":"glow","idPrefix":"bad prefix""#.into(),
        r#""preset":"glow","target":"no_such_element""#.into(),
        r#""preset":"smear""#.into(),
        r#""preset":"flash","start":100000"#.into(),
        r#""preset":"flash","start":40,"attack":10,"falloff":10"#.into(),
        r#""preset":"glow","idPrefix":"fx-vignette-1""#.into(),
        r#""preset":"glow","remove":true"#.into(),
        r#""preset":"glow","idPrefix":"fx-never","remove":true"#.into(),
        format!(r#""preset":"halftone","target":"{hero}","angle":9999"#),
        format!(r#""preset":"film_grain","target":"{hero}","remove":"yes""#),
    ];
    for call in bad {
        let reply = rig.doc_tool("effect_apply", call);
        assert!(reply.is_error, "accepted {call}: {}", reply.body);
        assert_eq!(rig.state(), state, "state changed by {call}");
        assert_eq!(
            timeline(&mut rig),
            timeline_before,
            "timeline changed by {call}"
        );
    }
}

#[test]
fn at_most_256_elements_per_preset() {
    let (mut rig, _) = scene();
    // 64 lines x 3 variants is the largest speed_lines fan and stays inside the cap.
    let reply = rig.doc_tool("effect_apply", r#""preset":"speed_lines","count":64"#);
    assert!(!reply.is_error, "{}", reply.body);
}

#[test]
fn tool_is_listed() {
    let (mut rig, _) = scene();
    assert!(rig.list_tools().iter().any(|t| t == "effect_apply"));
}

#[test]
fn showcase_file_renders() {
    let path = format!(
        "{}/../../../docs/media/effects_v2_showcase.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).unwrap();
    let scene = vexel_app::animation::frame::decode(&bytes).unwrap();
    let tile = scene
        .render_sized(0.0, (160, 90), &std::sync::atomic::AtomicBool::new(false))
        .unwrap();
    assert!(
        tile.rgba()
            .chunks(4)
            .any(|p| p[0] > 200 && p[1] > 200 && p[2] < 160)
    );
}

fn rename(rig: &mut Rig, from: &str, to: &str) {
    rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{from}","name":"id","value":"{to}""#),
    )
    .ok();
}

#[test]
fn remove_leaves_user_elements_named_like_the_prefix() {
    let (mut rig, hero) = scene();
    rename(&mut rig, &hero, "fx-mine");
    let before = source(&rig);
    let state = rig.state();
    let reply = rig.doc_tool(
        "effect_apply",
        r#""preset":"vignette","idPrefix":"fx-mine","remove":true"#,
    );
    assert!(reply.is_error, "{}", reply.body);
    assert_eq!(rig.state(), state);
    assert_eq!(source(&rig), before);
    // A user group `<prefix>-scene` is not ungrouped either.
    rename(&mut rig, "fx-mine", "fx-mine-scene");
    let reply = rig.doc_tool(
        "effect_apply",
        r#""preset":"glow","idPrefix":"fx-mine","remove":true"#,
    );
    assert!(reply.is_error, "{}", reply.body);
    assert!(source(&rig).contains(r#"id="fx-mine-scene""#));
}

#[test]
fn remove_matches_the_exact_generated_tracks_only() {
    let (mut rig, hero) = scene();
    let target = format!(r#","target":"{hero}""#);
    for prefix in ["fx-x", "fx-x-1"] {
        rig.doc_tool(
            "effect_apply",
            &format!(r#""preset":"vignette"{target},"idPrefix":"{prefix}""#),
        )
        .ok();
    }
    // A user track that merely shares the prefix survives too.
    rig.doc_tool(
        "native_timeline_add_property_track",
        &format!(
            r#""trackId":"fx-x-mine","targetId":"{hero}","property":"opacity","valueType":"scalar","interpolation":"linear","value":1"#
        ),
    )
    .ok();
    rig.doc_tool(
        "effect_apply",
        r#""preset":"vignette","idPrefix":"fx-x","remove":true"#,
    )
    .ok();
    let tl = timeline(&mut rig);
    assert!(!tl.contains("fx-x-edge"), "{tl}");
    assert!(tl.contains("fx-x-1-edge"), "{tl}");
    assert!(tl.contains("fx-x-mine"), "{tl}");
    let src = source(&rig);
    assert!(!src.contains(r#"id="fx-x""#) && src.contains(r#"id="fx-x-1""#));
}

#[test]
fn reserved_prefixes_are_refused_for_add_and_remove() {
    let (mut rig, _) = scene();
    rig.doc_tool("effect_apply", r#""preset":"vignette""#).ok();
    let state = rig.state();
    for call in [
        r#""preset":"vignette","idPrefix":"vexel-rig","remove":true"#,
        r#""preset":"vignette","idPrefix":"vexel-mouths","remove":true"#,
        r#""preset":"vignette","idPrefix":"vexel-x","remove":true"#,
        r#""preset":"vignette","idPrefix":"vexel-rig""#,
    ] {
        let reply = rig.doc_tool("effect_apply", call);
        assert!(reply.is_error, "accepted {call}: {}", reply.body);
        assert_eq!(rig.state(), state, "{call}");
    }
}
