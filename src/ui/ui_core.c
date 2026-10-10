#include "ui_core.h"

#include "raster/shape_render.h"
#include "shape.h"

#include <stdio.h>
#include <string.h>

#define UI_NO_NODE UINT16_MAX
#define UI_SVG_CAPACITY 256U

static uint32_t ui_color(uint8_t red, uint8_t green, uint8_t blue)
{
    return ((uint32_t)red << 24U) | ((uint32_t)green << 16U) |
           ((uint32_t)blue << 8U) | 255U;
}

ui_tokens ui_theme_tokens(ui_theme theme, bool increased_contrast)
{
    if (theme == UI_THEME_LIGHT) {
        uint32_t secondary = increased_contrast ? ui_color(58, 70, 86) : ui_color(82, 97, 113);
        uint32_t tertiary = increased_contrast ? ui_color(70, 82, 99) : ui_color(116, 128, 146);
        uint32_t disabled = increased_contrast ? ui_color(80, 91, 108) : ui_color(137, 147, 162);
        uint32_t separator = increased_contrast ? ui_color(151, 160, 172) : ui_color(201, 208, 218);
        uint32_t edge = increased_contrast ? ui_color(71, 82, 99) : ui_color(124, 135, 151);
        ui_tokens tokens = {
            .window = ui_color(242, 244, 247), .panel = ui_color(255, 255, 255),
            .panel_raised = ui_color(237, 240, 244), .canvas_well = ui_color(231, 235, 240),
            .panel_hover = ui_color(245, 247, 250), .pressed_fill = ui_color(220, 234, 255),
            .text_primary = ui_color(31, 39, 50), .text_secondary = secondary,
            .text_tertiary = tertiary, .disabled_text = disabled,
            .separator = separator, .control_edge = edge, .accent = ui_color(24, 95, 197),
            .selection_fill = ui_color(220, 234, 255), .selection_edge = ui_color(24, 95, 197),
            .focus_ring = increased_contrast ? ui_color(0, 73, 156) : ui_color(0, 95, 204),
            .checker_a = ui_color(224, 228, 234), .checker_b = ui_color(243, 245, 247),
            .guide = ui_color(208, 0, 168), .success = ui_color(20, 122, 83),
            .warning = ui_color(138, 90, 0), .error = ui_color(180, 35, 24),
            .popover_shadow = UINT32_C(0x00000024),
            .spacing = 8U, .radius = 6U, .font_px = 13U,
            .spacing_4 = 4U, .spacing_8 = 8U, .spacing_12 = 12U,
            .spacing_16 = 16U, .spacing_24 = 24U, .spacing_32 = 32U,
            .radius_small = 4U, .radius_control = 6U, .radius_large = 10U
        };
        tokens.success = increased_contrast ? ui_color(13, 101, 65) : tokens.success;
        tokens.warning = increased_contrast ? ui_color(107, 67, 0) : tokens.warning;
        tokens.error = increased_contrast ? ui_color(153, 28, 18) : tokens.error;
        tokens.panel_alt = tokens.panel_raised;
        tokens.canvas = tokens.canvas_well;
        tokens.text = tokens.text_primary;
        tokens.muted = tokens.text_secondary;
        tokens.border = tokens.separator;
        return tokens;
    }
    if (theme != UI_THEME_DARK) return (ui_tokens){0};
    uint32_t secondary = increased_contrast ? ui_color(222, 228, 236) : ui_color(185, 194, 207);
    uint32_t tertiary = increased_contrast ? ui_color(207, 215, 225) : ui_color(144, 153, 166);
    uint32_t disabled = increased_contrast ? ui_color(191, 200, 211) : ui_color(124, 134, 148);
    uint32_t separator = increased_contrast ? ui_color(89, 98, 111) : ui_color(59, 65, 75);
    uint32_t edge = increased_contrast ? ui_color(166, 177, 191) : ui_color(116, 128, 145);
    ui_tokens tokens = {
        .window = ui_color(23, 26, 32), .panel = ui_color(32, 36, 43),
        .panel_raised = ui_color(41, 46, 55), .canvas_well = ui_color(17, 21, 27),
        .panel_hover = ui_color(48, 54, 64), .pressed_fill = ui_color(38, 60, 88),
        .text_primary = ui_color(241, 244, 248), .text_secondary = secondary,
        .text_tertiary = tertiary, .disabled_text = disabled,
        .separator = separator, .control_edge = edge, .accent = ui_color(131, 180, 255),
        .selection_fill = ui_color(38, 60, 88), .selection_edge = ui_color(131, 180, 255),
        .focus_ring = increased_contrast ? ui_color(190, 218, 255) : ui_color(156, 197, 255),
        .checker_a = ui_color(41, 46, 54), .checker_b = ui_color(32, 36, 43),
        .guide = ui_color(255, 90, 200), .success = ui_color(102, 214, 164),
        .warning = ui_color(244, 193, 93), .error = ui_color(255, 142, 134),
        .popover_shadow = UINT32_C(0x0000006b),
        .spacing = 8U, .radius = 6U, .font_px = 13U,
        .spacing_4 = 4U, .spacing_8 = 8U, .spacing_12 = 12U,
        .spacing_16 = 16U, .spacing_24 = 24U, .spacing_32 = 32U,
        .radius_small = 4U, .radius_control = 6U, .radius_large = 10U
    };
    tokens.panel_alt = tokens.panel_raised;
    tokens.canvas = tokens.canvas_well;
    tokens.text = tokens.text_primary;
    tokens.muted = tokens.text_secondary;
    tokens.border = tokens.separator;
    return tokens;
}

ui_type_scale ui_theme_type_scale(void)
{
    return (ui_type_scale){.document_title_pt = 14U, .panel_heading_pt = 14U,
        .section_heading_pt = 13U, .body_pt = 13U, .secondary_pt = 12U,
        .metadata_pt = 11U, .geometry_pt = 12U, .empty_heading_pt = 22U,
        .geometry_family = UI_FONT_MONOSPACED, .title_weight = UI_FONT_SEMIBOLD,
        .section_weight = UI_FONT_SEMIBOLD, .empty_heading_weight = UI_FONT_SEMIBOLD};
}

ui_metrics ui_design_metrics(void)
{
    return (ui_metrics){.toolbar_height = 40U, .context_height = 32U,
        .tool_rail_width = 44U, .structure_width = 232U, .inspector_width = 320U,
        .status_height = 24U, .tree_row_height = 26U, .section_header_height = 32U,
        .field_height = 30U, .icon_button_target = 28U, .icon_button_glyph = 17U,
        .control_radius = 6U};
}

static ui_tokens ui_legacy_theme_tokens(ui_theme theme)
{
    ui_tokens tokens = {0};
    if (theme == UI_THEME_LIGHT) {
        tokens.window = ui_color(242, 244, 247); tokens.panel = ui_color(255, 255, 255);
        tokens.panel_alt = ui_color(232, 236, 242); tokens.canvas = ui_color(250, 251, 253);
        tokens.text = ui_color(30, 36, 45); tokens.muted = ui_color(105, 115, 128);
        tokens.accent = ui_color(45, 111, 219); tokens.border = ui_color(211, 217, 225);
    } else {
        tokens.window = ui_color(25, 28, 34); tokens.panel = ui_color(35, 39, 47);
        tokens.panel_alt = ui_color(44, 49, 59); tokens.canvas = ui_color(21, 24, 30);
        tokens.text = ui_color(233, 237, 244); tokens.muted = ui_color(148, 157, 171);
        tokens.accent = ui_color(105, 159, 255); tokens.border = ui_color(55, 61, 72);
    }
    tokens.panel_raised = tokens.panel_alt; tokens.canvas_well = tokens.canvas;
    tokens.text_primary = tokens.text; tokens.text_secondary = tokens.muted;
    tokens.separator = tokens.border; tokens.control_edge = tokens.border;
    tokens.selection_fill = tokens.panel_alt; tokens.focus_ring = tokens.accent;
    tokens.error = theme == UI_THEME_DARK ? ui_color(255, 142, 134) : ui_color(180, 35, 24);
    tokens.spacing = 8U; tokens.radius = 6U; tokens.font_px = 13U;
    return tokens;
}

static uint16_t ui_index(const ui_tree *tree, uint32_t id)
{
    if (tree == NULL || tree->count > UI_MAX_NODES || id == 0U) return UI_NO_NODE;
    for (size_t index = 0; index < tree->count; index++) {
        if (tree->nodes[index].occupied && tree->nodes[index].spec.id == id) {
            return (uint16_t)index;
        }
    }
    return UI_NO_NODE;
}

const ui_node *ui_tree_find(const ui_tree *tree, uint32_t id)
{
    uint16_t index = ui_index(tree, id);
    return index == UI_NO_NODE ? NULL : &tree->nodes[index];
}

bool ui_tree_init(ui_tree *tree, ui_theme theme)
{
    if (tree == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    memset(tree, 0, sizeof(*tree));
    tree->tokens = ui_legacy_theme_tokens(theme);
    tree->theme = theme;
    ui_node_spec root = {0};
    root.id = UI_ROOT_ID;
    root.axis = UI_AXIS_VERTICAL;
    root.color = tree->tokens.window;
    root.role = UI_ROLE_WINDOW;
    root.label = "Editor";
    root.label_len = 6U;
    return ui_tree_add(tree, &root);
}

bool ui_tree_init_accessible(ui_tree *tree, ui_theme theme, bool increased_contrast)
{
    if (tree == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    memset(tree, 0, sizeof(*tree));
    tree->tokens = ui_theme_tokens(theme, increased_contrast);
    tree->theme = theme;
    tree->increased_contrast = increased_contrast;
    tree->semantic_style = true;
    ui_node_spec root = {0};
    root.id = UI_ROOT_ID;
    root.axis = UI_AXIS_VERTICAL;
    root.color = tree->tokens.window;
    root.role = UI_ROLE_WINDOW;
    root.label = "Editor";
    root.label_len = 6U;
    return ui_tree_add(tree, &root);
}

static bool ui_utf8_valid(const char *text, size_t length)
{
    if (length > 0U && text == NULL) return false;
    size_t index = 0U;
    while (index < length) {
        uint8_t first = (uint8_t)text[index];
        size_t continuation = 0U;
        uint8_t lower = 0x80U, upper = 0xbfU;
        if (first < 0x80U) { index++; continue; }
        if (first >= 0xc2U && first <= 0xdfU) continuation = 1U;
        else if (first == 0xe0U) { continuation = 2U; lower = 0xa0U; }
        else if (first >= 0xe1U && first <= 0xecU) continuation = 2U;
        else if (first == 0xedU) { continuation = 2U; upper = 0x9fU; }
        else if (first >= 0xeeU && first <= 0xefU) continuation = 2U;
        else if (first == 0xf0U) { continuation = 3U; lower = 0x90U; }
        else if (first >= 0xf1U && first <= 0xf3U) continuation = 3U;
        else if (first == 0xf4U) { continuation = 3U; upper = 0x8fU; }
        else return false;
        if (continuation > length - index - 1U) return false;
        uint8_t second = (uint8_t)text[index + 1U];
        if (second < lower || second > upper) return false;
        for (size_t offset = 2U; offset <= continuation; offset++) {
            uint8_t byte = (uint8_t)text[index + offset];
            if (byte < 0x80U || byte > 0xbfU) return false;
        }
        index += continuation + 1U;
    }
    return true;
}

bool ui_tree_add(ui_tree *tree, const ui_node_spec *spec)
{
    if (tree == NULL || spec == NULL || tree->text_bytes_used > UI_TEXT_TOTAL_MAX_BYTES ||
        tree->laid_out || tree->count >= UI_MAX_NODES ||
        spec->id == 0U || spec->axis > UI_AXIS_VERTICAL ||
        spec->icon >= UI_ICON_COUNT ||
        spec->preferred_width > RASTER_TILE_MAX_AXIS ||
        spec->preferred_height > RASTER_TILE_MAX_AXIS ||
        spec->padding > RASTER_TILE_MAX_AXIS || spec->gap > RASTER_TILE_MAX_AXIS ||
        (spec->role < UI_ROLE_NONE || spec->role > UI_ROLE_STATUS) ||
        spec->label_len > UI_TEXT_TOTAL_MAX_BYTES || spec->value_len > UI_TEXT_TOTAL_MAX_BYTES ||
        spec->label_len > UI_TEXT_TOTAL_MAX_BYTES - tree->text_bytes_used ||
        spec->value_len > UI_TEXT_TOTAL_MAX_BYTES - tree->text_bytes_used - spec->label_len ||
        !ui_utf8_valid(spec->label, spec->label_len) ||
        !ui_utf8_valid(spec->value, spec->value_len) ||
        ui_index(tree, spec->id) != UI_NO_NODE) {
        return false;
    }
    uint16_t parent = UI_NO_NODE;
    if (tree->count == 0U) {
        if (spec->id != UI_ROOT_ID || spec->parent_id != 0U) return false;
    } else {
        parent = ui_index(tree, spec->parent_id);
        if (parent == UI_NO_NODE || tree->nodes[parent].child_count == UINT16_MAX) return false;
    }
    uint16_t added = (uint16_t)tree->count;
    ui_node *new_node = &tree->nodes[added];
    size_t label_preview_len = spec->label_len < UI_TEXT_MAX_BYTES - 1U ?
        spec->label_len : UI_TEXT_MAX_BYTES - 1U;
    while (label_preview_len > 0U && spec->label_len > label_preview_len &&
           spec->label != NULL && (((uint8_t)spec->label[label_preview_len] & 0xc0U) == 0x80U))
        label_preview_len--;
    size_t value_preview_len = spec->value_len < UI_TEXT_MAX_BYTES - 1U ?
        spec->value_len : UI_TEXT_MAX_BYTES - 1U;
    while (value_preview_len > 0U && spec->value_len > value_preview_len &&
           spec->value != NULL && (((uint8_t)spec->value[value_preview_len] & 0xc0U) == 0x80U))
        value_preview_len--;
    size_t label_offset = tree->text_bytes_used;
    size_t value_offset = label_offset + spec->label_len;
    *new_node = (ui_node){.spec = *spec, .first_child = UI_NO_NODE,
        .next_sibling = UI_NO_NODE, .occupied = true,
        .full_label_len = spec->label_len, .full_value_len = spec->value_len,
        .label_offset = label_offset, .value_offset = value_offset};
    new_node->spec.label = NULL;
    new_node->spec.value = NULL;
    new_node->spec.label_len = label_preview_len;
    new_node->spec.value_len = value_preview_len;
    new_node->spec.state &= ~(uint32_t)UI_STATE_FOCUSED;
    if (spec->label_len > 0U) memcpy(tree->text_storage + label_offset, spec->label, spec->label_len);
    if (spec->value_len > 0U) memcpy(tree->text_storage + value_offset, spec->value, spec->value_len);
    if (label_preview_len > 0U) memcpy(new_node->label, spec->label, label_preview_len);
    if (value_preview_len > 0U) memcpy(new_node->value, spec->value, value_preview_len);
    tree->text_bytes_used += spec->label_len + spec->value_len;
    tree->count++;
    if (parent != UI_NO_NODE) {
        ui_node *owner = &tree->nodes[parent];
        if (owner->first_child == UI_NO_NODE) owner->first_child = added;
        else {
            uint16_t last = owner->first_child;
            while (tree->nodes[last].next_sibling != UI_NO_NODE) last = tree->nodes[last].next_sibling;
            tree->nodes[last].next_sibling = added;
        }
        owner->child_count++;
    }
    return true;
}

static bool ui_layout_node(ui_tree *tree, uint16_t index, ui_rect box, uint32_t depth)
{
    if (depth > UI_MAX_DEPTH) return false;
    ui_node *node = &tree->nodes[index];
    node->bounds = box;
    if (node->child_count == 0U) return true;
    uint32_t twice_padding = node->spec.padding * 2U;
    uint32_t inner_x = box.x + node->spec.padding;
    uint32_t inner_y = box.y + node->spec.padding;
    uint32_t inner_width = box.width > twice_padding ? box.width - twice_padding : 0U;
    uint32_t inner_height = box.height > twice_padding ? box.height - twice_padding : 0U;
    if (node->spec.axis == UI_AXIS_OVERLAY) {
        for (uint16_t child = node->first_child; child != UI_NO_NODE;
             child = tree->nodes[child].next_sibling) {
            ui_node *item = &tree->nodes[child];
            uint32_t width = item->spec.preferred_width == 0U ? inner_width : item->spec.preferred_width;
            uint32_t height = item->spec.preferred_height == 0U ? inner_height : item->spec.preferred_height;
            if (width > inner_width) width = inner_width;
            if (height > inner_height) height = inner_height;
            if (!ui_layout_node(tree, child, (ui_rect){inner_x, inner_y, width, height}, depth + 1U)) {
                return false;
            }
        }
        return true;
    }
    uint32_t main_extent = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_width : inner_height;
    uint32_t cross_extent = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_height : inner_width;
    uint32_t gaps = node->spec.gap * (uint32_t)(node->child_count - 1U);
    uint32_t fixed = 0U, flexible = 0U;
    for (uint16_t child = node->first_child; child != UI_NO_NODE; child = tree->nodes[child].next_sibling) {
        ui_node_spec *spec = &tree->nodes[child].spec;
        uint32_t wanted = node->spec.axis == UI_AXIS_HORIZONTAL ? spec->preferred_width : spec->preferred_height;
        if (wanted == 0U) flexible++;
        else if (UINT32_MAX - fixed < wanted) return false;
        else fixed += wanted;
    }
    uint32_t bounded_gaps = gaps > main_extent ? main_extent : gaps;
    uint32_t available = main_extent - bounded_gaps;
    uint32_t flex_extent = available > fixed ? available - fixed : 0U;
    uint32_t remaining = available;
    uint32_t cursor = node->spec.axis == UI_AXIS_HORIZONTAL ? inner_x : inner_y;
    uint16_t child = node->first_child;
    while (child != UI_NO_NODE) {
        if (child != node->first_child && remaining > 0U) {
            uint32_t gap = node->spec.gap < remaining ? node->spec.gap : remaining;
            cursor += gap;
            remaining -= gap;
        }
        ui_node *item = &tree->nodes[child];
        uint32_t preferred = node->spec.axis == UI_AXIS_HORIZONTAL ? item->spec.preferred_width : item->spec.preferred_height;
        uint32_t extent = preferred == 0U ? (flexible == 0U ? 0U : flex_extent / flexible) : preferred;
        if (extent > remaining) extent = remaining;
        if (preferred == 0U && flexible > 0U) {
            flex_extent = flex_extent > extent ? flex_extent - extent : 0U;
            flexible--;
        }
        uint32_t cross = node->spec.axis == UI_AXIS_HORIZONTAL ? item->spec.preferred_height : item->spec.preferred_width;
        if (cross == 0U || cross > cross_extent) cross = cross_extent;
        ui_rect child_box = node->spec.axis == UI_AXIS_HORIZONTAL
            ? (ui_rect){cursor, inner_y, extent, cross}
            : (ui_rect){inner_x, cursor, cross, extent};
        if (!ui_layout_node(tree, child, child_box, depth + 1U)) return false;
        cursor += extent;
        remaining -= extent;
        child = item->next_sibling;
    }
    return true;
}

bool ui_tree_layout(ui_tree *tree, uint32_t width, uint32_t height)
{
    if (tree == NULL || tree->count == 0U || tree->count > UI_MAX_NODES ||
        tree->laid_out || width == 0U || height == 0U ||
        width > RASTER_TILE_MAX_AXIS || height > RASTER_TILE_MAX_AXIS) return false;
    if (!ui_layout_node(tree, 0U, (ui_rect){0U, 0U, width, height}, 0U)) return false;
    tree->laid_out = true;
    return true;
}

static bool ui_render_node(const ui_node *node, raster_tile *tile)
{
    char svg[UI_SVG_CAPACITY];
    uint32_t rgba = node->spec.color;
    int length = snprintf(svg, sizeof(svg),
        "<svg width=\"4\" height=\"4\"><rect width=\"4\" height=\"4\" fill=\"#%06x\"/></svg>",
        (rgba >> 8U) & 0xffffffU);
    if (length < 0 || (size_t)length >= sizeof(svg)) return false;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    base_bytes bytes = {(const uint8_t *)svg, (size_t)length};
    if (svg_shape_decode(bytes, &document, &diagnostic) != SVG_SHAPE_ERROR_NONE) return false;
    raster_tile sample = {0};
    bool okay = raster_shape_render(&document, &sample, &diagnostic);
    svg_shape_document_destroy(&document);
    if (!okay) return false;
    size_t sample_at = ((size_t)(sample.height / 2U) * sample.width + sample.width / 2U) * 4U;
    const uint8_t *paint = sample.rgba + sample_at;
    uint32_t x_end = node->bounds.x + node->bounds.width;
    uint32_t y_end = node->bounds.y + node->bounds.height;
    if (x_end > tile->width) x_end = tile->width;
    if (y_end > tile->height) y_end = tile->height;
    for (uint32_t y = node->bounds.y; y < y_end; y++) {
        for (uint32_t x = node->bounds.x; x < x_end; x++) {
            size_t at = ((size_t)y * tile->width + x) * 4U;
            uint32_t alpha = paint[3];
            uint32_t inverse = 255U - alpha;
            for (size_t channel = 0U; channel < 3U; channel++) {
                uint32_t value = (uint32_t)paint[channel] * alpha +
                    (uint32_t)tile->rgba[at + channel] * inverse;
                tile->rgba[at + channel] = (uint8_t)((value + 127U) / 255U);
            }
            tile->rgba[at + 3U] = 255U;
        }
    }
    raster_tile_destroy(&sample);
    return true;
}

static bool ui_render_focus_ring(const ui_node *node, uint32_t accent, raster_tile *tile)
{
    if (node->bounds.width < 2U || node->bounds.height < 2U) return true;
    ui_node edge = *node;
    edge.spec.color = accent;
    edge.bounds.height = 1U;
    if (!ui_render_node(&edge, tile)) return false;
    edge.bounds.y = node->bounds.y + node->bounds.height - 1U;
    if (!ui_render_node(&edge, tile)) return false;
    edge.bounds = (ui_rect){node->bounds.x, node->bounds.y + 1U, 1U, node->bounds.height - 2U};
    if (!ui_render_node(&edge, tile)) return false;
    edge.bounds.x = node->bounds.x + node->bounds.width - 1U;
    return ui_render_node(&edge, tile);
}

bool ui_tree_render(const ui_tree *tree, raster_tile *out_tile)
{
    if (tree == NULL || out_tile == NULL || !tree->laid_out || tree->count == 0U ||
        tree->count > UI_MAX_NODES) return false;
    *out_tile = (raster_tile){0};
    const ui_rect root = tree->nodes[0].bounds;
    if (!raster_tile_create(root.width, root.height, out_tile)) return false;
    for (size_t index = 0U; index < tree->count; index++) {
        const ui_node *node = &tree->nodes[index];
        if ((node->spec.color & 255U) == 0U) continue;
        if (!ui_render_node(node, out_tile) ||
            ((node->spec.state & UI_STATE_FOCUSED) != 0U &&
        !ui_render_focus_ring(node, tree->tokens.focus_ring, out_tile))) {
            raster_tile_destroy(out_tile);
            return false;
        }
    }
    return true;
}


static bool ui_role_focusable(ui_role role)
{
    return role == UI_ROLE_BUTTON || role == UI_ROLE_CHECKBOX ||
        role == UI_ROLE_TEXT_FIELD || role == UI_ROLE_SLIDER || role == UI_ROLE_ROW;
}

static bool ui_node_focusable(const ui_node *node)
{
    return node != NULL && node->occupied && ui_role_focusable(node->spec.role) &&
        (node->spec.state & UI_STATE_DISABLED) == 0U;
}

bool ui_focus_next(ui_tree *tree, bool reverse)
{
    if (tree == NULL || tree->count < 2U) return false;
    uint16_t current = ui_index(tree, tree->focused_id);
    size_t start = current == UI_NO_NODE ? (reverse ? tree->count - 1U : 0U) : (size_t)current;
    for (size_t step = 1U; step <= tree->count; step++) {
        size_t candidate;
        if (reverse) candidate = (start + tree->count - (step % tree->count)) % tree->count;
        else candidate = (start + step) % tree->count;
        if (ui_node_focusable(&tree->nodes[candidate])) {
            for (size_t index = 0U; index < tree->count; index++) {
                tree->nodes[index].spec.state &= ~(uint32_t)UI_STATE_FOCUSED;
            }
            tree->nodes[candidate].spec.state |= UI_STATE_FOCUSED;
            tree->focused_id = tree->nodes[candidate].spec.id;
            return true;
        }
    }
    return false;
}

bool ui_semantics_snapshot_build(const ui_tree *tree, ui_semantics_snapshot *out_snapshot)
{
    if (tree == NULL || out_snapshot == NULL || tree->count > UI_MAX_NODES ||
        tree->text_bytes_used > UI_TEXT_TOTAL_MAX_BYTES) return false;
    memset(out_snapshot, 0, sizeof(*out_snapshot));
    for (size_t index = 0U; index < tree->count; index++) {
        const ui_node *source = &tree->nodes[index];
        if (!source->occupied || source->spec.label_len > UI_TEXT_MAX_BYTES ||
            source->spec.value_len > UI_TEXT_MAX_BYTES ||
            source->full_label_len > UI_TEXT_TOTAL_MAX_BYTES ||
            source->full_value_len > UI_TEXT_TOTAL_MAX_BYTES ||
            source->label_offset > tree->text_bytes_used ||
            source->full_label_len > tree->text_bytes_used - source->label_offset ||
            source->value_offset > tree->text_bytes_used ||
            source->full_value_len > tree->text_bytes_used - source->value_offset) return false;
        ui_semantics_node *node = &out_snapshot->nodes[index];
        node->id = source->spec.id;
        node->parent_id = source->spec.parent_id;
        node->role = source->spec.role;
        node->state = source->spec.state;
        node->command_id = source->spec.command_id;
        node->label_len = source->spec.label_len;
        node->value_len = source->spec.value_len;
        node->label_full_len = source->full_label_len;
        node->value_full_len = source->full_value_len;
        node->frame = source->bounds;
        memcpy(node->label, source->label, node->label_len);
        memcpy(node->value, source->value, node->value_len);
        if (node->label_full_len > UI_TEXT_TOTAL_MAX_BYTES - out_snapshot->text_bytes_used ||
            node->value_full_len > UI_TEXT_TOTAL_MAX_BYTES - out_snapshot->text_bytes_used -
                node->label_full_len) return false;
        node->label_offset = out_snapshot->text_bytes_used;
        if (node->label_full_len > 0U)
            memcpy(out_snapshot->text_storage + node->label_offset,
                tree->text_storage + source->label_offset, node->label_full_len);
        node->value_offset = node->label_offset + node->label_full_len;
        if (node->value_full_len > 0U)
            memcpy(out_snapshot->text_storage + node->value_offset,
                tree->text_storage + source->value_offset, node->value_full_len);
        out_snapshot->text_bytes_used += node->label_full_len + node->value_full_len;
    }
    out_snapshot->count = tree->count;
    return true;
}

bool ui_semantics_text(const ui_semantics_snapshot *snapshot,
                       const ui_semantics_node *node, bool value,
                       const char **out_text, size_t *out_length)
{
    if (snapshot == NULL || node == NULL || out_text == NULL || out_length == NULL ||
        snapshot->count > UI_MAX_NODES || snapshot->text_bytes_used > UI_TEXT_TOTAL_MAX_BYTES)
        return false;
    size_t offset = value ? node->value_offset : node->label_offset;
    size_t length = value ? node->value_full_len : node->label_full_len;
    if (offset > snapshot->text_bytes_used || length > snapshot->text_bytes_used - offset)
        return false;
    *out_text = snapshot->text_storage + offset;
    *out_length = length;
    return true;
}

bool ui_command_registry_init(ui_command_registry *registry)
{
    if (registry == NULL) return false;
    *registry = (ui_command_registry){0};
    return true;
}

bool ui_command_register(ui_command_registry *registry, uint32_t command_id,
                         uint32_t key, uint32_t modifiers, bool enabled)
{
    if (registry == NULL || command_id == 0U || key == 0U ||
        (modifiers & ~(UI_MOD_SHIFT | UI_MOD_COMMAND | UI_MOD_OPTION | UI_MOD_CONTROL)) != 0U ||
        registry->count >= UI_MAX_COMMANDS) return false;
    for (size_t index = 0U; index < registry->count; index++) {
        const ui_command_binding *binding = &registry->bindings[index];
        if (binding->command_id == command_id ||
            (binding->key == key && binding->modifiers == modifiers)) return false;
    }
    registry->bindings[registry->count++] = (ui_command_binding){command_id, key, modifiers, enabled};
    return true;
}

static bool ui_command_lookup(const ui_command_registry *registry, uint32_t key,
                              uint32_t modifiers, uint32_t *out_command_id)
{
    if (registry == NULL || registry->count > UI_MAX_COMMANDS) return false;
    for (size_t index = 0U; index < registry->count; index++) {
        const ui_command_binding *binding = &registry->bindings[index];
        if (binding->enabled && binding->key == key && binding->modifiers == modifiers) {
            *out_command_id = binding->command_id;
            return true;
        }
    }
    return false;
}

bool ui_dispatch_key(ui_tree *tree, const ui_command_registry *registry,
                     uint32_t key, uint32_t modifiers, uint32_t *out_command_id)
{
    if (tree == NULL || out_command_id == NULL) return false;
    *out_command_id = 0U;
    if (key == UI_KEY_TAB) return ui_focus_next(tree, (modifiers & UI_MOD_SHIFT) != 0U);
    if (key == UI_KEY_RETURN || key == UI_KEY_SPACE) {
        uint16_t index = ui_index(tree, tree->focused_id);
        if (index == UI_NO_NODE || !ui_node_focusable(&tree->nodes[index])) return false;
        ui_node *focused = &tree->nodes[index];
        if (focused->spec.role == UI_ROLE_CHECKBOX) {
            focused->spec.state ^= (uint32_t)UI_STATE_CHECKED;
            *out_command_id = focused->spec.command_id;
            return true;
        }
        if (focused->spec.command_id == 0U) return false;
        *out_command_id = focused->spec.command_id;
        return true;
    }
    return ui_command_lookup(registry, key, modifiers, out_command_id);
}
