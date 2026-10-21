//! Hit geometry of the extended inspector controls: opacity, fill kind and
//! gradient stops, dash presets and the transform steppers. Each control is a
//! `Target::SectionHeader(EXT_FIRST + n)`, the same encoding the swatches and
//! width presets already use, so the hit test, tooltips and motion registry
//! need no new variant.

use super::{Layout, SECTION_BODY_H};
use vexel_ui::Rect;

/// First `Target::SectionHeader` index used by an extended control.
pub const EXT_FIRST: usize = 21;
/// Number of extended controls.
pub const EXT_COUNT: usize = 32;
/// One past the last `Target::SectionHeader` index that is an inspector control.
pub const EXT_END: usize = EXT_FIRST + EXT_COUNT;

/// Swatches per gradient stop row (the existing swatch palette).
pub const STOP_SWATCHES: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtControl {
    OpacityDown,
    OpacityUp,
    /// 0 Solid, 1 Linear, 2 Radial.
    FillKind(usize),
    /// Swatch `swatch` of gradient stop `stop` (0 first, 1 last).
    GradStop {
        stop: usize,
        swatch: usize,
    },
    AngleDown,
    AngleUp,
    /// 0 none, 1 dashed, 2 dotted.
    Dash(usize),
    RotateDown,
    RotateUp,
    ScaleXDown,
    ScaleXUp,
    ScaleYDown,
    ScaleYUp,
    PivotXDown,
    PivotXUp,
    PivotYDown,
    PivotYUp,
}

impl ExtControl {
    /// Offset from [`EXT_FIRST`].
    pub fn index(self) -> usize {
        EXT_FIRST
            + match self {
                Self::OpacityDown => 0,
                Self::OpacityUp => 1,
                Self::FillKind(k) => 2 + k.min(2),
                Self::GradStop { stop, swatch } => {
                    5 + stop.min(1) * STOP_SWATCHES + swatch.min(STOP_SWATCHES - 1)
                }
                Self::AngleDown => 17,
                Self::AngleUp => 18,
                Self::Dash(d) => 19 + d.min(2),
                Self::RotateDown => 22,
                Self::RotateUp => 23,
                Self::ScaleXDown => 24,
                Self::ScaleXUp => 25,
                Self::ScaleYDown => 26,
                Self::ScaleYUp => 27,
                Self::PivotXDown => 28,
                Self::PivotXUp => 29,
                Self::PivotYDown => 30,
                Self::PivotYUp => 31,
            }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Some(match index.checked_sub(EXT_FIRST)? {
            0 => Self::OpacityDown,
            1 => Self::OpacityUp,
            n @ 2..=4 => Self::FillKind(n - 2),
            n @ 5..=16 => Self::GradStop {
                stop: (n - 5) / STOP_SWATCHES,
                swatch: (n - 5) % STOP_SWATCHES,
            },
            17 => Self::AngleDown,
            18 => Self::AngleUp,
            n @ 19..=21 => Self::Dash(n - 19),
            22 => Self::RotateDown,
            23 => Self::RotateUp,
            24 => Self::ScaleXDown,
            25 => Self::ScaleXUp,
            26 => Self::ScaleYDown,
            27 => Self::ScaleYUp,
            28 => Self::PivotXDown,
            29 => Self::PivotXUp,
            30 => Self::PivotYDown,
            31 => Self::PivotYUp,
            _ => return None,
        })
    }

    /// Tooltip text: (name, hint).
    pub fn tip(self) -> (&'static str, &'static str) {
        match self {
            Self::OpacityDown => ("Less opaque", "-0.1"),
            Self::OpacityUp => ("More opaque", "+0.1"),
            Self::FillKind(0) => ("Solid fill", ""),
            Self::FillKind(1) => ("Linear gradient", ""),
            Self::FillKind(_) => ("Radial gradient", ""),
            Self::GradStop { stop: 0, .. } => ("First stop colour", ""),
            Self::GradStop { .. } => ("Last stop colour", ""),
            Self::AngleDown => ("Gradient angle", "-15\u{b0}"),
            Self::AngleUp => ("Gradient angle", "+15\u{b0}"),
            Self::Dash(0) => ("Solid stroke", ""),
            Self::Dash(1) => ("Dashed stroke", ""),
            Self::Dash(_) => ("Dotted stroke", ""),
            Self::RotateDown => ("Rotate", "-15\u{b0}"),
            Self::RotateUp => ("Rotate", "+15\u{b0}"),
            Self::ScaleXDown => ("Scale X", "-0.1"),
            Self::ScaleXUp => ("Scale X", "+0.1"),
            Self::ScaleYDown => ("Scale Y", "-0.1"),
            Self::ScaleYUp => ("Scale Y", "+0.1"),
            Self::PivotXDown => ("Pivot X", "-1"),
            Self::PivotXUp => ("Pivot X", "+1"),
            Self::PivotYDown => ("Pivot Y", "-1"),
            Self::PivotYUp => ("Pivot Y", "+1"),
        }
    }
}

/// A full-width row of the extended inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtRow {
    Opacity,
    FillKind,
    StopFirst,
    StopLast,
    Angle,
    Dash,
    Rotate,
    PivotX,
    PivotY,
    ScaleX,
    ScaleY,
}

impl ExtRow {
    /// (section, y offset in the body, height).
    fn place(self) -> (usize, u32, u32) {
        match self {
            Self::Opacity => (0, 32, 28),
            Self::FillKind => (1, 70, 24),
            Self::StopFirst => (1, 100, 20),
            Self::StopLast => (1, 128, 20),
            Self::Angle => (1, 156, 28),
            Self::Dash => (2, 136, 22),
            Self::Rotate => (3, 40, 28),
            Self::PivotX | Self::PivotY => (3, 72, 28),
            Self::ScaleX => (3, 104, 28),
            Self::ScaleY => (3, 136, 28),
        }
    }
}

/// The two stepper buttons at the right end of a stepper row.
pub fn stepper_buttons(row: Rect) -> (Rect, Rect) {
    let at = |dx: u32| Rect {
        x: row.x + row.width.saturating_sub(dx),
        y: row.y + 3,
        width: 22,
        height: row.height.saturating_sub(6),
    };
    (at(56), at(28))
}

/// Two equal halves of a row, 8 px apart (pivot X and Y share one row).
pub fn halves(row: Rect) -> [Rect; 2] {
    let w = row.width.saturating_sub(8) / 2;
    [
        Rect { width: w, ..row },
        Rect {
            x: row.x + w + 8,
            width: row.width.saturating_sub(w + 8),
            ..row
        },
    ]
}

/// Equal-width segments of a three-way row, 6 px apart.
pub fn segments(row: Rect) -> [Rect; 3] {
    let w = row.width.saturating_sub(12) / 3;
    std::array::from_fn(|i| Rect {
        x: row.x + i as u32 * (w + 6),
        width: w,
        ..row
    })
}

/// Swatch `index` of a stop row (after a 48 px caption).
pub fn stop_swatch(row: Rect, index: usize) -> Rect {
    Rect {
        x: row.x + 48 + index as u32 * 28,
        y: row.y,
        width: 20,
        height: 20,
    }
}

impl Layout {
    /// Rect of an extended row; `None` while its section is collapsed, absent
    /// or clipped.
    pub fn ext_row(&self, row: ExtRow) -> Option<Rect> {
        let (section, y, height) = row.place();
        let body = *self.section_bodies.get(section)?;
        if body.height != SECTION_BODY_H[section] {
            return None;
        }
        let full = Rect {
            x: body.x + 12,
            y: body.y + y,
            width: body.width.saturating_sub(24),
            height,
        };
        Some(match row {
            ExtRow::PivotX => halves(full)[0],
            ExtRow::PivotY => halves(full)[1],
            _ => full,
        })
    }

    /// Rect of one extended control.
    pub fn ext_control_rect(&self, control: ExtControl) -> Option<Rect> {
        let stepper = |row: ExtRow, up: bool| {
            let (down, plus) = stepper_buttons(self.ext_row(row)?);
            Some(if up { plus } else { down })
        };
        match control {
            ExtControl::OpacityDown => stepper(ExtRow::Opacity, false),
            ExtControl::OpacityUp => stepper(ExtRow::Opacity, true),
            ExtControl::FillKind(k) => Some(segments(self.ext_row(ExtRow::FillKind)?)[k.min(2)]),
            ExtControl::GradStop { stop, swatch } => {
                let row = self.ext_row(if stop == 0 {
                    ExtRow::StopFirst
                } else {
                    ExtRow::StopLast
                })?;
                (swatch < STOP_SWATCHES).then(|| stop_swatch(row, swatch))
            }
            ExtControl::AngleDown => stepper(ExtRow::Angle, false),
            ExtControl::AngleUp => stepper(ExtRow::Angle, true),
            ExtControl::Dash(d) => Some(segments(self.ext_row(ExtRow::Dash)?)[d.min(2)]),
            ExtControl::RotateDown => stepper(ExtRow::Rotate, false),
            ExtControl::RotateUp => stepper(ExtRow::Rotate, true),
            ExtControl::ScaleXDown => stepper(ExtRow::ScaleX, false),
            ExtControl::ScaleXUp => stepper(ExtRow::ScaleX, true),
            ExtControl::ScaleYDown => stepper(ExtRow::ScaleY, false),
            ExtControl::ScaleYUp => stepper(ExtRow::ScaleY, true),
            ExtControl::PivotXDown => stepper(ExtRow::PivotX, false),
            ExtControl::PivotXUp => stepper(ExtRow::PivotX, true),
            ExtControl::PivotYDown => stepper(ExtRow::PivotY, false),
            ExtControl::PivotYUp => stepper(ExtRow::PivotY, true),
        }
    }

    pub(super) fn ext_control_rect_at(&self, index: usize) -> Option<Rect> {
        self.ext_control_rect(ExtControl::from_index(index)?)
    }
}
