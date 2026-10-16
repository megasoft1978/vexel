//! Strict bounded icon path parser, mask rasterizer, and caller-owned mask cache.
use crate::{
    Rect, Theme, Tokens,
    draw::{blend, valid_buffer},
};
use std::f64::consts::PI;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IconKind {
    None = 0,
    SELECT = 1,
    PAN = 2,
    ZOOM_IN = 3,
    ZOOM_OUT = 4,
    RECTANGLE = 5,
    ELLIPSE = 6,
    LINE = 7,
    PEN = 8,
    NODE_EDIT = 9,
    PENCIL = 10,
    TEXT = 11,
    EYEDROPPER = 12,
    GRADIENT = 13,
    FILL = 14,
    STROKE = 15,
    LAYERS = 16,
    EYE = 17,
    EYE_OFF = 18,
    LOCK = 19,
    UNLOCK = 20,
    PLUS = 21,
    MINUS = 22,
    CLOSE = 23,
    CHEVRON_RIGHT = 24,
    CHEVRON_DOWN = 25,
    MORE = 26,
    SEARCH = 27,
    SAVE = 28,
    UNDO = 29,
    REDO = 30,
    DUPLICATE = 31,
    TRASH = 32,
    GROUP = 33,
    UNGROUP = 34,
    FIT = 35,
    ACTUAL_SIZE = 36,
    RULER = 37,
    GRID = 38,
    SNAP = 39,
    ALIGN_LEFT = 40,
    ALIGN_CENTER = 41,
    ALIGN_RIGHT = 42,
    ALIGN_TOP = 43,
    ALIGN_MIDDLE = 44,
    ALIGN_BOTTOM = 45,
    CIRCLE = 46,
    DOCUMENT = 47,
    STOP = 48,
    PATH = 49,
    POLYLINE = 50,
    IMAGE = 51,
    SIDEBAR_LEFT = 52,
    SIDEBAR_RIGHT = 53,
    COUNT = 54,
}
impl IconKind {
    pub const ZOOM: Self = Self::ZOOM_IN;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    Rest,
    Hover,
    Pressed,
    Focus,
    Disabled,
    Selected,
    Invalid,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconBounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconError {
    InvalidPath,
    TooComplex,
    InvalidBuffer,
    InvalidScale,
    InvalidIcon,
    Allocation,
}
#[derive(Debug, Clone, Copy)]
struct Segment {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}
const MAX_PATH: usize = 2048;
const MAX_SEGMENTS: usize = 384;
const MAX_SCALE: usize = 3;
const AXIS: usize = 24;

fn number(path: &str, at: &mut usize) -> Result<f64, IconError> {
    let b = path.as_bytes();
    while *at < b.len() && matches!(b[*at], b' ' | b'\t' | b'\r' | b'\n' | b',') {
        *at += 1
    }
    let start = *at;
    while *at < b.len() && matches!(b[*at], b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E') {
        *at += 1
    }
    if start == *at || *at - start >= 48 {
        return Err(IconError::InvalidPath);
    }
    let n = path[start..*at]
        .parse::<f64>()
        .map_err(|_| IconError::InvalidPath)?;
    if !n.is_finite() {
        return Err(IconError::InvalidPath);
    }
    Ok(n)
}
fn add(out: &mut Vec<Segment>, s: Segment) -> Result<(), IconError> {
    if out.len() >= MAX_SEGMENTS {
        return Err(IconError::TooComplex);
    }
    if ![s.x0, s.y0, s.x1, s.y1].iter().all(|v| v.is_finite()) {
        return Err(IconError::InvalidPath);
    }
    out.push(s);
    Ok(())
}
fn point(x: f64, y: f64) -> bool {
    x.is_finite() && y.is_finite() && (0.0..=24.0).contains(&x) && (0.0..=24.0).contains(&y)
}
fn pair(path: &str, at: &mut usize, bounds: &mut IconBounds) -> Result<(f64, f64), IconError> {
    let x = number(path, at)?;
    let y = number(path, at)?;
    if !point(x, y) {
        return Err(IconError::InvalidPath);
    }
    bounds.left = bounds.left.min(x);
    bounds.right = bounds.right.max(x);
    bounds.top = bounds.top.min(y);
    bounds.bottom = bounds.bottom.max(y);
    Ok((x, y))
}
fn cubic(
    out: &mut Vec<Segment>,
    p: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
    q: (f64, f64),
) -> Result<(), IconError> {
    let (mut x0, mut y0) = p;
    for i in 1..=16 {
        let t = i as f64 / 16.0;
        let u = 1.0 - t;
        let x = u * u * u * p.0 + 3.0 * u * u * t * a.0 + 3.0 * u * t * t * b.0 + t * t * t * q.0;
        let y = u * u * u * p.1 + 3.0 * u * u * t * a.1 + 3.0 * u * t * t * b.1 + t * t * t * q.1;
        add(
            out,
            Segment {
                x0,
                y0,
                x1: x,
                y1: y,
            },
        )?;
        x0 = x;
        y0 = y
    }
    Ok(())
}
fn quadratic(
    out: &mut Vec<Segment>,
    p: (f64, f64),
    a: (f64, f64),
    q: (f64, f64),
) -> Result<(), IconError> {
    let (mut x0, mut y0) = p;
    for i in 1..=12 {
        let t = i as f64 / 12.0;
        let u = 1.0 - t;
        let x = u * u * p.0 + 2.0 * u * t * a.0 + t * t * q.0;
        let y = u * u * p.1 + 2.0 * u * t * a.1 + t * t * q.1;
        add(
            out,
            Segment {
                x0,
                y0,
                x1: x,
                y1: y,
            },
        )?;
        x0 = x;
        y0 = y
    }
    Ok(())
}
fn arc(
    out: &mut Vec<Segment>,
    p: (f64, f64),
    mut rx: f64,
    mut ry: f64,
    rotation: f64,
    large: bool,
    sweep: bool,
    q: (f64, f64),
) -> Result<(), IconError> {
    if p == q {
        return Ok(());
    }
    rx = rx.abs();
    ry = ry.abs();
    if rx == 0.0 || ry == 0.0 {
        return add(
            out,
            Segment {
                x0: p.0,
                y0: p.1,
                x1: q.0,
                y1: q.1,
            },
        );
    }
    let phi = rotation * PI / 180.0;
    let (c, s) = (phi.cos(), phi.sin());
    let dx = (p.0 - q.0) * 0.5;
    let dy = (p.1 - q.1) * 0.5;
    let xp = c * dx + s * dy;
    let yp = -s * dx + c * dy;
    let lambda = xp * xp / (rx * rx) + yp * yp / (ry * ry);
    if lambda > 1.0 {
        let k = lambda.sqrt();
        rx *= k;
        ry *= k
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let den = rx2 * yp * yp + ry2 * xp * xp;
    let num = rx2 * ry2 - den;
    let mut f = if den == 0.0 {
        0.0
    } else {
        (num / den).max(0.0).sqrt()
    };
    if large == sweep {
        f = -f
    }
    let cxp = f * rx * yp / ry;
    let cyp = f * -ry * xp / rx;
    let cx = c * cxp - s * cyp + (p.0 + q.0) * 0.5;
    let cy = s * cxp + c * cyp + (p.1 + q.1) * 0.5;
    let ux = (xp - cxp) / rx;
    let uy = (yp - cyp) / ry;
    let vx = (-xp - cxp) / rx;
    let vy = (-yp - cyp) / ry;
    let start = uy.atan2(ux);
    let mut delta = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI
    }
    if sweep && delta < 0.0 {
        delta += 2.0 * PI
    }
    let steps = ((delta.abs() * rx.max(ry) / 0.65).ceil() as usize).clamp(4, 64);
    let (mut x0, mut y0) = p;
    for i in 1..=steps {
        let angle = start + delta * i as f64 / steps as f64;
        let mut x = cx + c * rx * angle.cos() - s * ry * angle.sin();
        let mut y = cy + s * rx * angle.cos() + c * ry * angle.sin();
        if i == steps {
            (x, y) = q
        }
        add(
            out,
            Segment {
                x0,
                y0,
                x1: x,
                y1: y,
            },
        )?;
        x0 = x;
        y0 = y
    }
    Ok(())
}
fn parse_path_data(path: &str) -> Result<(Vec<Segment>, IconBounds), IconError> {
    if path.is_empty() || path.len() > MAX_PATH {
        return Err(IconError::InvalidPath);
    }
    let mut out = Vec::new();
    out.try_reserve(MAX_SEGMENTS)
        .map_err(|_| IconError::Allocation)?;
    let mut bounds = IconBounds {
        left: 24.0,
        top: 24.0,
        right: 0.0,
        bottom: 0.0,
    };
    let (mut at, mut cur, mut start, mut has) = (0, (0.0, 0.0), (0.0, 0.0), false);
    while at < path.len() {
        while at < path.len() && matches!(path.as_bytes()[at], b' ' | b'\t' | b'\r' | b'\n' | b',')
        {
            at += 1
        }
        if at == path.len() {
            break;
        }
        let cmd = path.as_bytes()[at] as char;
        at += 1;
        if cmd == 'Z' {
            if !has {
                return Err(IconError::InvalidPath);
            }
            add(
                &mut out,
                Segment {
                    x0: cur.0,
                    y0: cur.1,
                    x1: start.0,
                    y1: start.1,
                },
            )?;
            cur = start;
            continue;
        }
        if !matches!(cmd, 'M' | 'L' | 'C' | 'Q' | 'A') || (cmd != 'M' && !has) {
            return Err(IconError::InvalidPath);
        }
        match cmd {
            'M' => {
                cur = pair(path, &mut at, &mut bounds)?;
                start = cur;
                has = true
            }
            'L' => {
                let q = pair(path, &mut at, &mut bounds)?;
                add(
                    &mut out,
                    Segment {
                        x0: cur.0,
                        y0: cur.1,
                        x1: q.0,
                        y1: q.1,
                    },
                )?;
                cur = q
            }
            'C' => {
                let a = pair(path, &mut at, &mut bounds)?;
                let b = pair(path, &mut at, &mut bounds)?;
                let q = pair(path, &mut at, &mut bounds)?;
                cubic(&mut out, cur, a, b, q)?;
                cur = q
            }
            'Q' => {
                let a = pair(path, &mut at, &mut bounds)?;
                let q = pair(path, &mut at, &mut bounds)?;
                quadratic(&mut out, cur, a, q)?;
                cur = q
            }
            'A' => {
                let rx = number(path, &mut at)?;
                let ry = number(path, &mut at)?;
                let rot = number(path, &mut at)?;
                let la = number(path, &mut at)?;
                let sw = number(path, &mut at)?;
                let q = pair(path, &mut at, &mut bounds)?;
                if rx < 0.0
                    || rx > 24.0
                    || ry < 0.0
                    || ry > 24.0
                    || rot.abs() > 360.0
                    || !matches!(la, 0.0 | 1.0)
                    || !matches!(sw, 0.0 | 1.0)
                {
                    return Err(IconError::InvalidPath);
                }
                arc(&mut out, cur, rx, ry, rot, la == 1.0, sw == 1.0, q)?;
                cur = q
            }
            _ => return Err(IconError::InvalidPath),
        }
    }
    if out.is_empty() || out.iter().any(|s| !point(s.x0, s.y0) || !point(s.x1, s.y1)) {
        return Err(IconError::InvalidPath);
    }
    for s in &out {
        bounds.left = bounds.left.min(s.x0);
        bounds.left = bounds.left.min(s.x1);
        bounds.right = bounds.right.max(s.x0).max(s.x1);
        bounds.top = bounds.top.min(s.y0).min(s.y1);
        bounds.bottom = bounds.bottom.max(s.y0).max(s.y1)
    }
    Ok((out, bounds))
}
pub(crate) fn parse_path(path: &str) -> Result<IconBounds, IconError> {
    parse_path_data(path).map(|(_, bounds)| bounds)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconCache {
    masks: [Option<Vec<u8>>; 216],
    legacy: [Option<Vec<u8>>; 30],
}
impl Default for IconCache {
    fn default() -> Self {
        Self {
            masks: std::array::from_fn(|_| None),
            legacy: std::array::from_fn(|_| None),
        }
    }
}
impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn clear(&mut self) {
        self.masks.fill(None);
        self.legacy.fill(None)
    }
    fn mask(&mut self, icon: IconKind, scale: usize) -> Result<&[u8], IconError> {
        if icon == IconKind::None
            || icon as usize >= IconKind::COUNT as usize
            || scale == 0
            || scale > MAX_SCALE
        {
            return Err(IconError::InvalidIcon);
        }
        let index = icon as usize * 4 + scale;
        if self.masks[index].is_none() {
            let def = crate::icon_path_at(icon as usize - 1).ok_or(IconError::InvalidIcon)?;
            let mut segs = Vec::new();
            segs.try_reserve(MAX_SEGMENTS)
                .map_err(|_| IconError::Allocation)?;
            // Parsing is repeated to obtain segments; the same strict parser backs public validation.
            segs = parse_path_data(def.path)?.0;
            let axis = AXIS * scale;
            let len = axis.checked_mul(axis).ok_or(IconError::Allocation)?;
            let mut mask = Vec::new();
            mask.try_reserve_exact(len)
                .map_err(|_| IconError::Allocation)?;
            mask.resize(len, 0);
            for py in 0..axis {
                for px in 0..axis {
                    let mut covered = 0u32;
                    for sy in 0..4 {
                        for sx in 0..4 {
                            let x = (px as f64 + (sx as f64 + 0.5) / 4.0) / scale as f64;
                            let y = (py as f64 + (sy as f64 + 0.5) / 4.0) / scale as f64;
                            if segs.iter().any(|s| {
                                let minx = s.x0.min(s.x1) - 0.8;
                                let maxx = s.x0.max(s.x1) + 0.8;
                                let miny = s.y0.min(s.y1) - 0.8;
                                let maxy = s.y0.max(s.y1) + 0.8;
                                x >= minx
                                    && x <= maxx
                                    && y >= miny
                                    && y <= maxy
                                    && distance2(x, y, s) <= 0.75 * 0.75
                            }) {
                                covered += 1
                            }
                        }
                    }
                    mask[py * axis + px] = ((covered * 255 + 8) / 16) as u8
                }
            }
            self.masks[index] = Some(mask);
        }
        self.masks[index].as_deref().ok_or(IconError::Allocation)
    }
    fn legacy_mask(
        &mut self,
        icon: IconKind,
        theme: Theme,
        scale: usize,
    ) -> Result<&[u8], IconError> {
        let ii = match icon {
            IconKind::SELECT => 0,
            IconKind::PAN => 1,
            IconKind::ZOOM_IN => 2,
            _ => return Err(IconError::InvalidIcon),
        };
        if scale == 0 || scale > 4 {
            return Err(IconError::InvalidScale);
        }
        let ti = usize::from(theme == Theme::Light);
        let ix = (ti * 5 + scale) * 3 + ii;
        if self.legacy[ix].is_none() {
            let axis = AXIS * scale;
            let mut mask = Vec::new();
            mask.try_reserve_exact(axis * axis)
                .map_err(|_| IconError::Allocation)?;
            mask.resize(axis * axis, 0);
            let geometry: &[(i32, i32, i32, i32)] = match ii {
                0 => &[
                    (4, 2, 19, 17),
                    (19, 17, 13, 17),
                    (13, 17, 10, 23),
                    (10, 23, 4, 2),
                    (4, 2, 4, 2),
                ],
                1 => &[
                    (12, 3, 12, 21),
                    (3, 12, 21, 12),
                    (12, 3, 9, 7),
                    (12, 3, 15, 7),
                    (12, 21, 9, 17),
                    (12, 21, 15, 17),
                    (3, 12, 7, 9),
                    (3, 12, 7, 15),
                    (21, 12, 17, 9),
                    (21, 12, 17, 15),
                ],
                _ => &[
                    (5, 12, 5, 10),
                    (5, 10, 6, 7),
                    (6, 7, 8, 5),
                    (8, 5, 11, 4),
                    (11, 4, 14, 5),
                    (14, 5, 16, 7),
                    (16, 7, 17, 10),
                    (17, 10, 17, 12),
                    (17, 12, 16, 15),
                    (16, 15, 14, 17),
                    (14, 17, 11, 18),
                    (11, 18, 8, 17),
                    (8, 17, 6, 15),
                    (6, 15, 5, 12),
                    (15, 15, 22, 22),
                    (8, 11, 14, 11),
                    (11, 8, 11, 14),
                ],
            };
            for &(x0, y0, x1, y1) in geometry {
                legacy_line(
                    &mut mask,
                    axis,
                    x0 * scale as i32,
                    y0 * scale as i32,
                    x1 * scale as i32,
                    y1 * scale as i32,
                    (1.6 * scale as f64).ceil() as i32,
                )
            }
            self.legacy[ix] = Some(mask)
        }
        self.legacy[ix].as_deref().ok_or(IconError::Allocation)
    }
}
fn distance2(x: f64, y: f64, s: &Segment) -> f64 {
    let dx = s.x1 - s.x0;
    let dy = s.y1 - s.y0;
    let len = dx * dx + dy * dy;
    let t = if len == 0.0 {
        0.0
    } else {
        (((x - s.x0) * dx + (y - s.y0) * dy) / len).clamp(0.0, 1.0)
    };
    let ox = x - (s.x0 + t * dx);
    let oy = y - (s.y0 + t * dy);
    ox * ox + oy * oy
}
fn legacy_line(
    mask: &mut [u8],
    axis: usize,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    stroke: i32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        let r = stroke / 2;
        for yy in -r..=r {
            for xx in -r..=r {
                let x = x0 + xx;
                let y = y0 + yy;
                if x >= 0 && y >= 0 && (x as usize) < axis && (y as usize) < axis {
                    mask[y as usize * axis + x as usize] = 255
                }
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e = 2 * err;
        if e >= dy {
            err += dy;
            x0 += sx
        }
        if e <= dx {
            err += dx;
            y0 += sy
        }
    }
}

fn paint_mask(
    mask: &[u8],
    axis: usize,
    bounds: Rect,
    color: u32,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) -> Result<(), IconError> {
    let ox = bounds
        .x
        .checked_add(bounds.width.saturating_sub(axis as u32) / 2)
        .ok_or(IconError::InvalidBuffer)?;
    let oy = bounds
        .y
        .checked_add(bounds.height.saturating_sub(axis as u32) / 2)
        .ok_or(IconError::InvalidBuffer)?;
    let dw = (bounds.width as usize).min(axis);
    let dh = (bounds.height as usize).min(axis);
    if ox > width
        || oy > height
        || dw > width as usize - ox as usize
        || dh > height as usize - oy as usize
    {
        return Err(IconError::InvalidBuffer);
    }
    for y in 0..dh {
        for x in 0..dw {
            let coverage = mask[y * axis + x];
            if coverage != 0 {
                let at = (oy as usize + y) * stride + (ox as usize + x) * 4;
                blend(&mut rgba[at..at + 4], color, coverage)
            }
        }
    }
    Ok(())
}
fn scale_value(s: f64, max: usize) -> Result<usize, IconError> {
    if !s.is_finite() || s < 0.5 || s > max as f64 {
        return Err(IconError::InvalidScale);
    }
    Ok(s.ceil() as usize)
}
impl IconCache {
    pub fn paint(
        &mut self,
        icon: IconKind,
        _theme: Theme,
        scale: f64,
        bounds: Rect,
        color: u32,
        rgba: &mut [u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<(), IconError> {
        if icon == IconKind::None {
            return Ok(());
        }
        if !valid_buffer(width, height, rgba.len(), stride) {
            return Err(IconError::InvalidBuffer);
        }
        let s = scale_value(scale, MAX_SCALE)?;
        let mask = self.mask(icon, s)?;
        paint_mask(mask, AXIS * s, bounds, color, rgba, width, height, stride)
    }
    pub fn paint_legacy(
        &mut self,
        icon: IconKind,
        theme: Theme,
        scale: f64,
        bounds: Rect,
        color: u32,
        rgba: &mut [u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<(), IconError> {
        if icon == IconKind::None {
            return Ok(());
        }
        if !valid_buffer(width, height, rgba.len(), stride) {
            return Err(IconError::InvalidBuffer);
        }
        let s = scale_value(scale, 4)?;
        let mask = self.legacy_mask(icon, theme, s)?;
        paint_mask(mask, AXIS * s, bounds, color, rgba, width, height, stride)
    }
    pub fn paint_state(
        &mut self,
        icon: IconKind,
        theme: Theme,
        _high_contrast: bool,
        scale: f64,
        bounds: Rect,
        state: ControlState,
        tokens: &Tokens,
        rgba: &mut [u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<(), IconError> {
        let color = match state {
            ControlState::Disabled => tokens.disabled_text,
            ControlState::Pressed | ControlState::Selected => tokens.accent,
            ControlState::Focus => tokens.focus_ring,
            ControlState::Invalid => tokens.error,
            ControlState::Rest | ControlState::Hover => tokens.text_primary,
        };
        self.paint(
            icon, theme, scale, bounds, color, rgba, width, height, stride,
        )
    }
    pub fn asset_hash(&mut self, theme: Theme, scale: u32) -> u64 {
        if !(1..=3).contains(&scale) {
            return 0;
        }
        let mut h = 1_469_598_103_934_665_603u64;
        for i in 1..54 {
            let Some(icon) = icon_from(i) else { return 0 };
            let Ok(mask) = self.mask(icon, scale as usize) else {
                return 0;
            };
            for b in mask {
                h ^= *b as u64;
                h = h.wrapping_mul(1_099_511_628_211)
            }
        }
        let _ = theme;
        h
    }
}
pub fn icon_from(value: u32) -> Option<IconKind> {
    if (1..54).contains(&value) {
        Some(kind_from_id(value as u8))
    } else {
        None
    }
}
fn kind_from_id(v: u8) -> IconKind {
    match v {
        1 => IconKind::SELECT,
        2 => IconKind::PAN,
        3 => IconKind::ZOOM_IN,
        4 => IconKind::ZOOM_OUT,
        5 => IconKind::RECTANGLE,
        6 => IconKind::ELLIPSE,
        7 => IconKind::LINE,
        8 => IconKind::PEN,
        9 => IconKind::NODE_EDIT,
        10 => IconKind::PENCIL,
        11 => IconKind::TEXT,
        12 => IconKind::EYEDROPPER,
        13 => IconKind::GRADIENT,
        14 => IconKind::FILL,
        15 => IconKind::STROKE,
        16 => IconKind::LAYERS,
        17 => IconKind::EYE,
        18 => IconKind::EYE_OFF,
        19 => IconKind::LOCK,
        20 => IconKind::UNLOCK,
        21 => IconKind::PLUS,
        22 => IconKind::MINUS,
        23 => IconKind::CLOSE,
        24 => IconKind::CHEVRON_RIGHT,
        25 => IconKind::CHEVRON_DOWN,
        26 => IconKind::MORE,
        27 => IconKind::SEARCH,
        28 => IconKind::SAVE,
        29 => IconKind::UNDO,
        30 => IconKind::REDO,
        31 => IconKind::DUPLICATE,
        32 => IconKind::TRASH,
        33 => IconKind::GROUP,
        34 => IconKind::UNGROUP,
        35 => IconKind::FIT,
        36 => IconKind::ACTUAL_SIZE,
        37 => IconKind::RULER,
        38 => IconKind::GRID,
        39 => IconKind::SNAP,
        40 => IconKind::ALIGN_LEFT,
        41 => IconKind::ALIGN_CENTER,
        42 => IconKind::ALIGN_RIGHT,
        43 => IconKind::ALIGN_TOP,
        44 => IconKind::ALIGN_MIDDLE,
        45 => IconKind::ALIGN_BOTTOM,
        46 => IconKind::CIRCLE,
        47 => IconKind::DOCUMENT,
        48 => IconKind::STOP,
        49 => IconKind::PATH,
        50 => IconKind::POLYLINE,
        51 => IconKind::IMAGE,
        52 => IconKind::SIDEBAR_LEFT,
        53 => IconKind::SIDEBAR_RIGHT,
        _ => IconKind::None,
    }
}
