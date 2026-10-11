#include "viewer_editor.h"
#include "svg/shape/shape.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='animated' x='8' y='8' width='24' "
    "height='16'><animate attributeName='x' from='8' to='24' dur='2s' "
    "repeatCount='indefinite'/></rect></svg>";

static const uint8_t reference_source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><g id='bundle'><defs><clipPath id='clip'>"
    "<circle cx='20' cy='20' r='8'/></clipPath></defs><rect id='target' "
    "x='8' y='8' width='24' height='16' clip-path='url(#clip)'/>"
    "</g></svg>";

int main(void)
{
    viewer_editor *editor = NULL;
    svg_shape_document decoded = {0};
    uint8_t *clipboard = NULL;
    uint8_t *materialized = NULL;
    size_t clipboard_length = 0U, source_length = 0U;
    int result = 1;

    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 64U, 64U);
    clipboard = malloc(DOCUMENT_EDIT_XML_FRAGMENT_LIMIT);
    materialized = malloc(DOCUMENT_EDIT_XML_FRAGMENT_LIMIT + 1U);
    CHECK(editor != NULL && clipboard != NULL && materialized != NULL);
    size_t offset = (size_t)(strstr((const char *)source, "<rect") -
        (const char *)source);
    CHECK(viewer_editor_select_at_source_offset(editor, offset));
    CHECK(viewer_editor_copy_selection(editor, clipboard,
        DOCUMENT_EDIT_XML_FRAGMENT_LIMIT, &clipboard_length));
    CHECK((viewer_editor_paste_svg_from_clipboard(editor, clipboard,
        clipboard_length, false) &
        VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(viewer_editor_materialize(editor, materialized,
        DOCUMENT_EDIT_XML_FRAGMENT_LIMIT, &source_length));
    materialized[source_length] = 0U;
    CHECK(strstr((const char *)materialized, "id='animated-copy-1'") != NULL);
    svg_shape_diagnostic diagnostic = {0};
    CHECK(svg_shape_decode((base_bytes){materialized, source_length}, &decoded,
        &diagnostic) == SVG_SHAPE_ERROR_NONE);
    CHECK(decoded.shape_count == 2U && decoded.animation.track_count == 2U);

    svg_shape_document_destroy(&decoded);
    viewer_editor_destroy(editor);
    editor = viewer_editor_create((base_bytes){reference_source,
        sizeof(reference_source) - 1U}, UI_THEME_DARK, 64U, 64U);
    CHECK(editor != NULL);
    offset = (size_t)(strstr((const char *)reference_source, "<g") -
        (const char *)reference_source);
    CHECK(viewer_editor_select_at_source_offset(editor, offset));
    CHECK(viewer_editor_copy_selection(editor, clipboard,
        DOCUMENT_EDIT_XML_FRAGMENT_LIMIT, &clipboard_length));
    CHECK((viewer_editor_paste_svg_from_clipboard(editor, clipboard,
        clipboard_length, false) & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(viewer_editor_materialize(editor, materialized,
        DOCUMENT_EDIT_XML_FRAGMENT_LIMIT, &source_length));
    materialized[source_length] = 0U;
    CHECK(strstr((const char *)materialized, "id='bundle-copy-1'") != NULL);
    CHECK(strstr((const char *)materialized, "id='clip-copy-1'") != NULL);
    CHECK(strstr((const char *)materialized,
        "clip-path='url(#clip-copy-1)'") != NULL);
    CHECK(svg_shape_decode((base_bytes){materialized, source_length}, &decoded,
        &diagnostic) == SVG_SHAPE_ERROR_NONE);
    result = 0;
done:
    svg_shape_document_destroy(&decoded);
    viewer_editor_destroy(editor);
    free(clipboard);
    free(materialized);
    return result;
}
