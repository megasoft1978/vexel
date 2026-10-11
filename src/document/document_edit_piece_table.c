#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>

#define DOCUMENT_EDIT_MAX_PIECES 1000000U

static void release_pieces(document_edit *edit, document_edit_piece *pieces,
                           size_t count)
{
    for (size_t i = 0U; i < count; ++i)
        document_edit_blob_release(edit, pieces[i].blob);
    free(pieces);
}

void document_edit_blob_retain(document_edit_blob *blob)
{
    if (blob != NULL) ++blob->references;
}

void document_edit_blob_release(document_edit *edit, document_edit_blob *blob)
{
    if (blob == NULL) return;
    if (blob->references > 1U) {
        --blob->references;
        return;
    }
    if (edit != NULL && edit->blob_bytes >= blob->length)
        edit->blob_bytes -= blob->length;
    free(blob->bytes);
    free(blob);
}

document_edit_blob *document_edit_blob_create(document_edit *edit,
    const uint8_t *bytes, size_t length, document_edit_error *out_error)
{
    if (out_error != NULL) *out_error = DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (edit == NULL || (length > 0U && bytes == NULL)) return NULL;
    if (length == 0U) {
        if (out_error != NULL) *out_error = DOCUMENT_EDIT_OK;
        return NULL;
    }
    if (length > edit->limits.history_bytes ||
        edit->blob_bytes > edit->limits.history_bytes - length) {
        if (out_error != NULL) *out_error = DOCUMENT_EDIT_HISTORY_LIMIT;
        return NULL;
    }
    document_edit_blob *blob = calloc(1U, sizeof(*blob));
    if (blob == NULL) {
        if (out_error != NULL) *out_error = DOCUMENT_EDIT_OUT_OF_MEMORY;
        return NULL;
    }
    blob->bytes = malloc(length);
    if (blob->bytes == NULL) {
        free(blob);
        if (out_error != NULL) *out_error = DOCUMENT_EDIT_OUT_OF_MEMORY;
        return NULL;
    }
    memcpy(blob->bytes, bytes, length);
    blob->length = length;
    blob->references = 1U;
    edit->blob_bytes += length;
    if (out_error != NULL) *out_error = DOCUMENT_EDIT_OK;
    return blob;
}

static bool piece_bytes(const document_edit *edit, const document_edit_piece *piece,
                        const uint8_t **out_bytes)
{
    if (edit == NULL || piece == NULL || out_bytes == NULL) return false;
    if (piece->blob == NULL) {
        if (piece->offset > edit->original.len ||
            piece->length > edit->original.len - piece->offset) return false;
        *out_bytes = edit->original.ptr + piece->offset;
    } else {
        if (piece->offset > piece->blob->length ||
            piece->length > piece->blob->length - piece->offset) return false;
        *out_bytes = piece->blob->bytes + piece->offset;
    }
    return true;
}

bool document_edit_read_range(const document_edit *edit, size_t offset,
                              size_t length, uint8_t *output)
{
    if (edit == NULL || (length > 0U && output == NULL) || offset > edit->byte_length ||
        length > edit->byte_length - offset) return false;
    size_t output_offset = 0U;
    size_t cursor = 0U;
    size_t end = offset + length;
    for (size_t i = 0U; i < edit->piece_count && output_offset < length; ++i) {
        const document_edit_piece *piece = &edit->pieces[i];
        size_t piece_end;
        if (!base_size_add(cursor, piece->length, &piece_end)) return false;
        if (piece_end > offset && cursor < end) {
            size_t take_start = cursor < offset ? offset - cursor : 0U;
            size_t take_end = piece_end > end ? end - cursor : piece->length;
            size_t take = take_end - take_start;
            const uint8_t *bytes = NULL;
            if (!piece_bytes(edit, piece, &bytes)) return false;
            memcpy(output + output_offset, bytes + take_start, take);
            output_offset += take;
        }
        cursor = piece_end;
    }
    return output_offset == length;
}

static bool append_piece(document_edit_piece *pieces, size_t capacity, size_t *count,
                         size_t offset, size_t length, document_edit_blob *blob)
{
    if (length == 0U) return true;
    if (*count > 0U) {
        document_edit_piece *last = &pieces[*count - 1U];
        size_t last_end;
        if (last->blob == blob && base_size_add(last->offset, last->length, &last_end) &&
            last_end == offset) {
            return base_size_add(last->length, length, &last->length);
        }
    }
    if (*count >= capacity) return false;
    pieces[*count] = (document_edit_piece){offset, length, blob};
    document_edit_blob_retain(blob);
    ++*count;
    return true;
}

static bool append_range(const document_edit *edit, document_edit_piece *output,
                         size_t capacity, size_t *count, size_t start, size_t end)
{
    if (start > end || end > edit->byte_length) return false;
    size_t cursor = 0U;
    for (size_t i = 0U; i < edit->piece_count && cursor < end; ++i) {
        const document_edit_piece *piece = &edit->pieces[i];
        size_t piece_end;
        if (!base_size_add(cursor, piece->length, &piece_end)) return false;
        if (piece_end > start && cursor < end) {
            size_t within = cursor < start ? start - cursor : 0U;
            size_t take_end = piece_end > end ? end - cursor : piece->length;
            if (!append_piece(output, capacity, count, piece->offset + within,
                              take_end - within, piece->blob)) return false;
        }
        cursor = piece_end;
    }
    return start == end || cursor >= end;
}

static bool piece_table_replace_copy(document_edit *edit, size_t offset,
                                     size_t remove_length, document_edit_blob *insert,
                                     document_edit_piece **out_pieces,
                                     size_t *out_count, size_t *out_capacity,
                                     size_t *out_length)
{
    if (offset > edit->byte_length || remove_length > edit->byte_length - offset)
        return false;
    size_t inserted_length = insert == NULL ? 0U : insert->length;
    size_t new_length = edit->byte_length - remove_length;
    if (!base_size_add(new_length, inserted_length, &new_length)) return false;
    size_t capacity;
    if (!base_size_add(edit->piece_count, 3U, &capacity) ||
        capacity > DOCUMENT_EDIT_MAX_PIECES) return false;
    document_edit_piece *pieces = calloc(capacity, sizeof(*pieces));
    if (pieces == NULL) return false;
    size_t count = 0U;
    bool ok = append_range(edit, pieces, capacity, &count, 0U, offset);
    if (ok && insert != NULL)
        ok = append_piece(pieces, capacity, &count, 0U, inserted_length, insert);
    size_t rest_start = offset + remove_length;
    if (ok) ok = append_range(edit, pieces, capacity, &count, rest_start,
                              edit->byte_length);
    if (!ok || count > DOCUMENT_EDIT_MAX_PIECES) {
        release_pieces(edit, pieces, count);
        return false;
    }
    *out_pieces = pieces;
    *out_count = count;
    *out_capacity = capacity;
    *out_length = new_length;
    return true;
}

bool document_edit_replace_raw(document_edit *edit, size_t offset, size_t remove_length,
                               document_edit_blob *insert)
{
    if (edit == NULL) return false;
    document_edit_piece *pieces = NULL;
    size_t count = 0U, capacity = 0U, length = 0U;
    if (!piece_table_replace_copy(edit, offset, remove_length, insert, &pieces,
                                   &count, &capacity, &length)) return false;
    release_pieces(edit, edit->pieces, edit->piece_count);
    edit->pieces = pieces;
    edit->piece_count = count;
    edit->piece_capacity = capacity;
    edit->byte_length = length;
    return true;
}

static void command_release(document_edit *edit, document_edit_command *command)
{
    document_edit_blob_release(edit, command->before);
    document_edit_blob_release(edit, command->after);
    memset(command, 0, sizeof(*command));
}

static void clear_redo(document_edit *edit)
{
    for (size_t i = edit->command_cursor; i < edit->command_count; ++i)
        command_release(edit, &edit->commands[i]);
    edit->command_count = edit->command_cursor;
}

static bool reserve_commands(document_edit *edit)
{
    if (edit->command_count < edit->command_capacity) return true;
    if (edit->command_count == edit->limits.history_commands &&
        edit->command_capacity == edit->limits.history_commands) return true;
    size_t next = edit->command_capacity == 0U ? 8U : edit->command_capacity * 2U;
    if (next > edit->limits.history_commands) next = edit->limits.history_commands;
    if (next <= edit->command_capacity || next > SIZE_MAX / sizeof(*edit->commands))
        return false;
    document_edit_command *commands = realloc(edit->commands,
                                               next * sizeof(*edit->commands));
    if (commands == NULL) return false;
    memset(commands + edit->command_capacity, 0,
           (next - edit->command_capacity) * sizeof(*commands));
    edit->commands = commands;
    edit->command_capacity = next;
    return true;
}

static void drop_oldest_command(document_edit *edit)
{
    if (edit->command_count == 0U) return;
    command_release(edit, &edit->commands[0]);
    if (edit->command_count > 1U)
        memmove(edit->commands, edit->commands + 1U,
                (edit->command_count - 1U) * sizeof(*edit->commands));
    --edit->command_count;
    if (edit->command_cursor > 0U) --edit->command_cursor;
    memset(&edit->commands[edit->command_count], 0, sizeof(*edit->commands));
}

static bool make_piece_copy(document_edit *edit, size_t offset, size_t length,
                            uint8_t **out_bytes)
{
    uint8_t *bytes = length == 0U ? NULL : malloc(length);
    if (length > 0U && bytes == NULL) return false;
    if (length > 0U && !document_edit_read_range(edit, offset, length, bytes)) {
        free(bytes);
        return false;
    }
    *out_bytes = bytes;
    return true;
}

document_edit_error document_edit_commit_replace(document_edit *edit,
    size_t offset, size_t remove_length, const uint8_t *insert, size_t insert_length,
    uint32_t target_slot, bool coalescible_attribute,
    document_edit_history_action action)
{
    base_bytes label = document_edit_history_action_label(action);
    if (edit == NULL || (insert_length > 0U && insert == NULL) ||
        !base_bytes_valid(label) || label.len == 0U ||
        label.len >= DOCUMENT_EDIT_HISTORY_LABEL_CAP ||
        offset > edit->byte_length || remove_length > edit->byte_length - offset)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    size_t next_length = edit->byte_length - remove_length;
    svg_xml_options xml_options = svg_xml_default_options();
    if (!base_size_add(next_length, insert_length, &next_length) ||
        next_length > xml_options.limits.input_bytes) return DOCUMENT_EDIT_LIMIT;
    if (edit->next_state_id == UINT64_MAX) return DOCUMENT_EDIT_LIMIT;
    if (remove_length == insert_length) {
        uint8_t *existing = NULL;
        if (!make_piece_copy(edit, offset, remove_length, &existing))
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        bool same = remove_length == 0U || memcmp(existing, insert, remove_length) == 0;
        free(existing);
        if (same) return DOCUMENT_EDIT_OK;
    }

    bool coalesce = coalescible_attribute && edit->command_cursor == edit->command_count &&
        edit->command_count > 0U &&
        edit->commands[edit->command_count - 1U].coalescible_attribute &&
        edit->commands[edit->command_count - 1U].target_slot == target_slot &&
        edit->commands[edit->command_count - 1U].offset == offset;
    uint8_t *before_bytes = NULL;
    if (!make_piece_copy(edit, offset, remove_length, &before_bytes))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    document_edit_error blob_error = DOCUMENT_EDIT_OK;
    document_edit_blob *before = coalesce ? NULL : document_edit_blob_create(
        edit, before_bytes, remove_length, &blob_error);
    free(before_bytes);
    if (!coalesce && remove_length > 0U && before == NULL) return blob_error;
    document_edit_blob *after = document_edit_blob_create(edit, insert, insert_length,
                                                          &blob_error);
    if (insert_length > 0U && after == NULL) {
        document_edit_blob_release(edit, before);
        return blob_error;
    }

    document_edit_piece *pieces = NULL;
    size_t piece_count = 0U, piece_capacity = 0U, byte_length = 0U;
    if (!piece_table_replace_copy(edit, offset, remove_length, after, &pieces,
                                  &piece_count, &piece_capacity, &byte_length)) {
        document_edit_blob_release(edit, before);
        document_edit_blob_release(edit, after);
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    }
    if (!coalesce && edit->command_count < edit->limits.history_commands &&
        !reserve_commands(edit)) {
        release_pieces(edit, pieces, piece_count);
        document_edit_blob_release(edit, before);
        document_edit_blob_release(edit, after);
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    }

    clear_redo(edit);
    uint64_t new_state = edit->next_state_id++;
    if (coalesce) {
        document_edit_command *previous = &edit->commands[edit->command_count - 1U];
        document_edit_blob_release(edit, previous->after);
        previous->after = after;
        previous->after_state = new_state;
        memcpy(previous->label, label.ptr, label.len);
        previous->label[label.len] = '\0';
    } else {
        if (edit->command_count == edit->limits.history_commands) drop_oldest_command(edit);
        document_edit_command command = {.offset = offset, .before = before,
            .after = after, .target_slot = target_slot,
            .coalescible_attribute = coalescible_attribute,
            .before_state = edit->state_id, .after_state = new_state};
        memcpy(command.label, label.ptr, label.len);
        command.label[label.len] = '\0';
        edit->commands[edit->command_count++] = command;
        edit->command_cursor = edit->command_count;
    }
    release_pieces(edit, edit->pieces, edit->piece_count);
    edit->pieces = pieces;
    edit->piece_count = piece_count;
    edit->piece_capacity = piece_capacity;
    edit->byte_length = byte_length;
    edit->state_id = new_state;
    document_edit_invalidate_index(edit);
    return DOCUMENT_EDIT_OK;
}

document_edit_error document_edit_undo(document_edit *edit)
{
    if (edit == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (edit->command_cursor == 0U) return DOCUMENT_EDIT_INVALID_OPERATION;
    document_edit_command *command = &edit->commands[edit->command_cursor - 1U];
    size_t after_length = command->after == NULL ? 0U : command->after->length;
    size_t before_length = command->before == NULL ? 0U : command->before->length;
    if (!document_edit_validate_replace(edit, 0U, edit->byte_length,
            command->offset, after_length,
            command->before == NULL ? NULL : command->before->bytes,
            before_length, false, false)) return DOCUMENT_EDIT_MALFORMED_XML;
    if (!document_edit_replace_raw(edit, command->offset, after_length, command->before))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    --edit->command_cursor;
    edit->state_id = command->before_state;
    document_edit_invalidate_index(edit);
    return DOCUMENT_EDIT_OK;
}

document_edit_error document_edit_redo(document_edit *edit)
{
    if (edit == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (edit->command_cursor >= edit->command_count) return DOCUMENT_EDIT_INVALID_OPERATION;
    document_edit_command *command = &edit->commands[edit->command_cursor];
    size_t before_length = command->before == NULL ? 0U : command->before->length;
    size_t after_length = command->after == NULL ? 0U : command->after->length;
    if (!document_edit_validate_replace(edit, 0U, edit->byte_length,
            command->offset, before_length,
            command->after == NULL ? NULL : command->after->bytes,
            after_length, false, false)) return DOCUMENT_EDIT_MALFORMED_XML;
    if (!document_edit_replace_raw(edit, command->offset, before_length, command->after))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    ++edit->command_cursor;
    edit->state_id = command->after_state;
    document_edit_invalidate_index(edit);
    return DOCUMENT_EDIT_OK;
}

size_t document_edit_byte_length(const document_edit *edit)
{
    return edit == NULL ? 0U : edit->byte_length;
}

bool document_edit_materialize(const document_edit *edit, uint8_t *buffer,
                               size_t capacity, size_t *out_length)
{
    if (edit == NULL || out_length == NULL ||
        (capacity > 0U && buffer == NULL)) return false;
    *out_length = edit->byte_length;
    if (capacity < edit->byte_length) return false;
    return document_edit_read_range(edit, 0U, edit->byte_length, buffer);
}

bool document_edit_is_dirty(const document_edit *edit)
{
    return edit != NULL && edit->state_id != edit->saved_state_id;
}

size_t document_edit_history_count(const document_edit *edit)
{
    return edit == NULL ? 0U : edit->command_count;
}

size_t document_edit_history_cursor(const document_edit *edit)
{
    return edit == NULL ? 0U : edit->command_cursor;
}

void document_edit_piece_table_destroy(document_edit *edit)
{
    if (edit == NULL) return;
    release_pieces(edit, edit->pieces, edit->piece_count);
    edit->pieces = NULL;
    edit->piece_count = 0U;
    for (size_t i = 0U; i < edit->command_count; ++i)
        command_release(edit, &edit->commands[i]);
    free(edit->commands);
    edit->commands = NULL;
    edit->command_count = 0U;
}
