#![forbid(unsafe_code)]
//! Safe retained UI core: bounded layout, primitive fills, keyboard focus and owned semantics.

mod draw;
mod icon_paths;
mod icons;
mod interaction;
mod paint;
mod tokens;
mod tree;
mod widget_add;
mod widget_canvas;
mod widget_input;

pub use draw::scalar_text_draw;
pub use icon_paths::{
    IconDefinition, icon_definitions, icon_path_at, icon_path_count, icon_path_validate,
    icon_validate_all,
};
pub use icons::{ControlState, IconBounds, IconCache, IconError, IconKind};
pub use interaction::{CommandBinding, CommandRegistry, SemanticsNode, SemanticsSnapshot};
pub use paint::PaintError;
pub use tokens::{
    Metrics, Theme, Tokens, TypeScale, design_metrics, theme_tokens, theme_tokens_accented,
    type_scale,
};
pub use tree::{
    Axis, MAX_COMMANDS, MAX_DEPTH, MAX_NODES, MAX_TEXT_BYTES, MAX_TEXT_TOTAL_BYTES, Node, NodeSpec,
    ROOT_ID, Rect, Role, Tree, TreeError,
};
pub use vexel_raster::Tile;
pub use widget_add::{WidgetSpec, add_button, add_checkbox, add_row, add_slider, add_text_field};
pub use widget_canvas::{
    FontFamily, FontWeight, WidgetCanvas, WidgetDrawSpec, WidgetKind, widget_paint,
    widget_paint_scratch_limit,
};
pub use widget_input::{
    ColorField, Dropdown, InputResult, KEY_BACKSPACE, KEY_DELETE, KEY_DOWN, KEY_END, KEY_ESCAPE,
    KEY_HOME, KEY_LEFT, KEY_RETURN, KEY_RIGHT, KEY_SPACE, KEY_TAB, KEY_UP,
    MAX_EDIT_BYTES as WIDGET_MAX_EDIT_BYTES, MAX_LABEL_BYTES as WIDGET_MAX_LABEL_BYTES,
    MAX_OPTIONS as WIDGET_MAX_OPTIONS, MAX_SEGMENTS as WIDGET_MAX_SEGMENTS,
    MAX_UNIT_BYTES as WIDGET_MAX_UNIT_BYTES, MOD_COMMAND, MOD_CONTROL, MOD_OPTION, MOD_SHIFT,
    NumberField, PointerEvent, PointerState, Slider, TextEdit, Tone, hex_format, hex_parse,
    number_format, number_parse,
};

#[cfg(test)]
mod tests;
