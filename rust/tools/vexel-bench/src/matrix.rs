//! Which cases each protocol level runs, and where their scene files live.

use crate::ops::{ALL_OPS, Op};
use crate::scene::{PROFILES, Profile, SCENE_VERSION};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// N up to 20000 (animated up to 5000).
    Full,
    /// N up to 1000; the level the committed baseline is recorded at.
    Reduced,
    /// N up to 500, deterministic ops only, single pass: for hosted CI.
    HashOnly,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Full => "full",
            Level::Reduced => "reduced",
            Level::HashOnly => "hash-only",
        }
    }
    pub fn sizes(self) -> &'static [usize] {
        match self {
            Level::Full => &[100, 500, 1000, 5000, 20000],
            Level::Reduced => &[100, 500, 1000],
            Level::HashOnly => &[100, 500],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseId {
    pub op: Op,
    pub profile: Profile,
    pub n: usize,
}

const HASH_ONLY_OPS: [Op; 8] = [
    Op::OpenDecode,
    Op::Render(512),
    Op::ZoomRecompose,
    Op::SelMove,
    Op::SelRotate,
    Op::SaveLoad,
    Op::Tick(512),
    Op::Scrub,
];

/// Export profiles: encoders are the point, and 96 frames of the slowest
/// scenes (strokes) would cost minutes per trial; per-frame cost is already
/// covered by the `tick_*` cases.
const EXPORT_PROFILES: [&str; 3] = ["fill", "opac", "mix"];

fn included(level: Level, op: Op, profile: Profile, n: usize) -> bool {
    let export = matches!(op, Op::ExportGif | Op::ExportApng | Op::ExportMp4);
    if export && !EXPORT_PROFILES.contains(&profile.name()) {
        return false;
    }
    match level {
        Level::HashOnly => {
            HASH_ONLY_OPS.contains(&op)
                || (matches!(op, Op::ExportGif | Op::ExportApng) && n == 100)
        }
        Level::Reduced => match op {
            Op::Tick(_) | Op::Scrub => n <= 5000,
            Op::ExportGif | Op::ExportApng | Op::ExportMp4 => n == 100,
            _ => true,
        },
        Level::Full => match op {
            Op::Tick(_) | Op::Scrub => n <= 5000,
            Op::ExportGif | Op::ExportApng => n <= 500,
            Op::ExportMp4 => n == 100,
            _ => true,
        },
    }
}

pub fn cases(level: Level, only: Option<&str>) -> Vec<CaseId> {
    let mut out = Vec::new();
    for &n in level.sizes() {
        for profile in PROFILES {
            for op in ALL_OPS {
                let id = CaseId { op, profile, n };
                let name = crate::child::name(op, profile.name(), n);
                if included(level, op, profile, n) && only.is_none_or(|f| name.contains(f)) {
                    out.push(id);
                }
            }
        }
    }
    out
}

pub fn scene_dir(root: &Path) -> PathBuf {
    root.join(".cache/bench/scenes").join(SCENE_VERSION)
}

pub fn svg_path(root: &Path, profile: Profile, n: usize) -> PathBuf {
    scene_dir(root).join(format!("{}-{n}.svg", profile.name()))
}

pub fn vexel_path(root: &Path, profile: Profile, n: usize) -> PathBuf {
    scene_dir(root).join(format!("{}-{n}-anim.vexel", profile.name()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_only_is_a_subset_of_reduced() {
        let reduced: Vec<String> = cases(Level::Reduced, None)
            .iter()
            .map(|c| crate::child::name(c.op, c.profile.name(), c.n))
            .collect();
        for c in cases(Level::HashOnly, None) {
            let name = crate::child::name(c.op, c.profile.name(), c.n);
            assert!(reduced.contains(&name), "{name} missing from reduced");
        }
    }

    #[test]
    fn animated_cases_stop_at_5000() {
        assert!(
            cases(Level::Full, None)
                .iter()
                .all(|c| !c.op.animated() || c.n <= 5000)
        );
        assert!(cases(Level::Full, None).iter().any(|c| c.n == 20000));
    }
}
