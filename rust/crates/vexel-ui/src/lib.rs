#![forbid(unsafe_code)]
//! Safe retained UI core: bounded layout, primitive fills, keyboard focus and owned semantics.

mod interaction;
mod draw;
mod icons;
mod icons_draw;
mod icon_paths;
mod paint;
mod theme;
mod tokens;
mod tree;
mod widget_add;
mod widget_input;
mod widget_canvas;

pub use interaction::{CommandBinding, CommandRegistry, SemanticsNode, SemanticsSnapshot};
pub use tokens::{design_metrics, theme_tokens, theme_tokens_accented, type_scale, Metrics, Motion, Theme, Tokens, TypeScale};
pub use tree::{Axis, Node, NodeSpec, Rect, Role, Tree, TreeError, MAX_COMMANDS, MAX_DEPTH, MAX_NODES, MAX_TEXT_BYTES, MAX_TEXT_TOTAL_BYTES, ROOT_ID};
pub use draw::scalar_text_draw;
pub use icons::{IconCache, IconBounds, IconError, IconKind, ControlState};
pub use icons_draw::icon_centered;
pub use paint::PaintError;
pub use icon_paths::{icon_definitions, icon_path_count, icon_path_at, icon_path_validate, icon_validate_all, IconDefinition};
pub use widget_add::{WidgetSpec, add_button, add_checkbox, add_text_field, add_slider, add_row};
pub use widget_input::{ColorField, Dropdown, InputResult, NumberField, PointerEvent, PointerState, Slider, TextEdit, Tone, number_parse, number_format, hex_parse, hex_format, MAX_EDIT_BYTES as WIDGET_MAX_EDIT_BYTES, MAX_LABEL_BYTES as WIDGET_MAX_LABEL_BYTES, MAX_UNIT_BYTES as WIDGET_MAX_UNIT_BYTES, MAX_OPTIONS as WIDGET_MAX_OPTIONS, MAX_SEGMENTS as WIDGET_MAX_SEGMENTS, MOD_SHIFT, MOD_COMMAND, MOD_OPTION, MOD_CONTROL, KEY_TAB, KEY_RETURN, KEY_SPACE, KEY_ESCAPE, KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_HOME, KEY_END, KEY_BACKSPACE, KEY_DELETE};
pub use widget_canvas::{WidgetCanvas, WidgetDrawSpec, WidgetKind, FontFamily, FontWeight, widget_paint, widget_paint_scratch_limit};
pub use vexel_raster::Tile;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_tokens;
#[cfg(test)]
mod tests_icons;
