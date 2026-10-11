#include "document_edit.h"

#include <stdio.h>
#include <string.h>

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' data-a='00' data-b='00'>"
    "<rect id='shape' width='4' height='4'/></svg>";

static bool materialize(document_edit *edit, uint8_t output[256], size_t *length)
{
    return document_edit_materialize(edit, output, 256U, length);
}

static bool edit_attribute(document_edit *edit, const char *name, char value)
{
    document_handle root = {0};
    uint8_t next[3] = {'0', (uint8_t)value, 0U};
    return document_edit_root(edit, &root) &&
        document_edit_set_attribute(edit, root,
            (base_bytes){(const uint8_t *)name, strlen(name)},
            (base_bytes){next, 2U}) == DOCUMENT_EDIT_OK;
}

static bool check_recent_entries_and_jump(void)
{
    document_edit_result created = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK) return false;
    document_edit *edit = created.edit;
    uint8_t states[26][256] = {{0}};
    size_t lengths[26] = {0};
    bool okay = materialize(edit, states[0], &lengths[0]);
    for (size_t index = 0U; okay && index < 25U; ++index) {
        okay = edit_attribute(edit, index % 2U == 0U ? "data-a" : "data-b",
            (char)('1' + (int)(index % 8U))) &&
            materialize(edit, states[index + 1U], &lengths[index + 1U]);
    }
    size_t saved_cursor = SIZE_MAX;
    document_edit_history_entry first = {{0}, 0U, false, false};
    document_edit_history_entry newest = {{0}, 0U, false, false};
    okay = okay && document_edit_history_count(edit) == 25U &&
        document_edit_history_cursor(edit) == 25U &&
        document_edit_history_saved_cursor(edit, &saved_cursor) && saved_cursor == 0U &&
        document_edit_history_entry_at(edit, 5U, &first) &&
        document_edit_history_entry_at(edit, 24U, &newest) &&
        strcmp(first.label, "Edit attribute") == 0 &&
        strcmp(newest.label, "Edit attribute") == 0 && newest.is_current &&
        !newest.is_saved;
    okay = okay && document_edit_history_jump(edit, 8U) == DOCUMENT_EDIT_OK;
    uint8_t actual[256] = {0};
    size_t actual_length = 0U;
    okay = okay && document_edit_history_cursor(edit) == 8U &&
        materialize(edit, actual, &actual_length) && actual_length == lengths[8U] &&
        memcmp(actual, states[8U], actual_length) == 0;
    okay = okay && edit_attribute(edit, "data-a", '9') &&
        document_edit_history_count(edit) == 9U &&
        document_edit_history_cursor(edit) == 9U &&
        document_edit_redo(edit) == DOCUMENT_EDIT_INVALID_OPERATION &&
        document_edit_history_jump(edit, 25U) == DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_destroy(edit);
    return okay;
}

static bool check_coalescing_and_eviction(void)
{
    document_edit_result created = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK) return false;
    document_edit *edit = created.edit;
    bool okay = edit_attribute(edit, "data-a", '1') &&
        edit_attribute(edit, "data-a", '2') &&
        edit_attribute(edit, "data-a", '3') &&
        document_edit_history_count(edit) == 1U;
    document_edit_history_entry entry = {{0}, 0U, false, false};
    okay = okay && document_edit_history_entry_at(edit, 0U, &entry) &&
        strcmp(entry.label, "Edit attribute") == 0 && entry.is_current &&
        document_edit_undo(edit) == DOCUMENT_EDIT_OK && !document_edit_is_dirty(edit);
    document_edit_destroy(edit);
    if (!okay) return false;

    document_edit_limits limits = {3U, 4096U};
    created = document_edit_create((base_bytes){source, sizeof(source) - 1U}, limits);
    if (created.error != DOCUMENT_EDIT_OK) return false;
    edit = created.edit;
    for (size_t index = 0U; okay && index < 5U; ++index)
        okay = edit_attribute(edit, index % 2U == 0U ? "data-a" : "data-b",
            (char)('1' + (int)index));
    size_t saved_cursor = 0U;
    okay = okay && document_edit_history_count(edit) == 3U &&
        document_edit_history_cursor(edit) == 3U &&
        !document_edit_history_saved_cursor(edit, &saved_cursor) &&
        document_edit_history_entry_at(edit, 0U, &entry) &&
        strcmp(entry.label, "Edit attribute") == 0 && entry.is_current == false;
    document_edit_destroy(edit);
    return okay;
}

int main(void)
{
    if (!check_recent_entries_and_jump()) {
        (void)fputs("history labels, exact jumps, or branch truncation failed\n", stderr);
        return 1;
    }
    if (!check_coalescing_and_eviction()) {
        (void)fputs("history coalescing or oldest-command eviction failed\n", stderr);
        return 2;
    }
    (void)puts("document edit history: labels, markers, jumps, branches, coalescing, eviction pass");
    return 0;
}
