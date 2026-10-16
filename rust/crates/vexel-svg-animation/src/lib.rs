//! Bounded, source-preserving SMIL animation parsing and evaluation.
//!
//! Source is borrowed for the lifetime of the document. All parsed track data is
//! owned; raw values and XML locations remain checked byte spans into that
//! source. No parser or evaluator operation rewrites the document.

#![forbid(unsafe_code)]

mod control;
#[cfg(test)]
mod control_scan_tests;
mod eval;
mod motion;
mod parse;
mod parse_values;

use std::sync::Arc;
use vexel_xml::XmlSpan;

pub const MAX_TRACKS: usize = 128;
pub const MAX_VALUES: usize = 256;
pub const MAX_DIAGNOSTICS: usize = 128;
pub const MAX_TIMELINE_SECONDS: f64 = 3600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Attribute {
    X,
    Y,
    Width,
    Height,
    Cx,
    Cy,
    R,
    Rx,
    Ry,
    Opacity,
    FillOpacity,
    StrokeOpacity,
    Fill,
    Stroke,
    StrokeWidth,
    StrokeDashoffset,
    D,
    Transform,
    Motion,
}
impl Attribute {
    pub const fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Width => "width",
            Self::Height => "height",
            Self::Cx => "cx",
            Self::Cy => "cy",
            Self::R => "r",
            Self::Rx => "rx",
            Self::Ry => "ry",
            Self::Opacity => "opacity",
            Self::FillOpacity => "fill-opacity",
            Self::StrokeOpacity => "stroke-opacity",
            Self::Fill => "fill",
            Self::Stroke => "stroke",
            Self::StrokeWidth => "stroke-width",
            Self::StrokeDashoffset => "stroke-dashoffset",
            Self::D => "d",
            Self::Transform => "transform",
            Self::Motion => "motion",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Animate,
    Transform,
    Set,
    Motion,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Number,
    Color,
    Transform,
    Motion,
    Path,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcMode {
    Linear,
    Spline,
    Discrete,
    Paced,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    Remove,
    Freeze,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    Translate,
    Scale,
    Rotate,
    SkewX,
    SkewY,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnsupportedElement,
    UnsupportedAttribute,
    UnsupportedTiming,
    UnsupportedComposition,
    UnsupportedPath,
    CssAnimation,
    InvalidTrack,
}
impl DiagnosticKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::UnsupportedElement => "unsupported-element",
            Self::UnsupportedAttribute => "unsupported-attribute",
            Self::UnsupportedTiming => "unsupported-timing",
            Self::UnsupportedComposition => "unsupported-composition",
            Self::UnsupportedPath => "unsupported-path",
            Self::CssAnimation => "css-animation",
            Self::InvalidTrack => "invalid-track",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Value {
    pub kind: ValueKind,
    pub number: [f64; 4],
    pub rgba: [u8; 4],
    pub raw: XmlSpan,
}
impl Default for Value {
    fn default() -> Self {
        Self {
            kind: ValueKind::Number,
            number: [0.0; 4],
            rgba: [0; 4],
            raw: XmlSpan::default(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
    pub time: f64,
    pub value: Value,
    pub source: XmlSpan,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionPoint {
    pub x: f64,
    pub y: f64,
    pub distance: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub kind: Kind,
    pub attribute: Attribute,
    pub transform_type: TransformType,
    pub calc_mode: CalcMode,
    pub fill_mode: FillMode,
    pub base_value: Value,
    pub keys: Vec<Keyframe>,
    pub motion_points: Vec<MotionPoint>,
    pub motion_length: f64,
    pub motion_fixed_rotation: f64,
    pub splines: Vec<[f64; 4]>,
    pub begin_seconds: f64,
    pub begin_offset: f64,
    pub duration_seconds: f64,
    pub end_seconds: f64,
    pub repeat_count: f64,
    pub repeat_duration: f64,
    pub begin_sync_track: Option<usize>,
    pub end_sync_track: Option<usize>,
    pub begin_sync_id: Option<XmlSpan>,
    pub end_sync_id: Option<XmlSpan>,
    pub begin_indefinite: bool,
    pub begin_sync_end: bool,
    pub has_begin_sync: bool,
    pub has_end: bool,
    pub end_is_sync: bool,
    pub repeat_indefinite: bool,
    pub duration_indefinite: bool,
    pub repeat_duration_indefinite: bool,
    pub additive_sum: bool,
    pub accumulate: bool,
    pub motion_rotate_auto: bool,
    pub motion_rotate_reverse: bool,
    pub supported: bool,
    pub target: XmlSpan,
    pub element: XmlSpan,
    pub id: XmlSpan,
}
impl Default for Track {
    fn default() -> Self {
        Self {
            kind: Kind::Animate,
            attribute: Attribute::X,
            transform_type: TransformType::Translate,
            calc_mode: CalcMode::Linear,
            fill_mode: FillMode::Remove,
            base_value: Value::default(),
            keys: Vec::new(),
            motion_points: Vec::new(),
            motion_length: 0.0,
            motion_fixed_rotation: 0.0,
            splines: Vec::new(),
            begin_seconds: 0.0,
            begin_offset: 0.0,
            duration_seconds: 0.0,
            end_seconds: 0.0,
            repeat_count: 1.0,
            repeat_duration: f64::INFINITY,
            begin_sync_track: None,
            end_sync_track: None,
            begin_sync_id: None,
            end_sync_id: None,
            begin_indefinite: false,
            begin_sync_end: false,
            has_begin_sync: false,
            has_end: false,
            end_is_sync: false,
            repeat_indefinite: false,
            duration_indefinite: false,
            repeat_duration_indefinite: false,
            additive_sum: false,
            accumulate: false,
            motion_rotate_auto: false,
            motion_rotate_reverse: false,
            supported: true,
            target: XmlSpan::default(),
            element: XmlSpan::default(),
            id: XmlSpan::default(),
        }
    }
}
#[derive(Debug)]
pub struct Document<'a> {
    pub source: &'a [u8],
    pub tracks: Vec<Track>,
    pub diagnostics: Vec<Diagnostic>,
}
/// Source and parsed animation retained together for worker-safe use.
#[derive(Debug)]
pub struct OwnedDocument {
    source_owner: Arc<Vec<u8>>,
    tracks: Vec<Track>,
    diagnostics: Vec<Diagnostic>,
}

/// Allocation-free immutable access to an animation document.
#[derive(Debug, Clone, Copy)]
pub struct DocumentView<'a> {
    pub source: &'a [u8],
    pub tracks: &'a [Track],
    pub diagnostics: &'a [Diagnostic],
}

impl Document<'_> {
    pub fn view(&self) -> DocumentView<'_> {
        DocumentView {
            source: self.source,
            tracks: &self.tracks,
            diagnostics: &self.diagnostics,
        }
    }
}

impl OwnedDocument {
    pub fn source_owner(&self) -> &Arc<Vec<u8>> {
        &self.source_owner
    }
    pub fn source(&self) -> &[u8] {
        self.source_owner.as_slice()
    }
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub fn track_capacity(&self) -> usize {
        self.tracks.capacity()
    }
    pub fn diagnostic_capacity(&self) -> usize {
        self.diagnostics.capacity()
    }
    pub fn view(&self) -> DocumentView<'_> {
        DocumentView {
            source: self.source(),
            tracks: &self.tracks,
            diagnostics: &self.diagnostics,
        }
    }
    /// Total retained capacity, including the source allocation exactly once.
    pub fn storage_bytes(&self) -> Option<usize> {
        let mut bytes = self.source_owner.capacity();
        bytes = bytes.checked_add(
            self.tracks
                .capacity()
                .checked_mul(std::mem::size_of::<Track>())?,
        )?;
        bytes = bytes.checked_add(
            self.diagnostics
                .capacity()
                .checked_mul(std::mem::size_of::<Diagnostic>())?,
        )?;
        for track in &self.tracks {
            bytes = bytes.checked_add(
                track
                    .keys
                    .capacity()
                    .checked_mul(std::mem::size_of::<Keyframe>())?,
            )?;
            bytes = bytes.checked_add(
                track
                    .motion_points
                    .capacity()
                    .checked_mul(std::mem::size_of::<MotionPoint>())?,
            )?;
            bytes = bytes.checked_add(
                track
                    .splines
                    .capacity()
                    .checked_mul(std::mem::size_of::<[f64; 4]>())?,
            )?;
        }
        Some(bytes)
    }
}
/// Parse failures distinguish source-track exhaustion from allocation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    TrackLimit { offset: usize },
    Allocation,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayItem {
    pub track_index: usize,
    pub value: Value,
    pub iteration: usize,
    pub active: bool,
}
#[derive(Debug, PartialEq)]
pub struct Overlay {
    pub items: Vec<OverlayItem>,
    pub time_seconds: f64,
}

pub use eval::evaluate;
pub use eval::evaluate_view;
pub use eval::{EvaluationError, evaluate_controlled, evaluate_view_controlled};
pub use parse::parse;
pub use parse::parse_owned;
pub use parse::{parse_controlled, parse_owned_controlled};

pub fn attribute_name(attribute: Attribute) -> &'static str {
    attribute.name()
}
pub fn diagnostic_name(kind: DiagnosticKind) -> &'static str {
    kind.name()
}
