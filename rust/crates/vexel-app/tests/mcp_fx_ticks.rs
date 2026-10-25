//! `effect_apply` tick bounds: with odd timeline durations and starts, no
//! generated key may land at or past the duration, so the call must succeed.

mod common;

use common::{Rig, element_id, no_grants};

/// White page with a red "hero" group, 512x512 (same scene as `mcp_fx`).
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
    (rig, hero)
}

fn set_duration(rig: &mut Rig, ticks: u32) {
    rig.doc_tool(
        "native_timeline_set_duration",
        &format!(r#""durationTicks":{ticks}"#),
    )
    .ok();
}

#[test]
fn vhs_roll_keys_stay_inside_odd_durations() {
    // The default 24-tick roll cycle puts its second key 23 ticks after each
    // start; the last cycle must be clipped to the window instead of overshooting.
    for (duration, start) in [(37, 5), (37, 0), (41, 7), (38, 13), (45, 20)] {
        let (mut rig, _) = scene();
        set_duration(&mut rig, duration);
        let reply = rig.doc_tool(
            "effect_apply",
            &format!(r#""preset":"vhs_scanlines","start":{start}"#),
        );
        assert!(
            !reply.is_error,
            "duration {duration}, start {start}: {}",
            reply.body
        );
        assert!(
            reply.body.contains("\"ok\":true"),
            "duration {duration}, start {start}: {}",
            reply.body
        );
    }
}
