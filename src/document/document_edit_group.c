#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>

typedef struct document_group_node {
    document_handle handle;
    document_handle parent;
    document_edit_node_info *info;
} document_group_node;

static int group_node_compare(const void *left, const void *right)
{
    const document_group_node *a = left;
    const document_group_node *b = right;
    if (a->info->span.offset < b->info->span.offset) return -1;
    if (a->info->span.offset > b->info->span.offset) return 1;
    return 0;
}

static bool handles_equal(document_handle left, document_handle right)
{
    return left.document_id == right.document_id && left.slot == right.slot &&
        left.generation == right.generation;
}

static document_edit_error collect_group_nodes(document_edit *edit,
    const document_handle *handles, size_t count, bool require_groups,
    document_group_node nodes[DOCUMENT_EDIT_GROUP_NODE_LIMIT],
    document_handle *out_parent)
{
    if (edit == NULL || handles == NULL || nodes == NULL || out_parent == NULL ||
        count == 0U) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (count > DOCUMENT_EDIT_GROUP_NODE_LIMIT) return DOCUMENT_EDIT_LIMIT;
    const document_model *model = document_edit_model(edit);
    if (model == NULL) return DOCUMENT_EDIT_INVALID_OPERATION;
    document_handle common_parent = {0U};
    for (size_t index = 0U; index < count; ++index) {
        document_edit_node_info *info = NULL;
        document_handle parent = {0U};
        if (!document_edit_handle_info(edit, handles[index], &info) ||
            !document_model_parent(model, handles[index], &parent))
            return DOCUMENT_EDIT_STALE_HANDLE;
        if (index == 0U) common_parent = parent;
        else if (!handles_equal(common_parent, parent))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        if (require_groups) {
            base_bytes tag = {0};
            if (!document_model_tag_name(model, handles[index], &tag))
                return DOCUMENT_EDIT_STALE_HANDLE;
            if (tag.len != 1U || tag.ptr[0] != 'g')
                return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        nodes[index] = (document_group_node){handles[index], parent, info};
    }
    qsort(nodes, count, sizeof(nodes[0]), group_node_compare);
    size_t previous_end = 0U;
    for (size_t index = 0U; index < count; ++index) {
        size_t end;
        if (!base_size_add(nodes[index].info->span.offset,
                nodes[index].info->span.length, &end) ||
            (index > 0U && nodes[index - 1U].info->span.offset >=
                nodes[index].info->span.offset) ||
            (index > 0U && previous_end > nodes[index].info->span.offset) ||
            end > document_edit_byte_length(edit))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        if (index + 1U < count) {
            document_handle next = {0U};
            if (!document_model_next_sibling(model, nodes[index].handle, &next) ||
                !handles_equal(next, nodes[index + 1U].handle))
                return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        previous_end = end;
    }
    *out_parent = common_parent;
    return DOCUMENT_EDIT_OK;
}

static bool group_region(document_edit *edit,
    const document_group_node nodes[DOCUMENT_EDIT_GROUP_NODE_LIMIT], size_t count,
    document_source_span *out_region)
{
    if (edit == NULL || nodes == NULL || count == 0U || out_region == NULL)
        return false;
    size_t start = nodes[0].info->span.offset;
    size_t last_end;
    if (!base_size_add(nodes[count - 1U].info->span.offset,
            nodes[count - 1U].info->span.length, &last_end) || last_end < start ||
        last_end - start > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT) return false;
    *out_region = (document_source_span){start, last_end - start};
    return true;
}

document_edit_error document_edit_group_siblings(document_edit *edit,
    const document_handle *handles, size_t count, document_handle *out_group)
{
    if (out_group == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_group_node nodes[DOCUMENT_EDIT_GROUP_NODE_LIMIT] = {{0}};
    document_handle parent = {0U};
    document_edit_error error = collect_group_nodes(edit, handles, count, false,
        nodes, &parent);
    if (error != DOCUMENT_EDIT_OK) return error;
    document_source_span region = {0U, 0U};
    if (!group_region(edit, nodes, count, &region) ||
        region.length > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT - 7U)
        return DOCUMENT_EDIT_LIMIT;
    base_bytes original = {0};
    if (!document_edit_span_bytes(edit, region, &original))
        return DOCUMENT_EDIT_INVALID_OPERATION;
    size_t replacement_length;
    if (!base_size_add(original.len, 7U, &replacement_length))
        return DOCUMENT_EDIT_LIMIT;
    uint8_t *replacement = malloc(replacement_length);
    if (replacement == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    replacement[0] = '<'; replacement[1] = 'g'; replacement[2] = '>';
    if (original.len > 0U)
        memcpy(replacement + 3U, original.ptr, original.len);
    replacement[3U + original.len] = '<';
    replacement[4U + original.len] = '/';
    replacement[5U + original.len] = 'g';
    replacement[6U + original.len] = '>';
    error = document_edit_replace_xml_region(edit, parent, region,
        (base_bytes){replacement, replacement_length});
    free(replacement);
    if (error != DOCUMENT_EDIT_OK) return error;
    return document_edit_element_at_offset(edit, region.offset, out_group) ?
        DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
}

document_edit_error document_edit_ungroup_siblings(document_edit *edit,
    const document_handle *handles, size_t count)
{
    document_group_node nodes[DOCUMENT_EDIT_GROUP_NODE_LIMIT] = {{0}};
    document_handle parent = {0U};
    document_edit_error error = collect_group_nodes(edit, handles, count, true,
        nodes, &parent);
    if (error != DOCUMENT_EDIT_OK) return error;
    document_source_span region = {0U, 0U};
    if (!group_region(edit, nodes, count, &region)) return DOCUMENT_EDIT_LIMIT;
    size_t new_length = region.length;
    for (size_t index = 0U; index < count; ++index) {
        const document_edit_node_info *info = nodes[index].info;
        size_t wrapper_length = info->span.length;
        if (!info->empty) {
            size_t span_end, open_end, close_end;
            if (!base_size_add(info->span.offset, info->span.length, &span_end) ||
                info->open.offset < info->span.offset ||
                info->open.offset - info->span.offset > info->span.length ||
                info->open.length > info->span.length -
                    (info->open.offset - info->span.offset) ||
                !base_size_add(info->open.offset, info->open.length, &open_end) ||
                info->close.offset < open_end ||
                info->close.offset > span_end ||
                !base_size_add(info->close.offset, info->close.length, &close_end) ||
                close_end > span_end)
                return DOCUMENT_EDIT_INVALID_OPERATION;
            size_t inner_length = info->close.offset - open_end;
            wrapper_length = info->span.length - inner_length;
        }
        if (wrapper_length > new_length) return DOCUMENT_EDIT_INVALID_OPERATION;
        new_length -= wrapper_length;
    }
    if (new_length > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT)
        return DOCUMENT_EDIT_LIMIT;
    uint8_t *replacement = new_length == 0U ? NULL : malloc(new_length);
    if (new_length > 0U && replacement == NULL)
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t cursor = region.offset;
    size_t output = 0U;
    for (size_t index = 0U; index < count; ++index) {
        const document_edit_node_info *info = nodes[index].info;
        size_t node_end;
        if (!base_size_add(info->span.offset, info->span.length, &node_end) ||
            info->span.offset < cursor) {
            free(replacement);
            return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        size_t gap_length = info->span.offset - cursor;
        if (output > new_length || gap_length > new_length - output ||
            (gap_length > 0U && !document_edit_read_range(edit, cursor,
                gap_length, replacement + output))) {
            free(replacement);
            return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        output += gap_length;
        if (!info->empty) {
            size_t inner_start;
            if (!base_size_add(info->open.offset, info->open.length,
                    &inner_start) || inner_start > info->close.offset) {
                free(replacement);
                return DOCUMENT_EDIT_INVALID_OPERATION;
            }
            size_t inner_length = info->close.offset - inner_start;
            if (output > new_length || inner_length > new_length - output ||
                (inner_length > 0U && !document_edit_read_range(edit, inner_start,
                    inner_length, replacement + output))) {
                free(replacement);
                return DOCUMENT_EDIT_INVALID_OPERATION;
            }
            output += inner_length;
        }
        cursor = node_end;
    }
    size_t region_end;
    if (!base_size_add(region.offset, region.length, &region_end) ||
        cursor > region_end) {
        free(replacement);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    size_t tail_length = region_end - cursor;
    if (output > new_length || tail_length > new_length - output ||
        (tail_length > 0U && !document_edit_read_range(edit, cursor,
            tail_length, replacement + output))) {
        free(replacement);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    output += tail_length;
    if (output != new_length) {
        free(replacement);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    error = document_edit_replace_xml_region(edit, parent, region,
        (base_bytes){replacement, new_length});
    free(replacement);
    return error;
}
