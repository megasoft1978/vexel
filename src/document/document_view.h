#ifndef VEXEL_DOCUMENT_VIEW_H
#define VEXEL_DOCUMENT_VIEW_H

#include "document_model.h"

/* Read-only, allocation-free tree access for virtualized inspectors. The view
 * borrows a live model; callers must keep it alive and avoid concurrent edits. */
typedef const document_model *document_view;

typedef struct document_view_row {
    document_handle handle;
    size_t depth;
} document_view_row;

static inline size_t document_view_node_count(document_view view)
{
    return document_model_live_node_count(view);
}

static inline bool document_view_root(document_view view, document_handle *out)
{
    return document_model_root(view, out);
}

static inline bool document_view_first_child(document_view view, document_handle parent,
                                             document_handle *out)
{
    return document_model_first_child(view, parent, out);
}

static inline bool document_view_next_sibling(document_view view, document_handle node,
                                              document_handle *out)
{
    return document_model_next_sibling(view, node, out);
}

static inline bool document_view_tag(document_view view, document_handle node,
                                     base_bytes *out_name)
{
    return document_model_tag_name(view, node, out_name);
}

static inline size_t document_view_attribute_count(document_view view,
                                                   document_handle node)
{
    return document_model_attribute_count(view, node);
}

static inline bool document_view_attribute_at(document_view view, document_handle node,
                                              size_t index, base_bytes *out_name,
                                              base_bytes *out_value)
{
    return document_model_attribute_at(view, node, index, out_name, out_value);
}

/* Enumerates live rows in [first_slot, slot_limit), with stable slot order.
 * On output-capacity exhaustion, next_slot is the first slot not yet examined. */
static inline bool document_view_range(document_view view, size_t first_slot,
                                       size_t slot_limit, document_view_row *rows,
                                       size_t row_capacity, size_t *out_count,
                                       size_t *out_next_slot)
{
    if (view == NULL || out_count == NULL || out_next_slot == NULL ||
        (row_capacity > 0U && rows == NULL) || first_slot > slot_limit ||
        slot_limit > document_model_node_count(view)) return false;
    size_t cursor = first_slot;
    size_t count = 0U;
    while (cursor < slot_limit && count < row_capacity) {
        document_handle handle;
        if (document_model_node_at(view, cursor, &handle)) {
            size_t depth = 0U;
            if (!document_model_depth(view, handle, &depth)) return false;
            rows[count++] = (document_view_row){handle, depth};
        }
        ++cursor;
    }
    *out_count = count;
    *out_next_slot = cursor;
    return true;
}

#endif
