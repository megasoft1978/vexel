//! Differential playback/export of a scene with path morphs, animated filter
//! parameters and gradient stops equals the uncached reference render
//! (`render_still`) byte for byte on every tick.

use vexel_app::animation::native_export::NativeExportSource;
use vexel_app::animation::native_export_frames::NativeFramesRequest;
use vexel_app::project_session::ProjectTimelineError;
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

const SETTINGS: EmptyTimelineSettings = EmptyTimelineSettings {
    ticks_per_second: 24,
    duration_ticks: 48,
    fps_numerator: 24,
    fps_denominator: 1,
    loop_mode: LoopMode::Repeat,
    onion_before: 0,
    onion_after: 0,
    onion_opacity: 0.25,
};

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><defs><filter id="f" x="-30%" y="-30%" width="160%" height="160%"><feOffset id="off" dx="0" dy="0" result="o"/><feGaussianBlur id="blur" stdDeviation="1"/></filter><linearGradient id="g"><stop id="s0" offset="0" stop-color="#f00"/><stop id="s1" offset="1" stop-color="#00f"/></linearGradient></defs><rect id="bg" width="128" height="128" fill="#eee"/><path id="p" d="M10 100 L60 20 L110 100 Z" fill="#c33" stroke="#000" stroke-width="2"/><polygon id="poly" points="5,5 40,5 40,40" fill="#393"/><rect id="r" x="64" y="64" width="40" height="40" fill="url(#g)" filter="url(#f)"/></svg>"##;

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

fn doc(tracks: &[(&str, &str, &str, &str, &str, &[(u32, &str)])]) -> AppDocument {
    let path =
        std::env::temp_dir().join(format!("vexel-morph-identity-{}.svg", std::process::id()));
    std::fs::write(&path, SVG).unwrap();
    let mut doc = AppDocument::new(128, 128).unwrap();
    doc.open_path(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    for (id, target, prop, kind, base, keys) in tracks {
        doc.edit_timeline(SETTINGS, EditLimits::default(), |c| {
            let mut c = c.add_property_track(
                id.as_bytes(),
                target.as_bytes(),
                prop.as_bytes(),
                kind.as_bytes(),
                b"linear",
                base.as_bytes(),
                id.as_bytes(),
            )?;
            for (tick, value) in *keys {
                c = c.set_property_key(id.as_bytes(), *tick, value.as_bytes())?;
            }
            Ok::<_, ProjectTimelineError>(c)
        })
        .unwrap_or_else(|e| panic!("track {id}: {e:?}"));
    }
    doc
}

fn assert_identical(label: &str, doc: &AppDocument, size: u32) {
    let source = NativeExportSource::from_document(doc).unwrap();
    let req = NativeFramesRequest {
        size: Some((size, size)),
        background: None,
        fps: None,
    };
    let plan = source.plan_frames(&req).unwrap();
    let ticks: Vec<u32> = source
        .loop_tick_list()
        .unwrap()
        .into_iter()
        .step_by(plan.stride as usize)
        .collect();
    let want: Vec<u64> = ticks
        .iter()
        .map(|tick| {
            let seconds = f64::from(*tick) / f64::from(plan.ticks_per_second);
            let (_, _, rgba) = source.render_still(seconds, |_| Ok((size, size))).unwrap();
            fnv(&rgba)
        })
        .collect();
    let mut got = vec![];
    source
        .render_ticks_differential(&ticks, (size, size), |_, rgba| got.push(fnv(rgba)))
        .unwrap();
    assert_eq!(want.len(), got.len(), "{label}");
    let bad: Vec<_> = (0..want.len()).filter(|&i| want[i] != got[i]).collect();
    assert!(bad.is_empty(), "{label} {size}: ticks differ {bad:?}");
    assert!(
        want.windows(2).any(|w| w[0] != w[1]),
        "{label}: the animation changes frames"
    );
}

#[test]
fn path_and_points_morph_identical() {
    let doc = doc(&[
        (
            "m",
            "p",
            "d",
            "path",
            r#""M10 100 L60 20 L110 100 Z""#,
            &[
                (12, r#""M30 90 L60 60 L100 110 Z""#),
                (30, r#""m 10 100 l 50 -80 l 50 80 z""#),
            ],
        ),
        (
            "pt",
            "poly",
            "points",
            "path",
            r#""5,5 40,5 40,40""#,
            &[(20, r#""30,30 100,30 100,100""#)],
        ),
    ]);
    for size in [128, 100] {
        assert_identical("morph", &doc, size);
    }
}

#[test]
fn filter_and_stop_animation_identical() {
    let doc = doc(&[
        (
            "b",
            "blur",
            "stdDeviation",
            "scalar",
            "1",
            &[(10, "8"), (30, "2")],
        ),
        ("o", "off", "dx", "scalar", "0", &[(20, "6")]),
        ("so", "s0", "offset", "scalar", "0", &[(24, "1.4")]),
        (
            "sc",
            "s1",
            "stop-color",
            "rgba",
            "[0,0,1,1]",
            &[(24, "[1,1,0,0.5]")],
        ),
    ]);
    assert_identical("filter+stops", &doc, 128);
}
