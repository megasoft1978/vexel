#define _POSIX_C_SOURCE 200809L

#include "viewer_editor.h"

#include "xml_tokenizer.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)

static const uint8_t source[] =
    "<svg width='32' height='32'><rect id='target' x='1' y='2' width='8' height='8'/>"
    "<circle id='other' cx='20' cy='20' r='4'/></svg>";

static uint32_t next_random(uint32_t *state)
{
    *state ^= *state << 13U;
    *state ^= *state >> 17U;
    *state ^= *state << 5U;
    return *state;
}

static bool materialize(viewer_editor *editor, uint8_t *buffer, size_t capacity,
                        size_t *out_length)
{
    return viewer_editor_materialize(editor, buffer, capacity, out_length);
}

static bool well_formed(viewer_editor *editor)
{
    uint8_t bytes[1024];
    size_t length = 0U;
    if (!materialize(editor, bytes, sizeof(bytes), &length)) return false;
    svg_xml_token tokens[128];
    svg_xml_result result = svg_xml_tokenize((base_bytes){bytes, length},
        svg_xml_default_options(), tokens, 128U);
    return result.diagnostic.category == SVG_XML_ERROR_NONE;
}

static bool bytes_equal(const uint8_t *left, size_t left_length,
                        const uint8_t *right, size_t right_length)
{
    return left_length == right_length &&
        (left_length == 0U || memcmp(left, right, left_length) == 0);
}

static bool bytes_contain(const uint8_t *bytes, size_t length,
                          const char *needle)
{
    size_t needle_length = strlen(needle);
    if (needle_length > length) return false;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(bytes + index, needle, needle_length) == 0) return true;
    return false;
}

static size_t bytes_count(const uint8_t *bytes, size_t length, const char *needle)
{
    size_t needle_length = strlen(needle);
    size_t count = 0U;
    if (needle_length > length) return 0U;
    for (size_t index = 0U; index <= length - needle_length; ++index)
        if (memcmp(bytes + index, needle, needle_length) == 0) ++count;
    return count;
}

static int test_edit_save_and_semantics(void)
{
    viewer_editor *editor = NULL;
    char path[] = "/tmp/vexel-editor-save.XXXXXX";
    int fd = -1;
    uint8_t bytes[2048];
    size_t length = 0U;
    int result = 1;
    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 1280U, 800U);
    CHECK(editor != NULL);
    ui_semantics_snapshot first = {0}, second = {0};
    CHECK(viewer_editor_semantics(editor, &first));
    CHECK(viewer_editor_semantics(editor, &second));
    CHECK(memcmp(&first, &second, sizeof(first)) == 0);
    CHECK(first.count > 20U);
    uint32_t result_flags = viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    CHECK((result_flags & VIEWER_EDITOR_HANDLED) != 0U);
    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "9", 1U);
    CHECK(viewer_editor_is_dirty(editor));
    (void)viewer_editor_key(editor, 53U, 0U, NULL, 0U);
    CHECK(!viewer_editor_is_dirty(editor));
    CHECK(well_formed(editor));
    CHECK(viewer_editor_selection_span(editor, &(document_source_span){0}));
    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "main", 4U);
    result_flags = viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    CHECK((result_flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(viewer_editor_is_dirty(editor));
    CHECK(well_formed(editor));
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length, "id='main'"));
    result_flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((result_flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, source, sizeof(source) - 1U));
    result_flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U);
    CHECK((result_flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length, "id='main'"));

    fd = mkstemp(path);
    CHECK(fd >= 0);
    size_t written = 0U;
    while (written < sizeof(source) - 1U) {
        ssize_t amount = write(fd, source + written, sizeof(source) - 1U - written);
        CHECK(amount > 0);
        written += (size_t)amount;
    }
    CHECK(close(fd) == 0);
    fd = -1;
    document_edit_save_result saved = viewer_editor_save(editor,
        (base_bytes){(const uint8_t *)path, strlen(path)});
    CHECK(saved.error == DOCUMENT_EDIT_OK);
    CHECK(!viewer_editor_is_dirty(editor));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0);
    ssize_t read_amount = read(fd, bytes, sizeof(bytes));
    CHECK(read_amount > 0);
    length = (size_t)read_amount;
    CHECK(close(fd) == 0);
    fd = -1;
    CHECK(bytes_equal(bytes, length,
        (const uint8_t *)"<svg width='32' height='32'><rect id='main' x='1' y='2' width='8' height='8'/>"
        "<circle id='other' cx='20' cy='20' r='4'/></svg>",
        strlen("<svg width='32' height='32'><rect id='main' x='1' y='2' width='8' height='8'/>"
        "<circle id='other' cx='20' cy='20' r='4'/></svg>")));
    result = 0;
done:
    if (fd >= 0) (void)close(fd);
    (void)unlink(path);
    viewer_editor_destroy(editor);
    return result;
}

static int test_randomized_events(void)
{
    viewer_editor *editor = NULL;
    uint8_t bytes[1024];
    size_t length = 0U;
    uint32_t random_state = UINT32_C(20261016);
    int result = 1;
    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_LIGHT, 1280U, 800U);
    CHECK(editor != NULL);
    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    for (size_t cycle = 0U; cycle < 40000U; ++cycle) {
        uint32_t random = next_random(&random_state);
        (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
        CHECK(well_formed(editor));

        random = next_random(&random_state);
        static const uint16_t movement_keys[] = {123U, 124U, 115U, 119U};
        uint16_t movement = movement_keys[random % 4U];
        uint32_t modifiers = (random & 8U) != 0U ? PLATFORM_MODIFIER_OPTION : 0U;
        if ((random & 16U) != 0U) modifiers |= PLATFORM_MODIFIER_SHIFT;
        (void)viewer_editor_key(editor, movement, modifiers, NULL, 0U);
        CHECK(well_formed(editor));

        random = next_random(&random_state);
        char digit = (char)('0' + random % 10U);
        (void)viewer_editor_key(editor, 0U, 0U, &digit, 1U);
        CHECK(well_formed(editor));

        random = next_random(&random_state);
        bool commit = (random & 1U) != 0U;
        uint32_t flags = commit ? viewer_editor_key(editor, 36U, 0U, NULL, 0U) :
            viewer_editor_key(editor, 53U, 0U, NULL, 0U);
        CHECK(well_formed(editor));

        if ((flags & VIEWER_EDITOR_MUTATED) != 0U)
            (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
        else
            (void)viewer_editor_pointer_down(editor, 600U, 400U, 0U);
        CHECK(well_formed(editor));
    }
    while (true) {
        uint32_t flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
        if ((flags & VIEWER_EDITOR_MUTATED) == 0U) break;
        CHECK(well_formed(editor));
    }
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, source, sizeof(source) - 1U));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

static int test_structure_and_attribute_controls(void)
{
    viewer_editor *editor = NULL;
    uint8_t bytes[2048];
    size_t length = 0U;
    int result = 1;
    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 1280U, 800U);
    CHECK(editor != NULL);
    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    uint32_t flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "d", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_count(bytes, length, "id='target'") == 2U);
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, source, sizeof(source) - 1U));

    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "g", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "g", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, source, sizeof(source) - 1U));

    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    flags = viewer_editor_key(editor, 0U, 0U, "]", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, source, sizeof(source) - 1U));

    (void)viewer_editor_semantics(editor, &(ui_semantics_snapshot){0});
    (void)viewer_editor_pointer_down(editor, 1000U, 275U, 0U);
    (void)viewer_editor_key(editor, 0U, 0U, "data-note", 9U);
    (void)viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    (void)viewer_editor_key(editor, 0U, 0U, "safe", 4U);
    (void)viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    flags = viewer_editor_pointer_down(editor, 1000U, 333U, 0U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(viewer_editor_is_dirty(editor));
    CHECK(well_formed(editor));
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length, "data-note=\"safe\""));

    flags = viewer_editor_key(editor, 51U, 0U, NULL, 0U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length, "id='target'"));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

static int test_navigation_commands_commit_active_field(void)
{
    viewer_editor *editor = NULL;
    int result = 1;
    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 1280U, 800U);
    CHECK(editor != NULL);
    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "9", 1U);
    uint32_t flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "q", 1U);
    CHECK((flags & (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_QUIT)) ==
        (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_QUIT));
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);

    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "7", 1U);
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "o", 1U);
    CHECK((flags & (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_OPEN)) ==
        (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_OPEN));
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);

    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "6", 1U);
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "r", 1U);
    CHECK((flags & (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_RELOAD)) ==
        (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_RELOAD));
    CHECK(well_formed(editor));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

static int test_save_as_shortcut_flags(void)
{
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U);
    int result = 1;
    CHECK(editor != NULL);
    uint32_t flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "s", 1U);
    CHECK((flags & (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE_AS)) ==
        (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE_AS));
    CHECK((flags & VIEWER_EDITOR_SAVE) == 0U);

    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    (void)viewer_editor_key(editor, 0U, 0U, "e", 1U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    (void)viewer_editor_key(editor, 0U, 0U, "11", 2U);
    flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "s", 1U);
    CHECK((flags & (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_SAVE_AS)) ==
        (VIEWER_EDITOR_MUTATED | VIEWER_EDITOR_SAVE_AS));
    CHECK((flags & VIEWER_EDITOR_SAVE) == 0U);
    CHECK(well_formed(editor));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

static int test_png_shortcut_flags(void)
{
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_DARK, 1280U, 800U);
    int result = 1;
    CHECK(editor != NULL);
    uint32_t flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "e", 1U);
    CHECK((flags & (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_EXPORT_PNG)) ==
        (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_EXPORT_PNG));
    flags = viewer_editor_key(editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_OPTION, "c", 1U);
    CHECK((flags & (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COPY_PNG)) ==
        (VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COPY_PNG));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

static int test_z_order_shortcuts(void)
{
    static const uint8_t ordered[] =
        "<svg><rect id='one'/><ellipse id='two'/><line id='three'/></svg>";
    viewer_editor *editor = viewer_editor_create((base_bytes){ordered,
        sizeof(ordered) - 1U}, UI_THEME_DARK, 1280U, 800U);
    uint8_t bytes[512];
    size_t length = 0U;
    int result = 1;
    CHECK(editor != NULL);
    (void)viewer_editor_key(editor, 124U, 0U, NULL, 0U);
    uint32_t flags = viewer_editor_key(editor, 30U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "]", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length,
        "<ellipse id='two'/><line id='three'/><rect id='one'/>") );
    CHECK(well_formed(editor));
    flags = viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_equal(bytes, length, ordered, sizeof(ordered) - 1U));

    size_t last_shape_offset = sizeof("<svg><rect id='one'/><ellipse id='two'/>") - 1U;
    CHECK(viewer_editor_select_at_source_offset(editor, last_shape_offset + 2U));
    flags = viewer_editor_key(editor, 33U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "[", 1U);
    CHECK((flags & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(materialize(editor, bytes, sizeof(bytes), &length));
    CHECK(bytes_contain(bytes, length,
        "<line id='three'/><rect id='one'/><ellipse id='two'/>") );
    CHECK(well_formed(editor));
    result = 0;
done:
    viewer_editor_destroy(editor);
    return result;
}

int main(void)
{
    if (test_edit_save_and_semantics() != 0) return 1;
    if (test_structure_and_attribute_controls() != 0) return 1;
    if (test_navigation_commands_commit_active_field() != 0) return 1;
    if (test_save_as_shortcut_flags() != 0) return 1;
    if (test_png_shortcut_flags() != 0) return 1;
    if (test_z_order_shortcuts() != 0) return 1;
    if (test_randomized_events() != 0) return 1;
    (void)puts("editor state: structural edits, save, semantics, and 200000 randomized events passed");
    return 0;
}
