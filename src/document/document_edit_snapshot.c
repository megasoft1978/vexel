#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>

struct document_edit_snapshot {
    document_edit *owner;
    base_bytes original;
    document_edit_piece *pieces;
    size_t piece_count;
    size_t byte_length;
};

static bool snapshot_piece_valid(base_bytes original,
    const document_edit_piece *piece)
{
    if (piece == NULL || piece->length == 0U) return false;
    if (piece->blob == NULL)
        return piece->offset <= original.len &&
            piece->length <= original.len - piece->offset;
    return piece->offset <= piece->blob->length &&
        piece->length <= piece->blob->length - piece->offset;
}

document_edit_error document_edit_snapshot_create(document_edit *edit,
    document_edit_snapshot **out_snapshot)
{
    if (out_snapshot != NULL) *out_snapshot = NULL;
    if (edit == NULL || out_snapshot == NULL ||
        !base_bytes_valid(edit->original) ||
        (edit->piece_count > 0U && edit->pieces == NULL))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    size_t piece_bytes = 0U;
    if (!base_size_mul(edit->piece_count, sizeof(document_edit_piece), &piece_bytes))
        return DOCUMENT_EDIT_LIMIT;
    document_edit_snapshot *snapshot = calloc(1U, sizeof(*snapshot));
    if (snapshot == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    if (piece_bytes > 0U) {
        snapshot->pieces = malloc(piece_bytes);
        if (snapshot->pieces == NULL) {
            free(snapshot);
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        }
        memcpy(snapshot->pieces, edit->pieces, piece_bytes);
    }
    snapshot->owner = edit;
    snapshot->original = edit->original;
    snapshot->piece_count = edit->piece_count;
    snapshot->byte_length = edit->byte_length;
    size_t total = 0U;
    for (size_t index = 0U; index < snapshot->piece_count; ++index) {
        document_edit_piece *piece = &snapshot->pieces[index];
        if (!snapshot_piece_valid(snapshot->original, piece) ||
            !base_size_add(total, piece->length, &total)) {
            for (size_t retained = 0U; retained < index; ++retained)
                document_edit_blob_release(edit,
                    snapshot->pieces[retained].blob);
            free(snapshot->pieces);
            free(snapshot);
            return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        document_edit_blob_retain(piece->blob);
    }
    if (total != snapshot->byte_length) {
        for (size_t index = 0U; index < snapshot->piece_count; ++index)
            document_edit_blob_release(edit,
                snapshot->pieces[index].blob);
        free(snapshot->pieces);
        free(snapshot);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    *out_snapshot = snapshot;
    return DOCUMENT_EDIT_OK;
}

size_t document_edit_snapshot_byte_length(const document_edit_snapshot *snapshot)
{
    return snapshot == NULL ? 0U : snapshot->byte_length;
}

bool document_edit_snapshot_materialize(const document_edit_snapshot *snapshot,
    uint8_t *buffer, size_t capacity, size_t *out_length)
{
    if (out_length != NULL) *out_length = 0U;
    if (snapshot == NULL || out_length == NULL ||
        (buffer == NULL && capacity != 0U) || capacity < snapshot->byte_length)
        return false;
    size_t output = 0U;
    for (size_t index = 0U; index < snapshot->piece_count; ++index) {
        const document_edit_piece *piece = &snapshot->pieces[index];
        if (!snapshot_piece_valid(snapshot->original, piece) ||
            piece->length > snapshot->byte_length - output) return false;
        const uint8_t *source = piece->blob == NULL ? snapshot->original.ptr :
            piece->blob->bytes;
        memcpy(buffer + output, source + piece->offset, piece->length);
        output += piece->length;
    }
    if (output != snapshot->byte_length) return false;
    *out_length = output;
    return true;
}

void document_edit_snapshot_destroy(document_edit_snapshot *snapshot)
{
    if (snapshot == NULL) return;
    for (size_t index = 0U; index < snapshot->piece_count; ++index)
        document_edit_blob_release(snapshot->owner,
            snapshot->pieces[index].blob);
    free(snapshot->pieces);
    free(snapshot);
}
