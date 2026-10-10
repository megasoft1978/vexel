#include "shape_scanline.h"

#include <math.h>
#include <stdlib.h>

#define RASTER_SCANLINE_MAX_TREE_DEPTH 64U

static const double sample_y_offsets[RASTER_SCANLINE_VALUES_PER_ROW] = {
    0.125, 0.375, 0.5, 0.625, 0.875
};

static double user_scanline_y(const svg_shape_document *doc, size_t slot,
                              double scale_y, double offset_y)
{
    size_t pixel_y = slot / RASTER_SCANLINE_VALUES_PER_ROW;
    size_t sample = slot % RASTER_SCANLINE_VALUES_PER_ROW;
    double device_y = (double)pixel_y + sample_y_offsets[sample];
    return doc->view_y + (device_y - offset_y) / scale_y;
}

static size_t first_scanline_at_or_above(const svg_shape_document *doc,
                                         size_t count, double target,
                                         double scale_y, double offset_y)
{
    size_t low = 0U;
    size_t high = count;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        double y = user_scanline_y(doc, middle, scale_y, offset_y);
        if (y < target) low = middle + 1U;
        else high = middle;
    }
    return low;
}

static int compare_events(const void *left_ptr, const void *right_ptr)
{
    const raster_scanline_event *left = left_ptr;
    const raster_scanline_event *right = right_ptr;
    if (left->slot < right->slot) return -1;
    if (left->slot > right->slot) return 1;
    if (left->edge < right->edge) return -1;
    if (left->edge > right->edge) return 1;
    if (left->add == right->add) return 0;
    return left->add ? 1 : -1;
}

static int compare_shape_events(const void *left_ptr, const void *right_ptr)
{
    const raster_shape_row_event *left = left_ptr;
    const raster_shape_row_event *right = right_ptr;
    if (left->row < right->row) return -1;
    if (left->row > right->row) return 1;
    if (left->add != right->add) return left->add ? 1 : -1;
    if (left->shape < right->shape) return -1;
    if (left->shape > right->shape) return 1;
    return 0;
}

static int32_t node_height(const raster_scanline_node *nodes, int32_t node)
{
    return node < 0 ? 0 : nodes[(size_t)node].height;
}

static void update_height(raster_scanline_node *nodes, int32_t node)
{
    int32_t left = nodes[(size_t)node].left;
    int32_t right = nodes[(size_t)node].right;
    int32_t left_height = node_height(nodes, left);
    int32_t right_height = node_height(nodes, right);
    nodes[(size_t)node].height = 1 + (left_height > right_height
                                        ? left_height : right_height);
}

static int32_t rotate_left(raster_scanline_node *nodes, int32_t root)
{
    int32_t pivot = nodes[(size_t)root].right;
    nodes[(size_t)root].right = nodes[(size_t)pivot].left;
    nodes[(size_t)pivot].left = root;
    update_height(nodes, root);
    update_height(nodes, pivot);
    return pivot;
}

static int32_t rotate_right(raster_scanline_node *nodes, int32_t root)
{
    int32_t pivot = nodes[(size_t)root].left;
    nodes[(size_t)root].left = nodes[(size_t)pivot].right;
    nodes[(size_t)pivot].right = root;
    update_height(nodes, root);
    update_height(nodes, pivot);
    return pivot;
}

static int32_t rebalance(raster_scanline_node *nodes, int32_t root)
{
    if (root < 0) return root;
    update_height(nodes, root);
    int32_t balance = node_height(nodes, nodes[(size_t)root].left) -
                      node_height(nodes, nodes[(size_t)root].right);
    if (balance > 1) {
        int32_t left = nodes[(size_t)root].left;
        if (node_height(nodes, nodes[(size_t)left].left) <
            node_height(nodes, nodes[(size_t)left].right))
            nodes[(size_t)root].left = rotate_left(nodes, left);
        return rotate_right(nodes, root);
    }
    if (balance < -1) {
        int32_t right = nodes[(size_t)root].right;
        if (node_height(nodes, nodes[(size_t)right].right) <
            node_height(nodes, nodes[(size_t)right].left))
            nodes[(size_t)root].right = rotate_right(nodes, right);
        return rotate_left(nodes, root);
    }
    return root;
}

static int32_t insert_node(raster_scanline_node *nodes, int32_t root,
                           int32_t key, size_t depth, bool *ok)
{
    if (depth >= RASTER_SCANLINE_MAX_TREE_DEPTH) {
        *ok = false;
        return root;
    }
    if (root < 0) return key;
    if (key < root) {
        nodes[(size_t)root].left = insert_node(nodes, nodes[(size_t)root].left,
                                               key, depth + 1U, ok);
    } else if (key > root) {
        nodes[(size_t)root].right = insert_node(nodes, nodes[(size_t)root].right,
                                                key, depth + 1U, ok);
    } else {
        *ok = false;
        return root;
    }
    return *ok ? rebalance(nodes, root) : root;
}

static int32_t remove_node(raster_scanline_node *nodes, int32_t root,
                           int32_t key, size_t depth, bool *ok)
{
    if (root < 0 || depth >= RASTER_SCANLINE_MAX_TREE_DEPTH) {
        *ok = false;
        return root;
    }
    if (key < root) {
        nodes[(size_t)root].left = remove_node(nodes, nodes[(size_t)root].left,
                                               key, depth + 1U, ok);
    } else if (key > root) {
        nodes[(size_t)root].right = remove_node(nodes, nodes[(size_t)root].right,
                                                key, depth + 1U, ok);
    } else {
        int32_t left = nodes[(size_t)root].left;
        int32_t right = nodes[(size_t)root].right;
        if (left < 0) return right;
        if (right < 0) return left;
        if (node_height(nodes, left) > node_height(nodes, right)) {
            root = rotate_right(nodes, root);
            nodes[(size_t)root].right = remove_node(
                nodes, nodes[(size_t)root].right, key, depth + 1U, ok);
        } else {
            root = rotate_left(nodes, root);
            nodes[(size_t)root].left = remove_node(
                nodes, nodes[(size_t)root].left, key, depth + 1U, ok);
        }
    }
    return *ok ? rebalance(nodes, root) : root;
}

static bool append_tree_in_order(const raster_scanline_node *nodes, int32_t root,
                                 uint32_t *destination, size_t capacity,
                                 size_t *cursor)
{
    int32_t stack[RASTER_SCANLINE_MAX_TREE_DEPTH];
    size_t depth = 0U;
    int32_t current = root;
    while (current >= 0 || depth != 0U) {
        while (current >= 0) {
            if (depth >= RASTER_SCANLINE_MAX_TREE_DEPTH) return false;
            stack[depth++] = current;
            current = nodes[(size_t)current].left;
        }
        current = stack[--depth];
        if (*cursor >= capacity) return false;
        destination[*cursor] = (uint32_t)current;
        (*cursor)++;
        current = nodes[(size_t)current].right;
    }
    return true;
}

bool raster_scanline_index_create(const svg_shape_document *doc,
                                  const raster_shape_bounds_set *bounds,
                                  double scale_y, double offset_y,
                                  raster_scanline_index *out,
                                  svg_shape_diagnostic *diagnostic)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
    if (out == NULL) return false;
    *out = (raster_scanline_index){0};
    if (doc == NULL || bounds == NULL || bounds->shape_count != doc->shape_count ||
        !isfinite(scale_y) || scale_y <= 0.0 ||
        (size_t)doc->height > SIZE_MAX / RASTER_SCANLINE_VALUES_PER_ROW ||
        doc->edge_count > SIZE_MAX / 2U || doc->shape_count > SIZE_MAX / 2U ||
        doc->edge_count > INT32_MAX || doc->shape_count > INT32_MAX ||
        doc->edge_count > SIZE_MAX / sizeof(*out->nodes) ||
        doc->shape_count > SIZE_MAX / sizeof(*out->roots)) return false;
    size_t max_events = doc->edge_count * 2U;
    size_t max_shape_events = doc->shape_count * 2U;
    size_t scanline_count = (size_t)doc->height * RASTER_SCANLINE_VALUES_PER_ROW;
    raster_scanline_event *events = max_events == 0U ? NULL :
        malloc(max_events * sizeof(*events));
    raster_scanline_node *nodes = doc->edge_count == 0U ? NULL :
        malloc(doc->edge_count * sizeof(*nodes));
    int32_t *roots = doc->shape_count == 0U ? NULL :
        malloc(doc->shape_count * sizeof(*roots));
    raster_shape_row_event *shape_events = max_shape_events == 0U ? NULL :
        malloc(max_shape_events * sizeof(*shape_events));
    raster_scanline_node *shape_nodes = doc->shape_count == 0U ? NULL :
        malloc(doc->shape_count * sizeof(*shape_nodes));
    if ((max_events != 0U && events == NULL) ||
        (doc->edge_count != 0U && nodes == NULL) ||
        (doc->shape_count != 0U && (roots == NULL || shape_nodes == NULL)) ||
        (max_shape_events != 0U && shape_events == NULL)) {
        free(events); free(nodes); free(roots); free(shape_events); free(shape_nodes);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    size_t shape_event_count = 0U;
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        roots[shape] = -1;
        const raster_pixel_bounds *box = &bounds->shapes[shape].pixels;
        if (!box->visible) continue;
        shape_events[shape_event_count++] = (raster_shape_row_event){
            box->y_first, (uint32_t)shape, true};
        shape_events[shape_event_count++] = (raster_shape_row_event){
            box->y_last + 1U, (uint32_t)shape, false};
    }
    size_t event_count = 0U;
    for (size_t edge_index = 0U; edge_index < doc->edge_count; edge_index++) {
        size_t shape = doc->shape_ids[edge_index];
        geom_edge edge = doc->edges[edge_index];
        if (!doc->shape_filled[shape] || isnan(edge.a.y) || isnan(edge.b.y) ||
            edge.a.y == edge.b.y) continue;
        double low_y = fmin(edge.a.y, edge.b.y);
        double high_y = fmax(edge.a.y, edge.b.y);
        size_t first = first_scanline_at_or_above(doc, scanline_count, low_y,
                                                   scale_y, offset_y);
        size_t end = first_scanline_at_or_above(doc, scanline_count, high_y,
                                                 scale_y, offset_y);
        if (first >= end || first >= scanline_count) continue;
        if (end > scanline_count) end = scanline_count;
        events[event_count++] = (raster_scanline_event){first,
            (uint32_t)edge_index, true};
        events[event_count++] = (raster_scanline_event){end,
            (uint32_t)edge_index, false};
    }
    if (event_count > 1U) qsort(events, event_count, sizeof(*events), compare_events);
    if (shape_event_count > 1U)
        qsort(shape_events, shape_event_count, sizeof(*shape_events), compare_shape_events);
    out->document = doc;
    out->events = events;
    out->nodes = nodes;
    out->roots = roots;
    out->shape_events = shape_events;
    out->shape_nodes = shape_nodes;
    out->shape_root = -1;
    out->event_count = event_count;
    out->shape_event_count = shape_event_count;
    out->shape_count = doc->shape_count;
    out->edge_count = doc->edge_count;
    out->scanline_count = scanline_count;
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    return true;
}

void raster_scanline_index_reset(raster_scanline_index *index)
{
    if (index == NULL) return;
    index->event_cursor = 0U;
    index->shape_event_cursor = 0U;
    index->listed_edge_count = 0U;
    index->shape_root = -1;
    for (size_t shape = 0U; shape < index->shape_count; shape++)
        index->roots[shape] = -1;
}

void raster_scanline_index_destroy(raster_scanline_index *index)
{
    if (index == NULL) return;
    free(index->events);
    free(index->nodes);
    free(index->roots);
    free(index->shape_events);
    free(index->shape_nodes);
    *index = (raster_scanline_index){0};
}

bool raster_scanline_index_build_row(raster_scanline_index *index,
                                     uint32_t pixel_y,
                                     size_t *shape_offsets,
                                     uint32_t *active_edges,
                                     size_t active_edge_capacity,
                                     uint32_t *active_shapes,
                                     size_t active_shape_capacity,
                                     size_t *active_shape_count)
{
    if (index == NULL || shape_offsets == NULL || active_shape_count == NULL ||
        (index->edge_count != 0U && active_edges == NULL) ||
        (index->shape_count != 0U && active_shapes == NULL) ||
        (size_t)pixel_y > SIZE_MAX / RASTER_SCANLINE_VALUES_PER_ROW) return false;
    *active_shape_count = 0U;
    while (index->shape_event_cursor < index->shape_event_count &&
           index->shape_events[index->shape_event_cursor].row <= pixel_y) {
        raster_shape_row_event event = index->shape_events[index->shape_event_cursor++];
        int32_t shape = (int32_t)event.shape;
        bool ok = true;
        if (event.add) {
            index->shape_nodes[event.shape] = (raster_scanline_node){-1, -1, 1};
            index->shape_root = insert_node(index->shape_nodes, index->shape_root,
                                            shape, 0U, &ok);
        } else {
            index->shape_root = remove_node(index->shape_nodes, index->shape_root,
                                            shape, 0U, &ok);
        }
        if (!ok) return false;
    }
    if (!append_tree_in_order(index->shape_nodes, index->shape_root, active_shapes,
                              active_shape_capacity, active_shape_count)) return false;
    size_t stride = index->shape_count + 1U;
    size_t write_cursor = 0U;
    for (size_t sample = 0U; sample < RASTER_SCANLINE_VALUES_PER_ROW; sample++) {
        size_t slot = (size_t)pixel_y * RASTER_SCANLINE_VALUES_PER_ROW + sample;
        while (index->event_cursor < index->event_count &&
               index->events[index->event_cursor].slot <= slot) {
            raster_scanline_event event = index->events[index->event_cursor++];
            size_t shape = index->document->shape_ids[event.edge];
            bool ok = true;
            if (event.add) {
                raster_scanline_node *node = &index->nodes[event.edge];
                *node = (raster_scanline_node){-1, -1, 1};
                index->roots[shape] = insert_node(index->nodes, index->roots[shape],
                    (int32_t)event.edge, 0U, &ok);
            } else {
                index->roots[shape] = remove_node(index->nodes, index->roots[shape],
                    (int32_t)event.edge, 0U, &ok);
            }
            if (!ok) return false;
        }
        size_t row_offset = sample * stride;
        for (size_t active = 0U; active < *active_shape_count; active++) {
            size_t shape = active_shapes[active];
            shape_offsets[row_offset + shape] = write_cursor;
            if (!append_tree_in_order(index->nodes, index->roots[shape], active_edges,
                                      active_edge_capacity, &write_cursor)) return false;
            shape_offsets[row_offset + shape + 1U] = write_cursor;
        }
    }
    index->listed_edge_count += write_cursor;
    return true;
}
