//! Per-tick render cost of each `effect_apply` preset on a 900 px scene.
//! `cargo test -p vexel-app --release --test mcp_fx_perf -- --ignored --nocapture`

mod common;

use common::{Rig, element_id, no_grants};
use std::time::Instant;
use vexel_app::animation::native_export::NativeExportSource;
use vexel_app::session::AppDocument;

fn scene() -> (Rig, String) {
    let mut rig = Rig::with_document(AppDocument::new(900, 900).unwrap(), no_grants());
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
        r##""x":"0","y":"0","width":"900","height":"900","fill":"#808080""##,
    );
    let hero = add(
        &mut rig,
        r##""x":"300","y":"300","width":"300","height":"300","fill":"#cc2222""##,
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
    (rig, hero)
}

/// Median ms over ticks 2..=9, three renders each.
fn median_ms(rig: &Rig) -> f64 {
    let native = NativeExportSource::from_document(&rig.app).unwrap();
    let mut times = Vec::new();
    for tick in 2..=9u32 {
        for _ in 0..3 {
            let t = Instant::now();
            native.render_still_at(f64::from(tick) / 24.0, Ok).unwrap();
            times.push(t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

#[test]
#[ignore]
fn preset_render_cost_on_900px() {
    let (rig, _) = scene();
    let base = median_ms(&rig);
    println!("baseline {base:.2} ms");
    let cases: &[(&str, &str)] = &[
        ("speed_lines", r#""count":32"#),
        ("glow", r#""radius":8"#),
        ("chromatic_split", r#""amount":8"#),
        ("film_grain", r#""frames":3"#),
        ("film_grain", r#""frames":2,"octaves":1"#),
        ("glow", r#""radius":4"#),
        ("halftone", r#""size":8"#),
        ("vhs_scanlines", r#""pitch":4"#),
        ("vhs_scanlines", r#""pitch":4,"noise":0"#),
        ("vhs_scanlines", r#""pitch":4,"noise":0.2,"jitter":0"#),
        ("motion_blur_dir", r#""amount":30,"copies":6"#),
        ("smear", r#""start":1,"ticks":12,"trail":3"#),
        ("vignette", r#""intensity":0.8"#),
        ("flash", r#""start":2,"falloff":10"#),
    ];
    for (preset, extra) in cases {
        let (mut rig, hero) = scene();
        let target = if matches!(
            *preset,
            "glow" | "chromatic_split" | "motion_blur_dir" | "smear"
        ) {
            format!(r#","target":"{hero}""#)
        } else {
            String::new()
        };
        rig.doc_tool(
            "effect_apply",
            &format!(r#""preset":"{preset}"{target},{extra}"#),
        )
        .ok();
        let ms = median_ms(&rig);
        println!("{preset:<16} {extra:<28} {ms:7.2} ms  (+{:.2})", ms - base);
    }
}
