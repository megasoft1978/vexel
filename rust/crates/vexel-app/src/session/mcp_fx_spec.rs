//! `effect_apply` arguments: the presets, their bounded numeric parameters and
//! the parsed call.

/// Numeric parameters: (name, min, max, integer only).
pub(crate) const PARAMS: &[(&str, f64, f64, bool)] = &[
    ("start", 0.0, 1_000_000.0, true),
    ("ticks", 1.0, 1_000_000.0, true),
    ("seed", 0.0, 4_294_967_295.0, true),
    ("count", 1.0, 64.0, true),
    ("length", 1.0, 4096.0, false),
    ("radius", 0.0, 32.0, false),
    ("amount", 0.0, 64.0, false),
    ("angle", -360.0, 360.0, false),
    ("intensity", 0.0, 1.0, false),
    ("pulse", 0.0, 1.0, false),
    ("period", 1.0, 1000.0, true),
    ("size", 2.0, 64.0, false),
    ("dot", 0.0, 1.0, false),
    ("dotTo", 0.0, 1.0, false),
    ("pitch", 2.0, 64.0, false),
    ("jitter", 0.0, 32.0, false),
    ("noise", 0.0, 1.0, false),
    ("copies", 1.0, 12.0, true),
    ("stretch", 1.0, 4.0, false),
    ("trail", 0.0, 8.0, true),
    ("attack", 1.0, 48.0, true),
    ("falloff", 1.0, 480.0, true),
    ("frames", 2.0, 6.0, true),
    ("frequency", 0.05, 4.0, false),
    ("octaves", 1.0, 3.0, true),
    ("softness", 0.0, 0.9, false),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    SpeedLines,
    Glow,
    ChromaticSplit,
    FilmGrain,
    Halftone,
    VhsScanlines,
    MotionBlurDir,
    Smear,
    Vignette,
    Flash,
}

impl Preset {
    pub(crate) const ALL: [Preset; 10] = [
        Preset::SpeedLines,
        Preset::Glow,
        Preset::ChromaticSplit,
        Preset::FilmGrain,
        Preset::Halftone,
        Preset::VhsScanlines,
        Preset::MotionBlurDir,
        Preset::Smear,
        Preset::Vignette,
        Preset::Flash,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Preset::SpeedLines => "speed_lines",
            Preset::Glow => "glow",
            Preset::ChromaticSplit => "chromatic_split",
            Preset::FilmGrain => "film_grain",
            Preset::Halftone => "halftone",
            Preset::VhsScanlines => "vhs_scanlines",
            Preset::MotionBlurDir => "motion_blur_dir",
            Preset::Smear => "smear",
            Preset::Vignette => "vignette",
            Preset::Flash => "flash",
        }
    }

    pub fn parse(name: &str) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Parameters (besides target, idPrefix, remove) this preset accepts.
    pub fn allowed(self) -> &'static [&'static str] {
        match self {
            Preset::SpeedLines => &[
                "start",
                "ticks",
                "seed",
                "count",
                "length",
                "intensity",
                "angle",
                "period",
                "mode",
                "color",
            ],
            Preset::Glow => &[
                "start",
                "ticks",
                "radius",
                "intensity",
                "pulse",
                "period",
                "blend",
            ],
            Preset::ChromaticSplit => &["start", "ticks", "amount", "angle", "pulse", "period"],
            Preset::FilmGrain => &[
                "start",
                "ticks",
                "seed",
                "intensity",
                "frames",
                "frequency",
                "octaves",
                "period",
            ],
            Preset::Halftone => &[
                "start",
                "ticks",
                "size",
                "dot",
                "dotTo",
                "angle",
                "intensity",
                "color",
            ],
            Preset::VhsScanlines => &[
                "start",
                "ticks",
                "seed",
                "pitch",
                "intensity",
                "jitter",
                "noise",
                "period",
            ],
            Preset::MotionBlurDir => &[
                "start",
                "ticks",
                "amount",
                "angle",
                "copies",
                "pulse",
                "period",
                "intensity",
            ],
            Preset::Smear => &[
                "start",
                "ticks",
                "angle",
                "stretch",
                "trail",
                "attack",
                "amount",
                "intensity",
            ],
            Preset::Vignette => &[
                "start",
                "ticks",
                "intensity",
                "pulse",
                "period",
                "softness",
                "color",
            ],
            Preset::Flash => &[
                "start",
                "ticks",
                "intensity",
                "attack",
                "falloff",
                "color",
                "blend",
            ],
        }
    }
}

/// One `effect_apply` call.
#[derive(Clone, Debug, PartialEq)]
pub struct FxSpec {
    pub preset: Preset,
    pub target: Option<String>,
    pub prefix: Option<String>,
    pub remove: bool,
    pub nums: Vec<(&'static str, f64)>,
    pub mode: Option<String>,
    pub color: Option<String>,
    pub blend: Option<String>,
}
