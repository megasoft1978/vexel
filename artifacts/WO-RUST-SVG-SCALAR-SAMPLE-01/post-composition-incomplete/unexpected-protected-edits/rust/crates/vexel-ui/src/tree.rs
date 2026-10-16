//! Bounded retained nodes, deterministic layout and primitive RGBA fill rendering.
use crate::icons::IconCache;
use crate::tokens::legacy_tokens;
use crate::{CommandRegistry, SemanticsSnapshot, Theme, Tokens};
use vexel_raster::{MAX_AXIS, Tile};

pub const MAX_NODES: usize = 128;
pub const MAX_DEPTH: usize = 24;
pub const ROOT_ID: u32 = 1;
pub const MAX_TEXT_BYTES: usize = 64;
pub const MAX_TEXT_TOTAL_BYTES: usize = 1024 * 1024;
pub const MAX_COMMANDS: usize = 64;
const STATE_DISABLED: u32 = 1;
const STATE_CHECKED: u32 = 2;
const STATE_FOCUSED: u32 = 4;
const NO_ICON: u32 = 54;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Overlay,
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    Window,
    Group,
    Button,
    Checkbox,
    Text,
    TextField,
    Slider,
    List,
    Row,
    Status,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpec {
    pub id: u32,
    pub parent_id: u32,
    pub axis: Axis,
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub padding: u32,
    pub gap: u32,
    pub color: u32,
    pub role: Role,
    pub label: String,
    pub value: String,
    pub state: u32,
    pub command_id: u32,
    pub icon_kind: u32,
    pub widget_kind: u32,
}
impl NodeSpec {
    pub fn new(id: u32, parent_id: u32) -> Self {
        Self {
            id,
            parent_id,
            axis: Axis::Overlay,
            preferred_width: 0,
            preferred_height: 0,
            padding: 0,
            gap: 0,
            color: 0,
            role: Role::None,
            label: String::new(),
            value: String::new(),
            state: 0,
            command_id: 0,
            icon_kind: 0,
            widget_kind: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub spec: NodeSpec,
    pub bounds: Rect,
    pub children: Vec<usize>,
    pub label_preview: String,
    pub value_preview: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeError {
    InvalidInput,
    Capacity,
    DuplicateId,
    MissingParent,
    InvalidUtf8,
    TextLimit,
    Layout,
    Allocation,
    NotLaidOut,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub text_bytes_used: usize,
    pub laid_out: bool,
    pub focused_id: u32,
    pub theme: Theme,
    pub increased_contrast: bool,
    pub semantic_style: bool,
    pub backing_scale: f64,
    pub tokens: Tokens,
    pub(crate) icon_cache: IconCache,
}

impl Tree {
    pub fn new(theme: Theme) -> Result<Self, TreeError> {
        let mut t = Self {
            nodes: Vec::new(),
            text_bytes_used: 0,
            laid_out: false,
            focused_id: 0,
            theme,
            increased_contrast: false,
            semantic_style: false,
            backing_scale: 0.0,
            tokens: legacy_tokens(theme),
            icon_cache: IconCache::new(),
        };
        let mut root = root_spec()?;
        root.color = t.tokens.window;
        t.add(root)?;
        Ok(t)
    }
    pub fn accessible(theme: Theme, increased_contrast: bool) -> Result<Self, TreeError> {
        let mut t = Self {
            nodes: Vec::new(),
            text_bytes_used: 0,
            laid_out: false,
            focused_id: 0,
            theme,
            increased_contrast,
            semantic_style: true,
            backing_scale: 0.0,
            tokens: crate::theme_tokens(theme, increased_contrast),
            icon_cache: IconCache::new(),
        };
        let mut root = root_spec()?;
        root.color = t.tokens.window;
        t.add(root)?;
        Ok(t)
    }
    pub fn find(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.spec.id == id)
    }
    pub fn find_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.spec.id == id)
    }
    pub fn add(&mut self, mut spec: NodeSpec) -> Result<(), TreeError> {
        let label_len = spec.label.len();
        let value_len = spec.value.len();
        if self.laid_out || self.nodes.len() >= MAX_NODES {
            return Err(TreeError::Capacity);
        }
        if spec.id == 0
            || spec.icon_kind >= NO_ICON
            || spec.preferred_width > MAX_AXIS
            || spec.preferred_height > MAX_AXIS
            || spec.padding > MAX_AXIS
            || spec.gap > MAX_AXIS
        {
            return Err(TreeError::InvalidInput);
        }
        if self.nodes.iter().any(|n| n.spec.id == spec.id) {
            return Err(TreeError::DuplicateId);
        }
        if label_len > MAX_TEXT_TOTAL_BYTES
            || value_len > MAX_TEXT_TOTAL_BYTES
            || label_len
                .checked_add(value_len)
                .and_then(|n| self.text_bytes_used.checked_add(n))
                .filter(|n| *n <= MAX_TEXT_TOTAL_BYTES)
                .is_none()
        {
            return Err(TreeError::TextLimit);
        }
        let parent = if self.nodes.is_empty() {
            if spec.id != ROOT_ID || spec.parent_id != 0 {
                return Err(TreeError::InvalidInput);
            }
            None
        } else {
            Some(
                self.nodes
                    .iter()
                    .position(|n| n.spec.id == spec.parent_id)
                    .ok_or(TreeError::MissingParent)?,
            )
        };
        let label_preview = preview(&spec.label)?;
        let value_preview = preview(&spec.value)?;
        if let Some(p) = parent {
            self.nodes[p]
                .children
                .try_reserve(1)
                .map_err(|_| TreeError::Allocation)?;
        }
        self.nodes
            .try_reserve(1)
            .map_err(|_| TreeError::Allocation)?;
        let idx = self.nodes.len();
        spec.state &= !STATE_FOCUSED;
        self.nodes.push(Node {
            spec,
            bounds: Rect::default(),
            children: Vec::new(),
            label_preview,
            value_preview,
        });
        if let Some(p) = parent {
            self.nodes[p].children.push(idx);
        }
        self.text_bytes_used += label_len + value_len;
        Ok(())
    }
    pub fn layout(&mut self, width: u32, height: u32) -> Result<(), TreeError> {
        if self.nodes.is_empty()
            || self.nodes.len() > MAX_NODES
            || self.laid_out
            || width == 0
            || height == 0
            || width > MAX_AXIS
            || height > MAX_AXIS
        {
            return Err(TreeError::Layout);
        }
        let mut bounds = Vec::new();
        bounds
            .try_reserve_exact(self.nodes.len())
            .map_err(|_| TreeError::Allocation)?;
        bounds.resize(self.nodes.len(), Rect::default());
        layout_node(
            &self.nodes,
            0,
            Rect {
                x: 0,
                y: 0,
                width,
                height,
            },
            0,
            &mut bounds,
        )?;
        for (n, b) in self.nodes.iter_mut().zip(bounds) {
            n.bounds = b;
        }
        self.laid_out = true;
        Ok(())
    }
    pub fn render(&self) -> Result<Tile, TreeError> {
        if !self.laid_out || self.nodes.is_empty() {
            return Err(TreeError::NotLaidOut);
        }
        let root = self.nodes[0].bounds;
        let mut tile = Tile::new(root.width, root.height).map_err(|_| TreeError::Allocation)?;
        tile.rgba_mut().fill(0);
        for node in &self.nodes {
            if node.spec.color & 255 == 0 {
                continue;
            }
            paint_rect(&mut tile, node.bounds, node.spec.color)?;
            if node.spec.state & STATE_FOCUSED != 0 {
                paint_ring(&mut tile, node.bounds, self.tokens.focus_ring)?;
            }
        }
        Ok(tile)
    }
    pub fn semantics(&self) -> Result<SemanticsSnapshot, TreeError> {
        SemanticsSnapshot::from_tree(self)
    }
    pub fn focus_next(&mut self, reverse: bool) -> bool {
        if self.nodes.len() < 2 {
            return false;
        }
        let current = self.nodes.iter().position(|n| n.spec.id == self.focused_id);
        let start = current.unwrap_or(if reverse { self.nodes.len() - 1 } else { 0 });
        for step in 1..=self.nodes.len() {
            let i = if reverse {
                (start + self.nodes.len() - (step % self.nodes.len())) % self.nodes.len()
            } else {
                (start + step) % self.nodes.len()
            };
            if focusable(&self.nodes[i]) {
                for n in &mut self.nodes {
                    n.spec.state &= !STATE_FOCUSED
                }
                self.nodes[i].spec.state |= STATE_FOCUSED;
                self.focused_id = self.nodes[i].spec.id;
                return true;
            }
        }
        false
    }
    pub fn dispatch_key(
        &mut self,
        registry: Option<&CommandRegistry>,
        key: u32,
        modifiers: u32,
    ) -> Option<u32> {
        const TAB: u32 = 0x110000;
        const RETURN: u32 = 0x110001;
        const SPACE: u32 = 0x110002;
        const SHIFT: u32 = 1;
        if key == TAB {
            return self.focus_next(modifiers & SHIFT != 0).then_some(0);
        }
        if key == RETURN || key == SPACE {
            let i = self
                .nodes
                .iter()
                .position(|n| n.spec.id == self.focused_id)?;
            if !focusable(&self.nodes[i]) {
                return None;
            }
            if self.nodes[i].spec.role == Role::Checkbox {
                self.nodes[i].spec.state ^= STATE_CHECKED;
                return Some(self.nodes[i].spec.command_id);
            }
            return (self.nodes[i].spec.command_id != 0).then_some(self.nodes[i].spec.command_id);
        }
        registry.and_then(|r| r.lookup(key, modifiers))
    }
}

fn root_spec() -> Result<NodeSpec, TreeError> {
    let mut label = String::new();
    label
        .try_reserve_exact(6)
        .map_err(|_| TreeError::Allocation)?;
    label.push_str("Editor");
    let mut root = NodeSpec::new(ROOT_ID, 0);
    root.axis = Axis::Vertical;
    root.role = Role::Window;
    root.label = label;
    Ok(root)
}
fn preview(s: &str) -> Result<String, TreeError> {
    let mut end = s.len().min(MAX_TEXT_BYTES - 1);
    while !s.is_char_boundary(end) {
        end -= 1
    }
    let mut out = String::new();
    out.try_reserve_exact(end)
        .map_err(|_| TreeError::Allocation)?;
    out.push_str(&s[..end]);
    Ok(out)
}
fn focusable(n: &Node) -> bool {
    matches!(
        n.spec.role,
        Role::Button | Role::Checkbox | Role::TextField | Role::Slider | Role::Row
    ) && n.spec.state & STATE_DISABLED == 0
}
fn layout_node(
    nodes: &[Node],
    i: usize,
    box_: Rect,
    depth: usize,
    out: &mut [Rect],
) -> Result<(), TreeError> {
    if depth > MAX_DEPTH {
        return Err(TreeError::Layout);
    }
    out[i] = box_;
    let n = &nodes[i];
    if n.children.is_empty() {
        return Ok(());
    }
    let twice = n.spec.padding.checked_mul(2).ok_or(TreeError::Layout)?;
    let x = box_
        .x
        .checked_add(n.spec.padding)
        .ok_or(TreeError::Layout)?;
    let y = box_
        .y
        .checked_add(n.spec.padding)
        .ok_or(TreeError::Layout)?;
    let iw = if box_.width > twice {
        box_.width - twice
    } else {
        0
    };
    let ih = if box_.height > twice {
        box_.height - twice
    } else {
        0
    };
    if n.spec.axis == Axis::Overlay {
        for &c in &n.children {
            let cn = &nodes[c];
            let w = if cn.spec.preferred_width == 0 {
                iw
            } else {
                cn.spec.preferred_width.min(iw)
            };
            let h = if cn.spec.preferred_height == 0 {
                ih
            } else {
                cn.spec.preferred_height.min(ih)
            };
            layout_node(
                nodes,
                c,
                Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                },
                depth + 1,
                out,
            )?
        }
        return Ok(());
    }
    let horizontal = n.spec.axis == Axis::Horizontal;
    let main = if horizontal { iw } else { ih };
    let cross = if horizontal { ih } else { iw };
    let gaps = n
        .spec
        .gap
        .checked_mul((n.children.len() - 1) as u32)
        .ok_or(TreeError::Layout)?;
    let mut fixed = 0u32;
    let mut flexible = 0u32;
    for &c in &n.children {
        let wanted = if horizontal {
            nodes[c].spec.preferred_width
        } else {
            nodes[c].spec.preferred_height
        };
        if wanted == 0 {
            flexible += 1
        } else {
            fixed = fixed.checked_add(wanted).ok_or(TreeError::Layout)?
        }
    }
    let available = main - main.min(gaps);
    let mut flex_extent = available.saturating_sub(fixed);
    let mut remaining = main;
    let mut cursor = if horizontal { x } else { y };
    for (pos, &c) in n.children.iter().enumerate() {
        if pos > 0 && remaining > 0 {
            let gap = n.spec.gap.min(remaining);
            cursor = cursor.checked_add(gap).ok_or(TreeError::Layout)?;
            remaining -= gap
        }
        let cn = &nodes[c];
        let preferred = if horizontal {
            cn.spec.preferred_width
        } else {
            cn.spec.preferred_height
        };
        let mut extent = if preferred == 0 {
            if flexible == 0 {
                0
            } else {
                flex_extent / flexible
            }
        } else {
            preferred
        };
        extent = extent.min(remaining);
        if preferred == 0 && flexible > 0 {
            flex_extent = flex_extent.saturating_sub(extent);
            flexible -= 1
        }
        let cp = if horizontal {
            cn.spec.preferred_height
        } else {
            cn.spec.preferred_width
        };
        let cextent = if cp == 0 || cp > cross { cross } else { cp };
        let child_box = if horizontal {
            Rect {
                x: cursor,
                y,
                width: extent,
                height: cextent,
            }
        } else {
            Rect {
                x,
                y: cursor,
                width: cextent,
                height: extent,
            }
        };
        layout_node(nodes, c, child_box, depth + 1, out)?;
        cursor = cursor.checked_add(extent).ok_or(TreeError::Layout)?;
        remaining -= extent;
    }
    Ok(())
}
fn paint_rect(tile: &mut Tile, r: Rect, color: u32) -> Result<(), TreeError> {
    if color & 255 == 0 {
        return Ok(());
    }
    let width = tile.width();
    let x2 =
        r.x.checked_add(r.width)
            .ok_or(TreeError::Layout)?
            .min(width);
    let y2 =
        r.y.checked_add(r.height)
            .ok_or(TreeError::Layout)?
            .min(tile.height());
    let src = [(color >> 24) as u8, (color >> 16) as u8, (color >> 8) as u8];
    let rgba = tile.rgba_mut();
    for y in r.y.min(y2)..y2 {
        for x in r.x.min(x2)..x2 {
            let at = ((y as usize)
                .checked_mul(width as usize)
                .and_then(|v| v.checked_add(x as usize))
                .and_then(|v| v.checked_mul(4)))
            .ok_or(TreeError::Layout)?;
            rgba[at..at + 3].copy_from_slice(&src);
            rgba[at + 3] = 255
        }
    }
    Ok(())
}
fn paint_ring(tile: &mut Tile, r: Rect, color: u32) -> Result<(), TreeError> {
    if r.width < 2 || r.height < 2 {
        return Ok(());
    }
    paint_rect(tile, Rect { height: 1, ..r }, color)?;
    paint_rect(
        tile,
        Rect {
            x: r.x,
            y: r.y + r.height - 1,
            width: r.width,
            height: 1,
        },
        color,
    )?;
    paint_rect(
        tile,
        Rect {
            x: r.x,
            y: r.y + 1,
            width: 1,
            height: r.height - 2,
        },
        color,
    )?;
    paint_rect(
        tile,
        Rect {
            x: r.x + r.width - 1,
            y: r.y + 1,
            width: 1,
            height: r.height - 2,
        },
        color,
    )
}
