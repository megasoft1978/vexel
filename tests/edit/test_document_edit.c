#include "document_edit.h"
#include "document_edit_internal.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

static bool contains(base_bytes bytes, const char *needle)
{
    size_t length = strlen(needle);
    if (length > bytes.len) return false;
    for (size_t i = 0U; i <= bytes.len - length; ++i)
        if (memcmp(bytes.ptr + i, needle, length) == 0) return true;
    return false;
}

static bool equal_bytes(base_bytes left, base_bytes right)
{
    return left.len == right.len &&
        (left.len == 0U || memcmp(left.ptr, right.ptr, left.len) == 0);
}

static bool materialize(document_edit *edit, uint8_t *buffer, size_t capacity,
                        base_bytes *out_bytes)
{
    size_t length = 0U;
    if (!document_edit_materialize(edit, buffer, capacity, &length)) return false;
    *out_bytes = (base_bytes){buffer, length};
    return true;
}

static bool tokenize(base_bytes bytes)
{
    svg_xml_token tokens[256];
    svg_xml_result result = svg_xml_tokenize(bytes, svg_xml_default_options(),
                                              tokens, 256U);
    return result.diagnostic.category == SVG_XML_ERROR_NONE;
}

static document_edit *create_edit(base_bytes source)
{
    document_edit_result result = document_edit_create(source,
                                                        document_edit_default_limits());
    if (result.error != DOCUMENT_EDIT_OK)
        (void)fprintf(stderr, "create edit failed: %s at %zu\n",
            document_edit_error_name(result.error), result.parse_diagnostic.offset);
    return result.error == DOCUMENT_EDIT_OK ? result.edit : NULL;
}

static bool find_id(document_edit *edit, const char *id, document_handle *out)
{
    return document_edit_find_id(edit,
        (base_bytes){(const uint8_t *)id, strlen(id)}, out);
}

static int test_queries_and_attributes(void)
{
    static const uint8_t source[] =
        "\xef\xbb\xbf<?xml version='1.0'?>\r\n"
        "<svg xmlns='urn:svg'\txmlns:x='urn:x'>\r\n"
        "\t<!--preserve--><g id='root'><text id='txt'>A&amp;B<![CDATA[<c>]]></text>"
        "<rect id='one' x='1'/><circle id='two'/></g>\r\n</svg>\r\n";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit *edit = create_edit(original);
    CHECK(edit != NULL && !document_edit_is_dirty(edit));
    document_handle root, rectangle, text_node;
    CHECK(document_edit_root(edit, &root));
    CHECK(find_id(edit, "one", &rectangle));
    CHECK(find_id(edit, "txt", &text_node));
    base_bytes tag;
    CHECK(document_edit_tag_name(edit, rectangle, &tag));
    CHECK(tag.len == 4U && memcmp(tag.ptr, "rect", 4U) == 0);
    CHECK(document_edit_attribute_count(edit, rectangle) == 2U);
    document_edit_attribute attribute;
    CHECK(document_edit_attribute_at(edit, rectangle, 1U, &attribute));
    CHECK(attribute.quote == '\'');
    document_source_span original_x_value = attribute.value;
    base_bytes raw_value;
    CHECK(document_edit_span_bytes(edit, attribute.value, &raw_value));
    CHECK(raw_value.len == 1U && raw_value.ptr[0] == '1');
    uint8_t decoded[64];
    size_t decoded_length = 0U;
    CHECK(document_edit_text_content(edit, text_node, decoded, sizeof(decoded),
                                     &decoded_length));
    CHECK(decoded_length == 6U && memcmp(decoded, "A&B<c>", 6U) == 0);
    uint8_t path[128];
    size_t path_length = 0U;
    CHECK(document_edit_element_path(edit, rectangle, path, sizeof(path), &path_length));
    CHECK(path_length == strlen("/svg[1]/g[1]/rect[1]") &&
          memcmp(path, "/svg[1]/g[1]/rect[1]", path_length) == 0);
    document_source_span rect_span;
    CHECK(document_edit_source_span(edit, rectangle, &rect_span));
    size_t rect_offset = rect_span.offset + 1U;
    document_handle mapped;
    CHECK(document_edit_element_at_offset(edit, rect_offset, &mapped));
    CHECK(mapped.slot == rectangle.slot && mapped.generation == rectangle.generation);

    uint8_t rejected_value[] = {0x01U};
    CHECK(document_edit_set_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){rejected_value, sizeof(rejected_value)}) ==
        DOCUMENT_EDIT_MALFORMED_XML);
    uint8_t unchanged_output[2048];
    base_bytes unchanged;
    CHECK(materialize(edit, unchanged_output, sizeof(unchanged_output), &unchanged));
    CHECK(equal_bytes(unchanged, original));

    static const uint8_t value[] = "<&'\"";
    CHECK(document_edit_set_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){value, sizeof(value) - 1U}) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_is_dirty(edit));
    CHECK(document_edit_set_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){value, sizeof(value) - 1U}) == DOCUMENT_EDIT_STALE_HANDLE);
    uint8_t output[2048];
    base_bytes edited;
    CHECK(materialize(edit, output, sizeof(output), &edited));
    CHECK(contains(edited, "x='&lt;&amp;&apos;\"'"));
    CHECK(memcmp(edited.ptr, original.ptr, original_x_value.offset) == 0);
    size_t edited_suffix = original_x_value.offset + strlen("&lt;&amp;&apos;\"");
    CHECK(edited_suffix <= edited.len &&
        memcmp(edited.ptr + edited_suffix,
            original.ptr + original_x_value.offset + original_x_value.length,
            original.len - original_x_value.offset - original_x_value.length) == 0);
    CHECK(tokenize(edited));
    CHECK(contains(edited, "\xef\xbb\xbf<?xml version='1.0'?>\r\n"));
    CHECK(contains(edited, "\t<!--preserve-->"));
    CHECK(contains(edited, "xmlns:x='urn:x'"));

    CHECK(find_id(edit, "one", &rectangle));
    CHECK(document_edit_add_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"class", 5U},
        (base_bytes){(const uint8_t *)"shape&mark", 10U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "one", &rectangle));
    CHECK(document_edit_add_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"class", 5U},
        (base_bytes){(const uint8_t *)"x", 1U}) == DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE);
    CHECK(find_id(edit, "one", &rectangle));
    CHECK(document_edit_add_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"bad name", 8U},
        (base_bytes){(const uint8_t *)"x", 1U}) == DOCUMENT_EDIT_INVALID_NAME);
    CHECK(find_id(edit, "one", &rectangle));
    CHECK(document_edit_remove_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"x", 1U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "one", &rectangle));
    CHECK(document_edit_remove_attribute(edit, rectangle,
        (base_bytes){(const uint8_t *)"missing", 7U}) == DOCUMENT_EDIT_NOT_FOUND);

    CHECK(find_id(edit, "txt", &text_node));
    static const uint8_t new_text[] = "hello <&>";
    CHECK(document_edit_set_text_content(edit, text_node,
        (base_bytes){new_text, sizeof(new_text) - 1U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "txt", &text_node));
    CHECK(document_edit_text_content(edit, text_node, decoded, sizeof(decoded),
                                     &decoded_length));
    CHECK(decoded_length == sizeof(new_text) - 1U &&
          memcmp(decoded, new_text, decoded_length) == 0);
    CHECK(materialize(edit, output, sizeof(output), &edited) && tokenize(edited));
    CHECK(contains(edited, "hello &lt;&amp;&gt;"));
    CHECK(document_edit_history_count(edit) >= 3U);
    document_edit_destroy(edit);
    return 0;
}

static int test_structural_operations(void)
{
    static const uint8_t source[] =
        "<svg><g id='layer'><rect id='a'/><!--gap--><circle id='b'/>"
        "<ellipse id='c'/></g></svg>";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit *edit = create_edit(original);
    CHECK(edit != NULL);
    document_handle a, b, c, changed;
    CHECK(find_id(edit, "a", &a) && find_id(edit, "b", &b));
    CHECK(document_edit_reorder_sibling(edit, a, b, false) == DOCUMENT_EDIT_OK);
    uint8_t output[1024];
    base_bytes bytes;
    CHECK(materialize(edit, output, sizeof(output), &bytes) && tokenize(bytes));
    CHECK(contains(bytes, "<!--gap--><circle id='b'/><rect id='a'/><ellipse"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && equal_bytes(bytes, original));
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && tokenize(bytes));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);

    CHECK(find_id(edit, "a", &a));
    CHECK(document_edit_duplicate_element(edit, a, &changed) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_tag_name(edit, changed, &(base_bytes){0}));
    CHECK(find_id(edit, "a", &a));
    CHECK(document_edit_wrap_in_group(edit, a, &changed) == DOCUMENT_EDIT_OK);
    base_bytes tag;
    CHECK(document_edit_tag_name(edit, changed, &tag) && tag.len == 1U && tag.ptr[0] == 'g');
    CHECK(document_edit_ungroup(edit, changed) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && tokenize(bytes));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && tokenize(bytes));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && equal_bytes(bytes, original));

    CHECK(find_id(edit, "b", &b));
    CHECK(document_edit_delete_element(edit, b) == DOCUMENT_EDIT_OK);
    CHECK(!find_id(edit, "b", &changed));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && equal_bytes(bytes, original));
    CHECK(document_edit_root(edit, &changed));
    CHECK(document_edit_delete_element(edit, changed) == DOCUMENT_EDIT_ROOT_DELETE);

    CHECK(find_id(edit, "layer", &changed));
    static const uint8_t attr_name[] = "width";
    static const uint8_t attr_value[] = "50";
    document_edit_template_attribute template_attribute = {
        {attr_name, sizeof(attr_name) - 1U}, {attr_value, sizeof(attr_value) - 1U}
    };
    CHECK(document_edit_insert_template(edit, changed, DOCUMENT_EDIT_TEMPLATE_PATH,
        &template_attribute, 1U, &c) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_tag_name(edit, c, &tag) && tag.len == 4U &&
          memcmp(tag.ptr, "path", 4U) == 0);
    CHECK(materialize(edit, output, sizeof(output), &bytes) &&
          contains(bytes, "<path width=\"50\"/>"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);

    static const char *const template_names[] = {
        "rect", "circle", "ellipse", "line", "path", "g", "text"
    };
    for (size_t kind = 0U; kind < sizeof(template_names) / sizeof(template_names[0]); ++kind) {
        CHECK(find_id(edit, "layer", &changed));
        CHECK(document_edit_insert_template(edit, changed,
            (document_edit_template_kind)kind, NULL, 0U, &c) == DOCUMENT_EDIT_OK);
        CHECK(document_edit_tag_name(edit, c, &tag));
        CHECK(tag.len == strlen(template_names[kind]) &&
              memcmp(tag.ptr, template_names[kind], tag.len) == 0);
        CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    }

    CHECK(find_id(edit, "layer", &changed));
    document_edit_template_attribute text_attribute = {
        {(const uint8_t *)"id", 2U}, {(const uint8_t *)"new-text", 8U}
    };
    CHECK(document_edit_insert_template(edit, changed, DOCUMENT_EDIT_TEMPLATE_TEXT,
        &text_attribute, 1U, &c) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_set_text_content(edit, c,
        (base_bytes){(const uint8_t *)"A<&>", 4U}) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) &&
          contains(bytes, "<text id=\"new-text\">A&lt;&amp;&gt;</text>"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);

    static const uint8_t self_closing[] = "<svg/>";
    document_edit_destroy(edit);
    edit = create_edit((base_bytes){self_closing, sizeof(self_closing) - 1U});
    CHECK(edit != NULL && document_edit_root(edit, &changed));
    CHECK(document_edit_insert_template(edit, changed, DOCUMENT_EDIT_TEMPLATE_CIRCLE,
        NULL, 0U, &c) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) &&
          equal_bytes(bytes, (base_bytes){(const uint8_t *)"<svg><circle/></svg>", 20U}));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) &&
          equal_bytes(bytes, (base_bytes){self_closing, sizeof(self_closing) - 1U}));
    document_edit_destroy(edit);
    return 0;
}

static int test_empty_text_and_xml_fragment(void)
{
    static const uint8_t text_source[] = "<svg><text id='empty'/></svg>";
    document_edit *edit = create_edit((base_bytes){text_source,
        sizeof(text_source) - 1U});
    CHECK(edit != NULL);
    document_handle text_node;
    CHECK(find_id(edit, "empty", &text_node));
    CHECK(document_edit_set_text_content(edit, text_node,
        (base_bytes){(const uint8_t *)"inserted", 8U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "empty", &text_node));
    CHECK(document_edit_set_text_content(edit, text_node,
        (base_bytes){NULL, 0U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "empty", &text_node));
    uint8_t text_buffer[16];
    size_t text_length = 99U;
    CHECK(document_edit_text_content(edit, text_node, text_buffer,
        sizeof(text_buffer), &text_length) && text_length == 0U);
    document_edit_destroy(edit);

    static const uint8_t source[] = "<svg/>";
    base_bytes original = {source, sizeof(source) - 1U};
    edit = create_edit(original);
    CHECK(edit != NULL);
    document_handle root, inserted, nested;
    CHECK(document_edit_root(edit, &root));
    static const uint8_t fragment[] =
        "<!--lead--><g id='fragment'><text id='nested'>old</text></g><!--tail-->";
    CHECK(document_edit_insert_xml_fragment(edit, root,
        (base_bytes){fragment, sizeof(fragment) - 1U}, &inserted) == DOCUMENT_EDIT_OK);
    base_bytes tag;
    CHECK(document_edit_tag_name(edit, inserted, &tag) && tag.len == 1U && tag.ptr[0] == 'g');
    CHECK(find_id(edit, "nested", &nested));
    CHECK(document_edit_set_text_content(edit, nested,
        (base_bytes){(const uint8_t *)"changed & safe", 14U}) == DOCUMENT_EDIT_OK);
    uint8_t output[512];
    base_bytes current;
    CHECK(materialize(edit, output, sizeof(output), &current) && tokenize(current) &&
          contains(current, "<text id='nested'>changed &amp; safe</text>"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &current) && equal_bytes(current, original));

    static const uint8_t malformed[] = "<g><text></g>";
    CHECK(document_edit_root(edit, &root));
    CHECK(document_edit_insert_xml_fragment(edit, root,
        (base_bytes){malformed, sizeof(malformed) - 1U}, &inserted) ==
        DOCUMENT_EDIT_MALFORMED_XML);
    static const uint8_t multiple[] = "<rect/><circle/>";
    CHECK(document_edit_insert_xml_fragment(edit, root,
        (base_bytes){multiple, sizeof(multiple) - 1U}, &inserted) ==
        DOCUMENT_EDIT_INVALID_OPERATION);
    uint8_t oversized[DOCUMENT_EDIT_XML_FRAGMENT_LIMIT + 1U];
    memset(oversized, 'x', sizeof(oversized));
    CHECK(document_edit_insert_xml_fragment(edit, root,
        (base_bytes){oversized, sizeof(oversized)}, &inserted) == DOCUMENT_EDIT_LIMIT);
    CHECK(!document_edit_is_dirty(edit) && document_edit_byte_length(edit) == original.len);
    document_edit_destroy(edit);
    return 0;
}

static int test_replace_xml_region(void)
{
    static const uint8_t source[] =
        "<svg><!--keep--><g id='group'>  <rect id='a'/><!--middle-->"
        "<ellipse id='b'/></g><path id='tail'/></svg>";
    document_edit *edit = create_edit((base_bytes){source, sizeof(source) - 1U});
    CHECK(edit != NULL);
    document_handle group, first, second;
    CHECK(find_id(edit, "group", &group) && find_id(edit, "a", &first) &&
        find_id(edit, "b", &second));
    document_source_span first_span, second_span;
    CHECK(document_edit_source_span(edit, first, &first_span) &&
        document_edit_source_span(edit, second, &second_span));
    size_t end = second_span.offset + second_span.length;
    static const uint8_t replacement[] = "<circle id='replacement'/>";
    CHECK(document_edit_replace_xml_region(edit, group,
        (document_source_span){first_span.offset, end - first_span.offset},
        (base_bytes){replacement, sizeof(replacement) - 1U}) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_history_count(edit) == 1U);
    uint8_t output[512];
    base_bytes current;
    CHECK(materialize(edit, output, sizeof(output), &current) && tokenize(current));
    CHECK(contains(current, "<g id='group'>  <circle id='replacement'/></g>"));
    CHECK(contains(current, "<!--keep-->") && contains(current, "<path id='tail'/>") &&
        !contains(current, "id='a'") && !contains(current, "id='b'"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &current) &&
        equal_bytes(current, (base_bytes){source, sizeof(source) - 1U}));

    static const uint8_t malformed[] = "<circle><rect></circle>";
    CHECK(find_id(edit, "group", &group) && find_id(edit, "a", &first) &&
        find_id(edit, "b", &second));
    CHECK(document_edit_source_span(edit, first, &first_span) &&
        document_edit_source_span(edit, second, &second_span));
    CHECK(document_edit_replace_xml_region(edit, group,
        (document_source_span){first_span.offset,
            second_span.offset + second_span.length - first_span.offset},
        (base_bytes){malformed, sizeof(malformed) - 1U}) ==
        DOCUMENT_EDIT_MALFORMED_XML);
    CHECK(!document_edit_is_dirty(edit) && document_edit_history_count(edit) == 1U);
    document_edit_destroy(edit);
    return 0;
}

static int test_replace_source_span(void)
{
    static const uint8_t source[] = "<svg><path d=\"M 1 2 L 3 4\"/></svg>";
    document_edit *edit = create_edit((base_bytes){source, sizeof(source) - 1U});
    CHECK(edit != NULL);
    size_t number_offset = sizeof("<svg><path d=\"M ") - 1U;
    static const uint8_t number[] = "10";
    CHECK(document_edit_replace_source_span(edit,
        (document_source_span){number_offset, 1U},
        (base_bytes){number, sizeof(number) - 1U}) == DOCUMENT_EDIT_OK);
    uint8_t output[128];
    base_bytes current;
    static const uint8_t expected[] = "<svg><path d=\"M 10 2 L 3 4\"/></svg>";
    CHECK(document_edit_history_count(edit) == 1U &&
        materialize(edit, output, sizeof(output), &current) &&
        equal_bytes(current, (base_bytes){expected, sizeof(expected) - 1U}));
    static const uint8_t invalid[] = "<";
    CHECK(document_edit_replace_source_span(edit,
        (document_source_span){number_offset, 2U},
        (base_bytes){invalid, sizeof(invalid) - 1U}) ==
        DOCUMENT_EDIT_MALFORMED_XML);
    CHECK(document_edit_history_count(edit) == 1U && document_edit_undo(edit) ==
        DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &current) &&
        equal_bytes(current, (base_bytes){source, sizeof(source) - 1U}));
    document_edit_destroy(edit);
    return 0;
}

static int test_insert_text_element(void)
{
    static const uint8_t source[] = "<svg/>";
    document_edit *edit = create_edit((base_bytes){source, sizeof(source) - 1U});
    CHECK(edit != NULL);
    document_handle root, inserted;
    CHECK(document_edit_root(edit, &root));
    document_edit_template_attribute attributes[2] = {
        {{(const uint8_t *)"x", 1U}, {(const uint8_t *)"12", 2U}},
        {{(const uint8_t *)"fill", 4U}, {(const uint8_t *)"#ffffff", 7U}}
    };
    CHECK(document_edit_insert_text_element(edit, root, attributes, 2U,
        (base_bytes){(const uint8_t *)"A<&>", 4U}, &inserted) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_history_count(edit) == 1U);
    static const uint8_t expected[] =
        "<svg><text x=\"12\" fill=\"#ffffff\">A&lt;&amp;&gt;</text></svg>";
    uint8_t output[256];
    base_bytes current;
    CHECK(materialize(edit, output, sizeof(output), &current) &&
        equal_bytes(current, (base_bytes){expected, sizeof(expected) - 1U}));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK &&
        materialize(edit, output, sizeof(output), &current) &&
        equal_bytes(current, (base_bytes){source, sizeof(source) - 1U}));
    CHECK(document_edit_root(edit, &root));
    attributes[1].name = attributes[0].name;
    CHECK(document_edit_insert_text_element(edit, root, attributes, 2U,
        (base_bytes){(const uint8_t *)"duplicate", 9U}, &inserted) ==
        DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE);
    CHECK(document_edit_history_count(edit) == 1U);
    document_edit_destroy(edit);
    return 0;
}

static int test_history_and_save(void)
{
    static const uint8_t source[] = "<svg><rect id='r' x='0'/></svg>";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit *edit = create_edit(original);
    CHECK(edit != NULL);
    document_handle rect;
    CHECK(find_id(edit, "r", &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"1", 1U}) == DOCUMENT_EDIT_OK);
    CHECK(find_id(edit, "r", &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"2", 1U}) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_history_count(edit) == 1U);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    uint8_t output[256];
    base_bytes bytes;
    CHECK(materialize(edit, output, sizeof(output), &bytes) && equal_bytes(bytes, original));
    CHECK(!document_edit_is_dirty(edit));
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, output, sizeof(output), &bytes) && contains(bytes, "x='2'"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);

    char path[] = "/tmp/vexel-edit-save-XXXXXX";
    int fd = mkstemp(path);
    CHECK(fd >= 0);
    size_t written = 0U;
    while (written < original.len) {
        ssize_t amount = write(fd, original.ptr + written, original.len - written);
        CHECK(amount > 0);
        written += (size_t)amount;
    }
    CHECK(close(fd) == 0);
    CHECK(find_id(edit, "r", &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"3", 1U}) == DOCUMENT_EDIT_OK);
    document_edit_save_result saved = document_edit_save_as(edit,
        (base_bytes){(const uint8_t *)path, strlen(path)}, NULL);
    CHECK(saved.error == DOCUMENT_EDIT_OK && !document_edit_is_dirty(edit));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0);
    uint8_t saved_bytes[256];
    ssize_t read_length = read(fd, saved_bytes, sizeof(saved_bytes));
    CHECK(read_length > 0 && close(fd) == 0);
    CHECK(contains((base_bytes){saved_bytes, (size_t)read_length}, "x='3'"));
    CHECK(unlink(path) == 0);

    char directory_template[] = "/tmp/vexel-edit-new-XXXXXX";
    char *directory = mkdtemp(directory_template);
    CHECK(directory != NULL);
    char new_path[256];
    int path_length = snprintf(new_path, sizeof(new_path), "%s/new.svg", directory);
    CHECK(path_length > 0 && (size_t)path_length < sizeof(new_path));
    CHECK(find_id(edit, "r", &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"4", 1U}) == DOCUMENT_EDIT_OK);
    saved = document_edit_save_as(edit,
        (base_bytes){(const uint8_t *)new_path, (size_t)path_length}, NULL);
    CHECK(saved.error == DOCUMENT_EDIT_OK && saved.source_result.file_result.renamed &&
          !document_edit_is_dirty(edit));
    fd = open(new_path, O_RDONLY);
    CHECK(fd >= 0);
    read_length = read(fd, saved_bytes, sizeof(saved_bytes));
    CHECK(read_length > 0 && close(fd) == 0);
    CHECK(contains((base_bytes){saved_bytes, (size_t)read_length}, "x='4'"));
    CHECK(unlink(new_path) == 0 && rmdir(directory) == 0);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK && document_edit_is_dirty(edit));
    document_edit_destroy(edit);
    return 0;
}

static int test_history_bounds(void)
{
    static const uint8_t source[] = "<svg><rect x='0'/></svg>";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit_limits limits = {2U, 64U};
    document_edit_result created = document_edit_create(original, limits);
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    document_edit *edit = created.edit;
    for (uint8_t value = '1'; value <= '3'; ++value) {
        CHECK(document_edit_validate_replace(edit, 5U, 13U, 14U, 1U,
            &value, 1U, false, true));
        CHECK(document_edit_commit_replace(edit, 14U, 1U, &value, 1U,
            1U, false) == DOCUMENT_EDIT_OK);
    }
    CHECK(document_edit_history_count(edit) == 2U &&
          document_edit_history_cursor(edit) == 2U);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_INVALID_OPERATION);
    uint8_t buffer[64];
    base_bytes current;
    CHECK(materialize(edit, buffer, sizeof(buffer), &current) && contains(current, "x='1'"));
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    CHECK(materialize(edit, buffer, sizeof(buffer), &current) && contains(current, "x='3'"));
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    uint8_t changed = '9';
    CHECK(document_edit_validate_replace(edit, 5U, 13U, 14U, 1U,
        &changed, 1U, false, true));
    CHECK(document_edit_commit_replace(edit, 14U, 1U, &changed, 1U,
        1U, false) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_INVALID_OPERATION);
    document_edit_destroy(edit);

    limits = (document_edit_limits){10U, 2U};
    created = document_edit_create(original, limits);
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    edit = created.edit;
    document_handle rect;
    CHECK(find_id(edit, "", &rect) == false);
    CHECK(document_edit_root(edit, &rect));
    CHECK(document_model_first_child(document_edit_model(edit), rect, &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"1", 1U}) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_find_id(edit,
        (base_bytes){(const uint8_t *)"absent", 6U}, &rect) == false);
    CHECK(document_edit_root(edit, &rect));
    CHECK(document_model_first_child(document_edit_model(edit), rect, &rect));
    CHECK(document_edit_set_attribute(edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"2", 1U}) == DOCUMENT_EDIT_HISTORY_LIMIT);
    CHECK(materialize(edit, buffer, sizeof(buffer), &current) && contains(current, "x='1'"));
    CHECK(document_edit_history_count(edit) == 1U && edit->blob_bytes == 2U);
    document_edit_destroy(edit);
    return 0;
}

static int test_invalid_source(void)
{
    static const uint8_t malformed[] = "<svg><g></svg>";
    document_edit_result result = document_edit_create(
        (base_bytes){malformed, sizeof(malformed) - 1U},
        document_edit_default_limits());
    CHECK(result.edit == NULL && result.error == DOCUMENT_EDIT_MALFORMED_XML);
    CHECK(result.parse_diagnostic.category == SVG_XML_ERROR_MISMATCHED_TAG);
    return 0;
}

static uint32_t random_state = 0x9e3779b9U;

static uint32_t random_next(void)
{
    random_state ^= random_state << 13U;
    random_state ^= random_state >> 17U;
    random_state ^= random_state << 5U;
    return random_state;
}

static int test_randomized_history(void)
{
    static const uint8_t source[] = "<svg><rect x='0'/></svg>";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit_limits limits = {200000U, 4U * 1024U * 1024U};
    document_edit_result created = document_edit_create(original, limits);
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    document_edit *edit = created.edit;
    uint8_t current[64];
    for (size_t step = 0U; step < 200000U; ++step) {
        uint32_t choice = random_next() % 3U;
        if (choice == 0U || (choice == 1U && document_edit_history_cursor(edit) == 0U) ||
            (choice == 2U && document_edit_history_cursor(edit) ==
                document_edit_history_count(edit))) {
            uint8_t value = (uint8_t)('0' + (random_next() % 10U));
            CHECK(document_edit_validate_replace(edit, 5U, 13U, 14U, 1U,
                &value, 1U, false, true));
            CHECK(document_edit_commit_replace(edit, 14U, 1U, &value, 1U,
                1U, false) == DOCUMENT_EDIT_OK);
        } else if (choice == 1U) {
            CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
        } else {
            CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
        }
        base_bytes snapshot;
        CHECK(materialize(edit, current, sizeof(current), &snapshot));
        CHECK(tokenize(snapshot));
        CHECK(snapshot.len == original.len &&
              memcmp(snapshot.ptr, original.ptr, 14U) == 0 &&
              memcmp(snapshot.ptr + 15U, original.ptr + 15U, original.len - 15U) == 0);
        CHECK(edit->blob_bytes <= limits.history_bytes);
    }
    while (document_edit_history_cursor(edit) < document_edit_history_count(edit))
        CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    uint8_t final_bytes[64];
    base_bytes expected_final;
    CHECK(materialize(edit, final_bytes, sizeof(final_bytes), &expected_final));
    while (document_edit_history_cursor(edit) > 0U)
        CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    uint8_t undone_bytes[64];
    base_bytes undone;
    CHECK(materialize(edit, undone_bytes, sizeof(undone_bytes), &undone));
    CHECK(equal_bytes(undone, original) && tokenize(undone));
    while (document_edit_history_cursor(edit) < document_edit_history_count(edit))
        CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    uint8_t redone_bytes[64];
    base_bytes redone;
    CHECK(materialize(edit, redone_bytes, sizeof(redone_bytes), &redone));
    CHECK(equal_bytes(redone, expected_final) && tokenize(redone));
    CHECK(edit->blob_bytes <= limits.history_bytes);
    document_edit_destroy(edit);
    return 0;
}

static uint32_t next_fragment_random(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value;
    return value;
}

static int test_randomized_xml_fragments(void)
{
    static const uint8_t source[] = "<svg/>";
    base_bytes original = {source, sizeof(source) - 1U};
    document_edit *edit = create_edit(original);
    CHECK(edit != NULL);
    uint32_t random = UINT32_C(0x57465835);
    size_t accepted = 0U, malformed_count = 0U, shape_rejected = 0U;
    for (size_t iteration = 0U; iteration < 200000U; ++iteration) {
        uint32_t choice = next_fragment_random(&random) % 5U;
        uint32_t value = next_fragment_random(&random);
        char markup[128];
        int amount;
        document_edit_error expected;
        if (choice == 0U) {
            amount = snprintf(markup, sizeof(markup), "<rect data-v='%u'/>", value);
            expected = DOCUMENT_EDIT_OK;
        } else if (choice == 1U) {
            amount = snprintf(markup, sizeof(markup), "<!--c--><circle id='r%u'/>", value);
            expected = DOCUMENT_EDIT_OK;
        } else if (choice == 2U) {
            amount = snprintf(markup, sizeof(markup), "<g><text>%u</text></g>", value);
            expected = DOCUMENT_EDIT_OK;
        } else if (choice == 3U) {
            amount = snprintf(markup, sizeof(markup), "<rect><circle>%u</rect>", value);
            expected = DOCUMENT_EDIT_MALFORMED_XML;
        } else {
            memcpy(markup, "<rect/><circle/>", sizeof("<rect/><circle/>"));
            amount = (int)(sizeof("<rect/><circle/>") - 1U);
            expected = DOCUMENT_EDIT_INVALID_OPERATION;
        }
        CHECK(amount > 0 && (size_t)amount < sizeof(markup));
        document_handle root, inserted;
        CHECK(document_edit_root(edit, &root));
        document_edit_error actual = document_edit_insert_xml_fragment(edit, root,
            (base_bytes){(const uint8_t *)markup, (size_t)amount}, &inserted);
        CHECK(actual == expected);
        if (actual == DOCUMENT_EDIT_OK) {
            accepted++;
            CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
        } else if (actual == DOCUMENT_EDIT_MALFORMED_XML) malformed_count++;
        else shape_rejected++;
        CHECK(document_edit_byte_length(edit) == original.len &&
              !document_edit_is_dirty(edit));
    }
    uint8_t output[16];
    base_bytes final;
    CHECK(materialize(edit, output, sizeof(output), &final) && equal_bytes(final, original));
    CHECK(accepted > 0U && malformed_count > 0U && shape_rejected > 0U);
    document_edit_destroy(edit);
    (void)printf("fragment fuzz: 200000 seeded cases, seed 0x57465835, accepted=%zu malformed=%zu rejected=%zu\n",
        accepted, malformed_count, shape_rejected);
    return 0;
}

int main(void)
{
    CHECK(test_queries_and_attributes() == 0);
    CHECK(test_structural_operations() == 0);
    CHECK(test_empty_text_and_xml_fragment() == 0);
    CHECK(test_replace_xml_region() == 0);
    CHECK(test_replace_source_span() == 0);
    CHECK(test_insert_text_element() == 0);
    CHECK(test_history_and_save() == 0);
    CHECK(test_history_bounds() == 0);
    CHECK(test_invalid_source() == 0);
    CHECK(test_randomized_history() == 0);
    CHECK(test_randomized_xml_fragments() == 0);
    (void)puts("{\"edit_ops\":true,\"byte_preservation\":true,\"undo_redo_sequences\":200000,\"xml_fragments\":200000,\"final_round_trip\":true,\"bounded_history\":true}");
    return 0;
}
