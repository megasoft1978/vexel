//! Safe owned equivalents of the five legacy retained widget add wrappers.
use crate::{Axis, NodeSpec, Role, Tree, TreeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetSpec {
    pub id: u32,
    pub parent_id: u32,
    pub axis: Axis,
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub padding: u32,
    pub gap: u32,
    pub color: u32,
    pub command_id: u32,
    pub state: u32,
    pub widget_kind: u32,
    pub label: String,
    pub value: String,
    pub icon_kind: u32,
}
impl WidgetSpec {
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
            command_id: 0,
            state: 0,
            widget_kind: 0,
            label: String::new(),
            value: String::new(),
            icon_kind: 0,
        }
    }
}
fn add(tree: &mut Tree, s: &WidgetSpec, role: Role) -> Result<(), TreeError> {
    let mut n = NodeSpec::new(s.id, s.parent_id);
    n.axis = s.axis;
    n.preferred_width = s.preferred_width;
    n.preferred_height = s.preferred_height;
    n.padding = s.padding;
    n.gap = s.gap;
    n.color = s.color;
    n.role = role;
    n.command_id = s.command_id;
    n.state = s.state;
    n.widget_kind = s.widget_kind;
    n.label
        .try_reserve_exact(s.label.len())
        .map_err(|_| TreeError::Allocation)?;
    n.label.push_str(&s.label);
    n.value
        .try_reserve_exact(s.value.len())
        .map_err(|_| TreeError::Allocation)?;
    n.value.push_str(&s.value);
    n.icon_kind = s.icon_kind;
    tree.add(n)
}
pub fn add_button(tree: &mut Tree, s: &WidgetSpec) -> Result<(), TreeError> {
    add(tree, s, Role::Button)
}
pub fn add_checkbox(tree: &mut Tree, s: &WidgetSpec) -> Result<(), TreeError> {
    add(tree, s, Role::Checkbox)
}
pub fn add_text_field(tree: &mut Tree, s: &WidgetSpec) -> Result<(), TreeError> {
    add(tree, s, Role::TextField)
}
pub fn add_slider(tree: &mut Tree, s: &WidgetSpec) -> Result<(), TreeError> {
    add(tree, s, Role::Slider)
}
pub fn add_row(tree: &mut Tree, s: &WidgetSpec) -> Result<(), TreeError> {
    add(tree, s, Role::Row)
}
