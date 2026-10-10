#include "viewer_editor_internal.h"

#include "svg/xml/xml_tokenizer.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define EDITOR_CLIPBOARD_FRAGMENT_LIMIT (64U * 1024U)
#define EDITOR_CLIPBOARD_ID_ATTEMPTS 65536U
#define EDITOR_CLIPBOARD_ID_SET_LIMIT 131072U

enum {
    EDITOR_CLIPBOARD_ID_DOCUMENT = 1U << 0U,
    EDITOR_CLIPBOARD_ID_FRAGMENT = 1U << 1U,
    EDITOR_CLIPBOARD_ID_GENERATED = 1U << 2U
};

typedef struct clipboard_id_rename {
    svg_xml_span source_value;
    const uint8_t *replacement;
    size_t replacement_length;
} clipboard_id_rename;

typedef struct clipboard_patch {
    size_t offset;
    size_t length;
    const uint8_t *replacement;
    size_t replacement_length;
    bool prepend_hash;
} clipboard_patch;

typedef struct clipboard_id_slot {
    uint64_t hash;
    base_bytes value;
    uint8_t flags;
    bool occupied;
} clipboard_id_slot;

typedef struct clipboard_id_set {
    clipboard_id_slot *slots;
    size_t capacity;
} clipboard_id_set;

static bool clipboard_name_is(base_bytes source, svg_xml_span name,
                              const char *literal)
{
    size_t length = strlen(literal);
    return name.offset <= source.len && name.length == length &&
        length <= source.len - name.offset &&
        memcmp(source.ptr + name.offset, literal, length) == 0;
}

static bool clipboard_id_attribute(base_bytes source,
                                   const svg_xml_token *token)
{
    return token->kind == SVG_XML_TOKEN_ATTRIBUTE &&
        token->prefix.length == 0U &&
        clipboard_name_is(source, token->local_name, "id");
}

static bool clipboard_id_character(uint8_t byte, bool first)
{
    bool letter = (byte >= 'a' && byte <= 'z') ||
        (byte >= 'A' && byte <= 'Z') || byte == '_';
    if (first) return letter;
    return letter || (byte >= '0' && byte <= '9') || byte == '-' ||
        byte == '.' || byte == ':';
}

static bool clipboard_plain_id(base_bytes fragment, svg_xml_span value)
{
    if (value.length == 0U || value.offset > fragment.len ||
        value.length > fragment.len - value.offset) return false;
    for (size_t index = 0U; index < value.length; ++index) {
        if (!clipboard_id_character(fragment.ptr[value.offset + index],
                index == 0U)) return false;
    }
    return true;
}

static uint64_t clipboard_id_hash(base_bytes id)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    for (size_t index = 0U; index < id.len; ++index) {
        hash ^= id.ptr[index];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static clipboard_id_slot *clipboard_id_set_find(clipboard_id_set *set,
                                                 base_bytes id)
{
    if (set == NULL || set->slots == NULL || set->capacity == 0U || id.len == 0U)
        return NULL;
    uint64_t hash = clipboard_id_hash(id);
    size_t slot = (size_t)hash & (set->capacity - 1U);
    for (size_t probes = 0U; probes < set->capacity; ++probes) {
        clipboard_id_slot *candidate = &set->slots[slot];
        if (!candidate->occupied) return NULL;
        if (candidate->hash == hash && candidate->value.len == id.len &&
            memcmp(candidate->value.ptr, id.ptr, id.len) == 0) return candidate;
        slot = (slot + 1U) & (set->capacity - 1U);
    }
    return NULL;
}

static bool clipboard_id_set_insert(clipboard_id_set *set, base_bytes id,
    uint8_t flags, uint8_t *out_previous_flags)
{
    if (set == NULL || set->slots == NULL || id.ptr == NULL || id.len == 0U ||
        out_previous_flags == NULL) return false;
    uint64_t hash = clipboard_id_hash(id);
    size_t slot = (size_t)hash & (set->capacity - 1U);
    for (size_t probes = 0U; probes < set->capacity; ++probes) {
        clipboard_id_slot *entry = &set->slots[slot];
        if (!entry->occupied) {
            *entry = (clipboard_id_slot){hash, id, flags, true};
            *out_previous_flags = 0U;
            return true;
        }
        if (entry->hash == hash && entry->value.len == id.len &&
            memcmp(entry->value.ptr, id.ptr, id.len) == 0) {
            *out_previous_flags = entry->flags;
            entry->flags |= flags;
            return true;
        }
        slot = (slot + 1U) & (set->capacity - 1U);
    }
    return false;
}

static bool clipboard_document_id_count(const document_model *model,
                                        size_t *out_count)
{
    if (model == NULL || out_count == NULL) return false;
    *out_count = 0U;
    size_t node_count = document_model_node_count(model);
    for (size_t node_index = 0U; node_index < node_count; ++node_index) {
        document_handle node = {0};
        if (!document_model_node_at(model, node_index, &node)) continue;
        size_t attribute_count = document_model_attribute_count(model, node);
        for (size_t attribute_index = 0U; attribute_index < attribute_count;
             ++attribute_index) {
            base_bytes name = {0}, value = {0};
            if (!document_model_attribute_at(model, node, attribute_index,
                    &name, &value)) return false;
            if (name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
            if (value.len == 0U || *out_count >= EDITOR_CLIPBOARD_ID_SET_LIMIT)
                return false;
            ++*out_count;
        }
    }
    return true;
}

static bool clipboard_id_set_create(const document_model *model,
    base_bytes fragment, const svg_xml_token *tokens, size_t token_count,
    size_t fragment_id_count, clipboard_id_set *out_set)
{
    if (model == NULL || out_set == NULL) return false;
    *out_set = (clipboard_id_set){0};
    size_t document_id_count = 0U;
    if (!clipboard_document_id_count(model, &document_id_count)) return false;
    size_t total_ids = 0U, target_capacity = 0U;
    if (!base_size_add(document_id_count, fragment_id_count, &total_ids) ||
        total_ids > EDITOR_CLIPBOARD_ID_SET_LIMIT ||
        !base_size_mul(total_ids, 2U, &target_capacity)) return false;
    size_t capacity = 16U;
    while (capacity < target_capacity) {
        if (capacity > SIZE_MAX / 2U) return false;
        capacity *= 2U;
    }
    size_t bytes = 0U;
    if (!base_size_mul(capacity, sizeof(clipboard_id_slot), &bytes)) return false;
    out_set->slots = calloc(1U, bytes);
    if (out_set->slots == NULL) return false;
    out_set->capacity = capacity;

    size_t node_count = document_model_node_count(model);
    for (size_t node_index = 0U; node_index < node_count; ++node_index) {
        document_handle node = {0};
        if (!document_model_node_at(model, node_index, &node)) continue;
        size_t attribute_count = document_model_attribute_count(model, node);
        for (size_t attribute_index = 0U; attribute_index < attribute_count;
             ++attribute_index) {
            base_bytes name = {0}, value = {0};
            uint8_t previous = 0U;
            if (!document_model_attribute_at(model, node, attribute_index,
                    &name, &value)) goto fail;
            if (name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
            if (!clipboard_id_set_insert(out_set, value,
                    EDITOR_CLIPBOARD_ID_DOCUMENT, &previous)) goto fail;
        }
    }
    for (size_t index = 0U; index < token_count; ++index) {
        if (!clipboard_id_attribute(fragment, &tokens[index])) continue;
        svg_xml_span value_span = tokens[index].value;
        if (value_span.offset > fragment.len ||
            value_span.length > fragment.len - value_span.offset) goto fail;
        base_bytes value = {fragment.ptr + value_span.offset, value_span.length};
        uint8_t previous = 0U;
        if (!clipboard_id_set_insert(out_set, value,
                EDITOR_CLIPBOARD_ID_FRAGMENT, &previous) ||
            (previous & EDITOR_CLIPBOARD_ID_FRAGMENT) != 0U) goto fail;
    }
    return true;
fail:
    free(out_set->slots);
    *out_set = (clipboard_id_set){0};
    return false;
}

static bool clipboard_patch_add(clipboard_patch *patches, size_t capacity,
    size_t *count, size_t offset, size_t length, const uint8_t *replacement,
    size_t replacement_length, bool prepend_hash)
{
    if (patches == NULL || count == NULL || *count >= capacity || length == 0U ||
        replacement == NULL || replacement_length == 0U) return false;
    patches[*count] = (clipboard_patch){offset, length, replacement,
        replacement_length, prepend_hash};
    ++*count;
    return true;
}

static int clipboard_patch_compare(const void *left, const void *right)
{
    const clipboard_patch *a = left;
    const clipboard_patch *b = right;
    if (a->offset < b->offset) return -1;
    if (a->offset > b->offset) return 1;
    return 0;
}

static bool clipboard_url_reference(const uint8_t *value, size_t length,
                                    size_t hash_offset)
{
    size_t cursor = hash_offset;
    while (cursor > 0U && (value[cursor - 1U] == ' ' ||
            value[cursor - 1U] == '\t' || value[cursor - 1U] == '\r' ||
            value[cursor - 1U] == '\n' || value[cursor - 1U] == '\'' ||
            value[cursor - 1U] == '"')) --cursor;
    if (cursor == 0U || value[cursor - 1U] != '(') return false;
    --cursor;
    while (cursor > 0U && (value[cursor - 1U] == ' ' ||
            value[cursor - 1U] == '\t' || value[cursor - 1U] == '\r' ||
            value[cursor - 1U] == '\n')) --cursor;
    static const uint8_t url[] = "url";
    return cursor >= sizeof(url) - 1U &&
        memcmp(value + cursor - (sizeof(url) - 1U), url,
            sizeof(url) - 1U) == 0 &&
        (cursor == sizeof(url) - 1U ||
         !clipboard_id_character(value[cursor - (sizeof(url))], false)) &&
        hash_offset < length;
}

static bool clipboard_make_ids_unique(viewer_editor *editor, base_bytes fragment,
    uint8_t **out_owned, size_t *out_length)
{
    if (editor == NULL || editor->edit == NULL || !base_bytes_valid(fragment) ||
        out_owned == NULL || out_length == NULL) return false;
    *out_owned = NULL;
    *out_length = fragment.len;
    size_t token_capacity = fragment.len / 2U;
    if (!base_size_add(token_capacity, 2U, &token_capacity)) return false;
    size_t token_bytes = 0U;
    if (!base_size_mul(token_capacity, sizeof(svg_xml_token), &token_bytes))
        return false;
    svg_xml_token *tokens = calloc(1U, token_bytes);
    if (tokens == NULL) return false;
    svg_xml_result tokenized = svg_xml_tokenize(fragment, svg_xml_default_options(),
        tokens, token_capacity);
    if (tokenized.diagnostic.category != SVG_XML_ERROR_NONE) {
        free(tokens);
        return false;
    }
    size_t id_count = 0U;
    for (size_t index = 0U; index < tokenized.token_count; ++index) {
        if (clipboard_id_attribute(fragment, &tokens[index])) ++id_count;
    }
    if (id_count == 0U) {
        free(tokens);
        return true;
    }
    size_t rename_bytes = 0U, suffix_bytes = 0U;
    if (!base_size_mul(id_count, sizeof(clipboard_id_rename), &rename_bytes) ||
        !base_size_mul(id_count, 12U, &suffix_bytes) ||
        !base_size_add(fragment.len, suffix_bytes, &suffix_bytes)) {
        free(tokens);
        return false;
    }
    clipboard_id_rename *renames = calloc(1U, rename_bytes);
    uint8_t *rename_storage = malloc(suffix_bytes);
    clipboard_id_set id_set = {0};
    size_t patch_bytes = 0U;
    if (!base_size_mul(fragment.len, sizeof(clipboard_patch), &patch_bytes)) {
        free(tokens);
        free(renames);
        free(rename_storage);
        return false;
    }
    clipboard_patch *patches = malloc(patch_bytes);
    if (renames == NULL || rename_storage == NULL || patches == NULL) {
        free(tokens);
        free(renames);
        free(rename_storage);
        free(patches);
        return false;
    }
    const document_model *model = document_edit_model(editor->edit);
    if (!clipboard_id_set_create(model, fragment, tokens,
            tokenized.token_count, id_count, &id_set)) {
        free(tokens);
        free(renames);
        free(rename_storage);
        free(patches);
        return false;
    }

    size_t rename_count = 0U, storage_used = 0U, patch_count = 0U;
    bool okay = true;
    for (size_t index = 0U; index < tokenized.token_count && okay; ++index) {
        if (!clipboard_id_attribute(fragment, &tokens[index])) continue;
        svg_xml_span id = tokens[index].value;
        base_bytes id_bytes = {fragment.ptr + id.offset, id.length};
        clipboard_id_slot *original = clipboard_id_set_find(&id_set, id_bytes);
        if (original == NULL ||
            (original->flags & EDITOR_CLIPBOARD_ID_DOCUMENT) == 0U) continue;
        if (!clipboard_plain_id(fragment, id)) { okay = false; break; }

        size_t candidate_length = 0U;
        bool found = false;
        for (size_t attempt = 1U; attempt <= EDITOR_CLIPBOARD_ID_ATTEMPTS; ++attempt) {
            char suffix[16];
            int suffix_length = snprintf(suffix, sizeof(suffix), "-copy-%zu", attempt);
            if (suffix_length < 0 || (size_t)suffix_length >= sizeof(suffix) ||
                id.length > SIZE_MAX - (size_t)suffix_length) break;
            candidate_length = id.length + (size_t)suffix_length;
            if (candidate_length > suffix_bytes - storage_used) break;
            uint8_t *candidate = rename_storage + storage_used;
            memcpy(candidate, id_bytes.ptr, id.length);
            memcpy(candidate + id.length, suffix, (size_t)suffix_length);
            base_bytes candidate_bytes = {candidate, candidate_length};
            uint8_t previous = 0U;
            if (clipboard_id_set_find(&id_set, candidate_bytes) == NULL &&
                clipboard_id_set_insert(&id_set, candidate_bytes,
                    EDITOR_CLIPBOARD_ID_GENERATED, &previous)) {
                found = true;
                break;
            }
        }
        if (!found) { okay = false; break; }
        renames[rename_count++] = (clipboard_id_rename){id,
            rename_storage + storage_used, candidate_length};
        storage_used += candidate_length;
        if (!clipboard_patch_add(patches, fragment.len, &patch_count,
                id.offset, id.length, renames[rename_count - 1U].replacement,
                candidate_length, false)) okay = false;
    }

    for (size_t token_index = 0U;
         token_index < tokenized.token_count && okay; ++token_index) {
        const svg_xml_token *token = &tokens[token_index];
        if (token->kind != SVG_XML_TOKEN_ATTRIBUTE ||
            clipboard_id_attribute(fragment, token) || token->value.offset > fragment.len ||
            token->value.length > fragment.len - token->value.offset) continue;
        const uint8_t *value = fragment.ptr + token->value.offset;
        for (size_t rename_index = 0U; rename_index < rename_count && okay;
             ++rename_index) {
            const clipboard_id_rename *rename = &renames[rename_index];
            for (size_t position = 0U; position < token->value.length; ++position) {
                if (value[position] != '#') continue;
                size_t id_length = rename->source_value.length;
                if (id_length > token->value.length - position - 1U ||
                    memcmp(value + position + 1U,
                        fragment.ptr + rename->source_value.offset, id_length) != 0)
                    continue;
                size_t end = position + 1U + id_length;
                if (end < token->value.length &&
                    clipboard_id_character(value[end], false)) continue;
                bool href = clipboard_name_is(fragment, token->local_name, "href") &&
                    position == 0U && end == token->value.length;
                if (!href && !clipboard_url_reference(value,
                        token->value.length, position)) continue;
                if (!clipboard_patch_add(patches, fragment.len, &patch_count,
                        token->value.offset + position, id_length + 1U,
                        rename->replacement, rename->replacement_length, true)) {
                    okay = false;
                    break;
                }
                position = end;
            }
        }
    }

    uint8_t *rewritten = NULL;
    size_t rewritten_length = fragment.len;
    if (okay && patch_count > 0U) {
        qsort(patches, patch_count, sizeof(*patches), clipboard_patch_compare);
        size_t cursor = 0U;
        rewritten_length = 0U;
        for (size_t index = 0U; index < patch_count; ++index) {
            const clipboard_patch *patch = &patches[index];
            size_t replacement_length = patch->replacement_length;
            if (patch->prepend_hash &&
                !base_size_add(replacement_length, 1U, &replacement_length)) {
                okay = false;
                break;
            }
            if (patch->offset < cursor || patch->offset > fragment.len ||
                patch->length > fragment.len - patch->offset ||
                !base_size_add(rewritten_length, patch->offset - cursor,
                    &rewritten_length) ||
                !base_size_add(rewritten_length, replacement_length,
                    &rewritten_length)) {
                okay = false;
                break;
            }
            cursor = patch->offset + patch->length;
        }
        if (okay && !base_size_add(rewritten_length, fragment.len - cursor,
                &rewritten_length)) okay = false;
        if (okay && rewritten_length > EDITOR_CLIPBOARD_FRAGMENT_LIMIT) okay = false;
        if (okay) rewritten = malloc(rewritten_length);
        if (okay && rewritten == NULL) okay = false;
        if (okay) {
            size_t source_cursor = 0U, output_cursor = 0U;
            for (size_t index = 0U; index < patch_count; ++index) {
                const clipboard_patch *patch = &patches[index];
                size_t prefix_length = patch->offset - source_cursor;
                memcpy(rewritten + output_cursor, fragment.ptr + source_cursor,
                    prefix_length);
                output_cursor += prefix_length;
                source_cursor = patch->offset + patch->length;
                if (patch->prepend_hash) rewritten[output_cursor++] = '#';
                memcpy(rewritten + output_cursor, patch->replacement,
                    patch->replacement_length);
                output_cursor += patch->replacement_length;
            }
            memcpy(rewritten + output_cursor, fragment.ptr + source_cursor,
                fragment.len - source_cursor);
            *out_owned = rewritten;
            *out_length = rewritten_length;
        }
    }

    if (!okay) free(rewritten);
    free(tokens);
    free(renames);
    free(rename_storage);
    free(patches);
    free(id_set.slots);
    return okay;
}

static void sort_spans(document_source_span *spans, size_t count)
{
    for (size_t index = 1U; index < count; ++index) {
        document_source_span value = spans[index];
        size_t cursor = index;
        while (cursor > 0U && (spans[cursor - 1U].offset > value.offset ||
            (spans[cursor - 1U].offset == value.offset &&
             spans[cursor - 1U].length < value.length))) {
            spans[cursor] = spans[cursor - 1U];
            --cursor;
        }
        spans[cursor] = value;
    }
}

static bool append_bytes(uint8_t *buffer, size_t capacity, size_t *used,
    const uint8_t *bytes, size_t length)
{
    if (buffer == NULL || used == NULL || (length > 0U && bytes == NULL) ||
        *used > capacity || length > capacity - *used) return false;
    if (length > 0U) memcpy(buffer + *used, bytes, length);
    *used += length;
    return true;
}

bool viewer_editor_copy_selection(viewer_editor *editor, uint8_t *buffer,
    size_t capacity, size_t *out_length)
{
    if (editor == NULL || editor->edit == NULL || buffer == NULL ||
        out_length == NULL || capacity == 0U) return false;
    *out_length = 0U;
    size_t requested = editor->canvas_selection_count;
    if (requested == 0U) requested = 1U;
    if (requested > EDITOR_CANVAS_SELECTION_CAP) return false;
    document_source_span spans[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U;
    for (size_t index = 0U; index < requested; ++index) {
        size_t offset = editor->canvas_selection_count == 0U ?
            editor->selected_offset : editor->canvas_selection_offsets[index];
        document_handle handle = {0};
        document_source_span span;
        if (!document_edit_element_at_offset(editor->edit, offset, &handle) ||
            !document_edit_source_span(editor->edit, handle, &span) ||
            span.length == 0U) return false;
        spans[count++] = span;
    }
    sort_spans(spans, count);
    size_t visible_count = 0U;
    size_t covered_end = 0U;
    for (size_t index = 0U; index < count; ++index) {
        if (visible_count > 0U && spans[index].offset < covered_end) continue;
        spans[visible_count++] = spans[index];
        if (spans[index].length > SIZE_MAX - spans[index].offset) return false;
        covered_end = spans[index].offset + spans[index].length;
    }
    if (visible_count == 0U) return false;
    if (visible_count == 1U) {
        base_bytes source;
        if (!document_edit_span_bytes(editor->edit, spans[0], &source) ||
            source.len > capacity || source.len > EDITOR_CLIPBOARD_FRAGMENT_LIMIT)
            return false;
        memcpy(buffer, source.ptr, source.len);
        *out_length = source.len;
        return true;
    }
    static const uint8_t prefix[] =
        "<g xmlns='http://www.w3.org/2000/svg'>";
    static const uint8_t suffix[] = "</g>";
    size_t used = 0U;
    if (!append_bytes(buffer, capacity, &used, prefix, sizeof(prefix) - 1U))
        return false;
    for (size_t index = 0U; index < visible_count; ++index) {
        base_bytes source;
        if (!document_edit_span_bytes(editor->edit, spans[index], &source) ||
            source.len > EDITOR_CLIPBOARD_FRAGMENT_LIMIT - used ||
            !append_bytes(buffer, capacity, &used, source.ptr, source.len)) return false;
    }
    if (!append_bytes(buffer, capacity, &used, suffix, sizeof(suffix) - 1U))
        return false;
    *out_length = used;
    return true;
}

uint32_t viewer_editor_cut_selection(viewer_editor *editor)
{
    if (editor == NULL) return 0U;
    if (editor->canvas_selection_count > 1U)
        return editor_canvas_multi_delete(editor);
    return editor_apply_button(editor, EDITOR_BUTTON_DELETE);
}

static uint32_t viewer_editor_paste_svg_impl(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place,
    bool make_ids_unique)
{
    if (editor == NULL || editor->edit == NULL || bytes == NULL || length == 0U ||
        length > EDITOR_CLIPBOARD_FRAGMENT_LIMIT || memchr(bytes, 0, length) != NULL) {
        if (editor != NULL) editor_status_ok(editor,
            "Clipboard SVG is empty or exceeds the 64 KiB limit");
        return VIEWER_EDITOR_HANDLED;
    }
    document_handle parent = {0}, root = {0}, inserted = {0};
    if (!document_edit_root(editor->edit, &root)) return VIEWER_EDITOR_HANDLED;
    parent = root;
    base_bytes tag = {0};
    if (document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 1U && tag.ptr[0] == 'g') parent = editor->selected;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    uint8_t *unique_fragment = NULL;
    size_t unique_length = length;
    if (make_ids_unique && !clipboard_make_ids_unique(editor,
            (base_bytes){bytes, length},
            &unique_fragment, &unique_length)) {
        editor_status_ok(editor,
            "Clipboard SVG IDs could not be made unique within the paste limit");
        return VIEWER_EDITOR_HANDLED;
    }
    const uint8_t *paste_bytes = unique_fragment == NULL ? bytes : unique_fragment;
    document_edit_error error = document_edit_insert_xml_fragment(editor->edit,
        parent, (base_bytes){paste_bytes, unique_length}, &inserted);
    free(unique_fragment);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0});
    editor->canvas_selection_count = 0U;
    editor->canvas_selection_valid = false;
    editor_status_ok(editor, paste_in_place ?
        "SVG pasted in place · preview updating" : "SVG pasted · preview updating");
    return editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset, old_id,
        inserted);
}

uint32_t viewer_editor_paste_svg(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place)
{
    return viewer_editor_paste_svg_impl(editor, bytes, length, paste_in_place,
        false);
}

uint32_t viewer_editor_paste_svg_from_clipboard(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place)
{
    return viewer_editor_paste_svg_impl(editor, bytes, length, paste_in_place,
        true);
}
