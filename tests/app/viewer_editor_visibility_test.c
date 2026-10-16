#include "viewer_editor.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        (void)fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

bool viewer_editor_visibility_test_run(void);

static bool check_visibility_source(const char *source, bool expected_ok,
    bool expected_visible)
{
    viewer_editor *editor = viewer_editor_create(
        (base_bytes){(const uint8_t *)source, strlen(source)},
        UI_THEME_DARK, 640U, 480U);
    CHECK(editor != NULL);
    const char *element = strstr(source, "<g id='x'");
    if (element == NULL) element = strstr(source, "<g");
    CHECK(element != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(element - source) + 1U));
    uint8_t id[32U]; size_t id_length = 0U; bool visible = true;
    bool actual = viewer_editor_selected_visibility_target(editor, id,
        sizeof(id), &id_length, &visible);
    CHECK(actual == expected_ok);
    if (actual) CHECK(visible == expected_visible);
    viewer_editor_destroy(editor);
    return true;
}

bool viewer_editor_visibility_test_run(void)
{
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g id='x'/></svg>", true, true));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g id='x' visibility='hidden'/></svg>",
        true, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g id='x' visibility='collapse'/></svg>",
        true, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g id='x' visibility='visible' style='visibility:hidden'/></svg>",
        false, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g visibility='hidden'><g id='x'/></g></svg>",
        false, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><style>g{visibility:hidden}</style><g id='x'/></svg>",
        false, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g/></svg>", false, false));
    CHECK(check_visibility_source(
        "<svg width='8' height='8'><g id='x'/><g id='x'/></svg>",
        false, false));
    return true;
}
