#include "document_model.h"
#include "document_view.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

static bool name_is(const document_model *model, document_handle handle, const char *name)
{
    base_bytes actual;
    size_t length = strlen(name);
    return document_model_tag_name(model, handle, &actual) && actual.len == length &&
           memcmp(actual.ptr, name, length) == 0;
}

static bool token_storage_limit_case(void)
{
    const size_t token_limit = (64U * 1024U * 1024U) / sizeof(svg_xml_token);
    if (token_limit >= 1000000U) return false;
    size_t children = token_limit;
    size_t child_bytes = 0U, total_bytes = 0U;
    if (!base_size_mul(children, 4U, &child_bytes) ||
        !base_size_add(5U, child_bytes, &total_bytes) ||
        !base_size_add(total_bytes, 6U, &total_bytes)) return false;
    uint8_t *source = malloc(total_bytes);
    if (source == NULL) return false;
    size_t offset = 0U;
    memcpy(source + offset, "<svg>", 5U); offset += 5U;
    for (size_t i = 0U; i < children; ++i) {
        memcpy(source + offset, "<a/>", 4U); offset += 4U;
    }
    memcpy(source + offset, "</svg>", 6U); offset += 6U;
    document_model_result result = document_model_parse((base_bytes){source, offset});
    bool passed = result.model == NULL &&
        result.error == DOCUMENT_MODEL_RESOURCE_EXHAUSTED &&
        result.parse_diagnostic.category == SVG_XML_ERROR_OUTPUT_EXHAUSTED;
    document_model_destroy(result.model);
    free(source);
    return passed;
}

int main(void)
{
    static const uint8_t source[] =
        "<?xml version='1.0'?>\r\n"
        "<svg xmlns='urn:svg' xmlns:i='urn:inkscape'>"
        "<g i:label='keep'><rect id='one'/><i:unknown><path d='M0 0'/></i:unknown></g>"
        "<circle/>"
        "</svg>\n";
    uint8_t original[sizeof(source)];
    memcpy(original, source, sizeof(source));
    base_bytes bytes = {source, sizeof(source) - 1U};
    document_model_result first_result = document_model_parse(bytes);
    document_model_result second_result = document_model_parse(bytes);
    CHECK(first_result.error == DOCUMENT_MODEL_OK && first_result.model != NULL);
    CHECK(second_result.error == DOCUMENT_MODEL_OK && second_result.model != NULL);
    document_model *first = first_result.model;
    document_model *second = second_result.model;
    CHECK(document_model_node_count(first) == 6U);
    CHECK(document_model_live_node_count(first) == 6U);

    document_handle root, group, rectangle, unknown, circle, path;
    document_handle second_root;
    CHECK(document_model_root(first, &root) && name_is(first, root, "svg"));
    CHECK(document_model_root(second, &second_root));
    CHECK(root.document_id != second_root.document_id);
    CHECK(document_model_first_child(first, root, &group) && name_is(first, group, "g"));
    CHECK(document_model_first_child(first, group, &rectangle) && name_is(first, rectangle, "rect"));
    CHECK(document_model_next_sibling(first, rectangle, &unknown));
    CHECK(name_is(first, unknown, "i:unknown"));
    CHECK(document_model_first_child(first, unknown, &path) && name_is(first, path, "path"));
    CHECK(document_model_next_sibling(first, group, &circle) && name_is(first, circle, "circle"));
    document_handle stable_again;
    CHECK(document_model_first_child(first, root, &stable_again));
    CHECK(stable_again.document_id == group.document_id && stable_again.slot == group.slot &&
          stable_again.generation == group.generation);

    CHECK(document_view_attribute_count(first, group) == 1U);
    base_bytes attribute_name, attribute_value;
    CHECK(document_view_attribute_at(first, group, 0U, &attribute_name, &attribute_value));
    CHECK(attribute_name.len == 7U && memcmp(attribute_name.ptr, "i:label", 7U) == 0);
    CHECK(attribute_value.len == 4U && memcmp(attribute_value.ptr, "keep", 4U) == 0);
    document_view_row rows[2];
    size_t row_count = 0U, next_slot = 0U;
    CHECK(document_view_range(first, 0U, document_model_node_count(first), rows, 2U,
                              &row_count, &next_slot));
    CHECK(row_count == 2U && next_slot == 2U && rows[0].handle.slot == 0U &&
          rows[1].depth == 1U);

    document_source_span span;
    CHECK(document_model_source_span(first, group, &span));
    static const char expected_group[] =
        "<g i:label='keep'><rect id='one'/><i:unknown><path d='M0 0'/></i:unknown></g>";
    CHECK(span.length == sizeof(expected_group) - 1U &&
          memcmp(source + span.offset, expected_group, span.length) == 0);
    CHECK(document_model_source_span(first, circle, &span));
    CHECK(span.length == sizeof("<circle/>") - 1U &&
          memcmp(source + span.offset, "<circle/>", span.length) == 0);
    CHECK(memcmp(source, original, sizeof(source)) == 0);

    document_handle foreign = root;
    CHECK(!document_model_tag_name(second, foreign, &(base_bytes){0}));
    CHECK(!document_model_tag_name(first, (document_handle){root.document_id, UINT32_MAX, 1U},
                                   &(base_bytes){0}));
    CHECK(!document_model_tag_name(first, (document_handle){root.document_id, root.slot, 0U},
                                   &(base_bytes){0}));
    CHECK(document_model_remove_subtree(first, group));
    CHECK(document_model_live_node_count(first) == 2U);
    CHECK(!document_model_tag_name(first, group, &(base_bytes){0}));
    CHECK(!document_model_tag_name(first, rectangle, &(base_bytes){0}));
    CHECK(!document_model_tag_name(first, unknown, &(base_bytes){0}));
    CHECK(!document_model_tag_name(first, path, &(base_bytes){0}));
    CHECK(document_model_next_sibling(first, root, &(document_handle){0}) == false);
    CHECK(document_model_first_child(first, root, &circle));
    CHECK(!document_model_remove_subtree(first, group));
    CHECK(memcmp(source, original, sizeof(source)) == 0);
    CHECK(document_model_remove_subtree(first, root));
    CHECK(document_model_live_node_count(first) == 0U);
    CHECK(!document_model_tag_name(first, root, &(base_bytes){0}));

    static const uint8_t malformed[] = "<svg><g></svg>";
    document_model_result invalid = document_model_parse(
        (base_bytes){malformed, sizeof(malformed) - 1U});
    CHECK(invalid.model == NULL && invalid.error == DOCUMENT_MODEL_PARSE);
    uint8_t *oversized = malloc((16U * 1024U * 1024U) + 1U);
    CHECK(oversized != NULL);
    document_model_result limited = document_model_parse(
        (base_bytes){oversized, (16U * 1024U * 1024U) + 1U});
    CHECK(limited.model == NULL && limited.error == DOCUMENT_MODEL_LIMIT);
    free(oversized);
    CHECK(token_storage_limit_case());
    document_model_destroy(first);
    document_model_destroy(second);
    (void)puts("{\"stable_handles\":true,\"stale_rejections\":6,\"foreign_rejections\":1,\"source_unchanged\":true,\"exact_ranges\":true,\"malformed_xml\":true,\"input_limit\":true,\"token_storage_limit\":true}");
    return 0;
}
