#include "document_edit_internal.h"

document_edit_error document_edit_replace_xml_region_labeled(document_edit *edit,
    document_handle parent_handle, document_source_span region, base_bytes replacement,
    document_edit_history_action action)
{
    if (edit == NULL || !base_bytes_valid(replacement))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *parent = NULL;
    if (!document_edit_handle_info(edit, parent_handle, &parent))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (region.offset < parent->span.offset ||
        region.offset - parent->span.offset > parent->span.length ||
        region.length > parent->span.length - (region.offset - parent->span.offset))
        return DOCUMENT_EDIT_INVALID_OPERATION;
    if (region.length > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT ||
        replacement.len > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT)
        return DOCUMENT_EDIT_LIMIT;
    if (!document_edit_validate_replace(edit, parent->span.offset, parent->span.length,
            region.offset, region.length, replacement.ptr, replacement.len, true, false))
        return DOCUMENT_EDIT_MALFORMED_XML;
    return document_edit_commit_replace(edit, region.offset, region.length,
        replacement.ptr, replacement.len, parent_handle.slot, false,
        action);
}

document_edit_error document_edit_replace_xml_region(document_edit *edit,
    document_handle parent_handle, document_source_span region, base_bytes replacement)
{
    return document_edit_replace_xml_region_labeled(edit, parent_handle, region,
        replacement, DOCUMENT_EDIT_HISTORY_ACTION_EDIT_OBJECTS);
}

document_edit_error document_edit_replace_source_span(document_edit *edit,
    document_source_span span, base_bytes replacement)
{
    if (edit == NULL || !base_bytes_valid(replacement))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (!document_edit_ensure_index(edit)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (span.offset > edit->snapshot_length ||
        span.length > edit->snapshot_length - span.offset)
        return DOCUMENT_EDIT_INVALID_OPERATION;
    if (span.length > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT ||
        replacement.len > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT)
        return DOCUMENT_EDIT_LIMIT;
    if (!document_edit_validate_replace(edit, 0U, edit->snapshot_length,
            span.offset, span.length, replacement.ptr, replacement.len, false, false))
        return DOCUMENT_EDIT_MALFORMED_XML;
    return document_edit_commit_replace(edit, span.offset, span.length,
        replacement.ptr, replacement.len, 0U, false,
        DOCUMENT_EDIT_HISTORY_ACTION_EDIT_PATH);
}
