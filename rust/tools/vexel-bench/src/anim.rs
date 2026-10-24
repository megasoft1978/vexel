//! Animated scene variants: the same SVG plus a native timeline (opacity, TRS
//! and colour tracks, 96 ticks at 24 tps) built through the app's own
//! `AppDocument::edit_timeline` / `ProjectTimelineCandidate` APIs.

use crate::scene::Lcg;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use vexel_app::project_session::{ProjectTimelineCandidate, ProjectTimelineError};
use vexel_app::session::AppDocument;
use vexel_timeline::{EditLimits, EmptyTimelineSettings, LoopMode};

pub const TICKS: u32 = 96;
pub const TPS: u32 = 24;
/// Shapes that receive tracks; the rest of the scene is static, as in real use.
pub const MAX_ANIMATED: usize = 48;

pub fn settings() -> EmptyTimelineSettings {
    EmptyTimelineSettings {
        ticks_per_second: TPS,
        duration_ticks: TICKS,
        fps_numerator: TPS,
        fps_denominator: 1,
        loop_mode: LoopMode::Once,
        onion_before: 0,
        onion_after: 0,
        onion_opacity: 0.25,
    }
}

type Step = Result<ProjectTimelineCandidate, ProjectTimelineError>;

fn track(
    c: ProjectTimelineCandidate,
    id: &str,
    target: &str,
    prop: &str,
    kind: &str,
    values: &[String],
) -> Step {
    let last = (TICKS - 1) as usize;
    let mut c = c.add_property_track(
        id.as_bytes(),
        target.as_bytes(),
        prop.as_bytes(),
        kind.as_bytes(),
        b"linear",
        values[0].as_bytes(),
        id.as_bytes(),
    )?;
    for (k, value) in values.iter().enumerate().skip(1) {
        let tick = (k * last / (values.len() - 1)) as u32;
        c = c.set_property_key(id.as_bytes(), tick, value.as_bytes())?;
    }
    Ok(c)
}

fn trs(rng: &mut Lcg, pivot: (f64, f64), first: bool) -> String {
    let (tx, ty, rot, sx, sy) = if first {
        (0.0, 0.0, 0.0, 1.0, 1.0)
    } else {
        (
            rng.range(-40.0, 40.0),
            rng.range(-40.0, 40.0),
            rng.range(-120.0, 120.0),
            rng.range(0.7, 1.3),
            rng.range(0.7, 1.3),
        )
    };
    format!(
        "{{\"tx\":{tx:.2},\"ty\":{ty:.2},\"rotation_deg\":{rot:.2},\"sx\":{sx:.3},\"sy\":{sy:.3},\
         \"pivot_x\":{:.1},\"pivot_y\":{:.1}}}",
        pivot.0, pivot.1
    )
}

fn build(candidate: ProjectTimelineCandidate, n: usize) -> Step {
    let mut rng = Lcg::new(0xA11CE + n as u64);
    let animated = n.clamp(1, MAX_ANIMATED);
    let mut c = candidate;
    for j in 0..animated {
        let target = format!("s{}", j * n / animated);
        let pivot = (rng.range(100.0, 400.0), rng.range(100.0, 400.0));
        let opacity: Vec<String> = (0..5)
            .map(|k| {
                if k == 0 {
                    "1".into()
                } else {
                    format!("{:.2}", rng.range(0.2, 1.0))
                }
            })
            .collect();
        c = track(c, &format!("o{j}"), &target, "opacity", "scalar", &opacity)?;
        let motion: Vec<String> = (0..5).map(|k| trs(&mut rng, pivot, k == 0)).collect();
        c = track(c, &format!("t{j}"), &target, "transform", "trs", &motion)?;
        let colour: Vec<String> = (0..5)
            .map(|_| format!("[{:.3},{:.3},{:.3},1]", rng.unit(), rng.unit(), rng.unit()))
            .collect();
        c = track(c, &format!("c{j}"), &target, "fill", "rgba", &colour)?;
    }
    Ok(c)
}

/// Opens `svg`, adds the tracks and writes the project to `out`.
pub fn build_animated(svg: &Path, out: &Path, n: usize) -> Result<(), String> {
    let mut doc = AppDocument::new(512, 512).map_err(|e| format!("{e:?}"))?;
    doc.open_path(svg)
        .map_err(|e| format!("open {}: {e:?}", svg.display()))?;
    let limits = EditLimits::default();
    doc.edit_timeline(settings(), limits, |c| build(c.clone(), n))
        .map_err(|e| format!("timeline edit: {e:?}"))?;
    doc.save_as(out.as_os_str().as_bytes())
        .map_err(|e| format!("save {}: {e:?}", out.display()))
}
