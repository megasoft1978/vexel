//! Retained-tree RGBA painter: semantic fills, clipped text, icon masks and focus rings.
use crate::{
    MAX_NODES, MAX_TEXT_BYTES, Rect, Role, Tree,
    draw::{blend, glyph, valid_buffer},
    icons::{IconCache, icon_from},
};

const DISABLED: u32 = 1;
const CHECKED: u32 = 2;
const FOCUSED: u32 = 4;
const HOVERED: u32 = 1 << 4;
const PRESSED: u32 = 1 << 5;
const INVALID: u32 = 1 << 6;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintError {
    NotLaidOut,
    InvalidBuffer,
    InvalidTree,
    Icon(crate::IconError),
}
impl Tree {
    /// Paint all retained content into caller-owned storage after validating the full RGBA layout.
    pub fn paint_into(&mut self, rgba: &mut [u8], stride: usize) -> Result<(), PaintError> {
        self.paint_internal(rgba, stride, true)
    }
    /// Paint fills, borders, focus and icons while leaving text to the native platform presenter.
    pub fn paint_backgrounds_into(
        &mut self,
        rgba: &mut [u8],
        stride: usize,
    ) -> Result<(), PaintError> {
        self.paint_internal(rgba, stride, false)
    }
    fn paint_internal(
        &mut self,
        rgba: &mut [u8],
        stride: usize,
        text: bool,
    ) -> Result<(), PaintError> {
        if !self.laid_out || self.nodes.is_empty() || self.nodes.len() > MAX_NODES {
            return Err(PaintError::NotLaidOut);
        }
        let root = self.nodes[0].bounds;
        if !valid_buffer(root.width, root.height, rgba.len(), stride) {
            return Err(PaintError::InvalidBuffer);
        }
        for n in &self.nodes {
            if n.bounds.x.checked_add(n.bounds.width).is_none()
                || n.bounds.y.checked_add(n.bounds.height).is_none()
            {
                return Err(PaintError::InvalidTree);
            }
        }
        let (width, height) = (root.width, root.height);
        let cache = &mut self.icon_cache;
        for n in &self.nodes {
            paint_node(
                n,
                self.theme,
                self.backing_scale,
                self.semantic_style,
                self.tokens,
                cache,
                rgba,
                width,
                height,
                stride,
                text,
            )?
        }
        Ok(())
    }
}
fn clipped(r: Rect, width: u32, height: u32) -> Option<Rect> {
    if r.x >= width || r.y >= height {
        return None;
    }
    let right = r.x.checked_add(r.width)?.min(width);
    let bottom = r.y.checked_add(r.height)?.min(height);
    Some(Rect {
        x: r.x,
        y: r.y,
        width: right - r.x,
        height: bottom - r.y,
    })
}
fn inside_corner(x: u32, y: u32, w: u32, h: u32, r: u32) -> bool {
    if r == 0 {
        return true;
    }
    let dx = if x < r {
        r - x
    } else if x >= w - r {
        x - (w - r - 1)
    } else {
        0
    };
    let dy = if y < r {
        r - y
    } else if y >= h - r {
        y - (h - r - 1)
    } else {
        0
    };
    dx == 0 || dy == 0 || (dx as u64 * dx as u64 + dy as u64 * dy as u64 <= r as u64 * r as u64)
}
fn fill(
    r: Rect,
    color: u32,
    radius: u32,
    outline: bool,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) {
    let Some(c) = clipped(r, width, height) else {
        return;
    };
    let rad = radius.min(c.width / 2).min(c.height / 2);
    for y in c.y..c.y + c.height {
        for x in c.x..c.x + c.width {
            let edge = x == r.x
                || y == r.y
                || x + 1 == r.x.saturating_add(r.width)
                || y + 1 == r.y.saturating_add(r.height);
            if outline && !edge {
                continue;
            }
            if !inside_corner(
                x - r.x,
                y - r.y,
                r.width.min(width - r.x),
                r.height.min(height - r.y),
                rad,
            ) {
                continue;
            }
            let at = y as usize * stride + x as usize * 4;
            blend(&mut rgba[at..at + 4], color, 255)
        }
    }
}
fn text_line(
    s: &str,
    x: u32,
    y: u32,
    scale: u32,
    color: u32,
    clip: Rect,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    stride: usize,
) {
    let right = clip.x.saturating_add(clip.width).min(width);
    let bottom = clip.y.saturating_add(clip.height).min(height);
    let mut cursor = x;
    for ch in s.chars().take(MAX_TEXT_BYTES) {
        let c = if ch.is_ascii() { ch as u8 } else { b'?' };
        if cursor > right || right - cursor < 5 {
            break;
        }
        for row in 0..7 {
            for col in 0..5 {
                if glyph(c, row) & (1 << (4 - col)) == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = cursor + col as u32 * scale + sx;
                        let py = y + row as u32 * scale + sy;
                        if px >= clip.x && py >= clip.y && px < right && py < bottom {
                            let at = py as usize * stride + px as usize * 4;
                            blend(&mut rgba[at..at + 4], color, 255)
                        }
                    }
                }
            }
        }
        cursor = match cursor.checked_add(6 * scale) {
            Some(v) => v,
            None => break,
        }
    }
}
fn paint_node(
    n: &crate::Node,
    theme: crate::Theme,
    backing: f64,
    semantic: bool,
    t: crate::Tokens,
    cache: &mut IconCache,
    rgba: &mut [u8],
    w: u32,
    h: u32,
    stride: usize,
    draw_text: bool,
) -> Result<(), PaintError> {
    let role = n.spec.role;
    let state = n.spec.state;
    let actionable = matches!(role, Role::Button | Role::TextField | Role::Row);
    let mut color = n.spec.color;
    if state & PRESSED != 0 && actionable {
        color = t.selection_fill
    } else if state & HOVERED != 0 && actionable {
        color = t.panel_raised
    }
    let radius = if semantic && matches!(role, Role::Button | Role::TextField) {
        t.radius
    } else {
        0
    };
    if color & 255 != 0 {
        fill(n.bounds, color, radius, false, rgba, w, h, stride)
    }
    if semantic && (role == Role::TextField || (role == Role::Button && n.spec.widget_kind != 2)) {
        fill(n.bounds, t.control_edge, t.radius, true, rgba, w, h, stride)
    }
    if state & FOCUSED != 0 && n.spec.widget_kind != 16 && n.bounds.width > 1 && n.bounds.height > 1
    {
        let r = n.bounds;
        fill(
            Rect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: 1,
            },
            t.focus_ring,
            0,
            false,
            rgba,
            w,
            h,
            stride,
        );
        fill(
            Rect {
                x: r.x,
                y: r.y + r.height - 1,
                width: r.width,
                height: 1,
            },
            t.focus_ring,
            0,
            false,
            rgba,
            w,
            h,
            stride,
        );
        fill(
            Rect {
                x: r.x,
                y: r.y + 1,
                width: 1,
                height: r.height - 2,
            },
            t.focus_ring,
            0,
            false,
            rgba,
            w,
            h,
            stride,
        );
        fill(
            Rect {
                x: r.x + r.width - 1,
                y: r.y + 1,
                width: 1,
                height: r.height - 2,
            },
            t.focus_ring,
            0,
            false,
            rgba,
            w,
            h,
            stride,
        )
    }
    let mut ink = if state & INVALID != 0 {
        t.error
    } else if state & DISABLED != 0 {
        t.text_secondary
    } else {
        t.text_primary
    };
    let scale = (t.font_px / 13 + u32::from(t.font_px % 13 != 0)).clamp(1, 16);
    if role == Role::Row && state & CHECKED != 0 {
        ink = t.accent
    }
    if n.spec.icon_kind != 0 {
        if !draw_text {
            return Ok(());
        }
        let icon =
            icon_from(n.spec.icon_kind).ok_or(PaintError::Icon(crate::IconError::InvalidIcon))?;
        if state & CHECKED != 0 {
            ink = t.accent
        }
        cache
            .paint_legacy(
                icon,
                theme,
                if backing > 0.0 { backing } else { 1.0 },
                n.bounds,
                ink,
                rgba,
                w,
                h,
                stride,
            )
            .map_err(PaintError::Icon)?;
        return Ok(());
    }
    if !draw_text {
        return Ok(());
    }
    let glyph_h = 7 * scale;
    let y = n.bounds.y + n.bounds.height.saturating_sub(glyph_h) / 2;
    let inset = 8 * scale;
    let mut x = n.bounds.x + if n.bounds.width > inset { inset } else { 0 };
    if role == Role::Button && n.spec.label.len() <= 40 {
        let tw = n.spec.label.len() as u32 * 6 * scale;
        if tw < n.bounds.width {
            x = n.bounds.x + (n.bounds.width - tw) / 2
        }
    }
    text_line(
        &n.label_preview,
        x,
        y,
        scale,
        ink,
        n.bounds,
        rgba,
        w,
        h,
        stride,
    );
    if role == Role::TextField && !n.value_preview.is_empty() {
        text_line(
            &n.value_preview,
            n.bounds.x + n.bounds.width / 2,
            y,
            scale,
            ink,
            n.bounds,
            rgba,
            w,
            h,
            stride,
        )
    }
    Ok(())
}
