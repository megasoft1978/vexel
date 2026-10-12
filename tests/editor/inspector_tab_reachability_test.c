/* WO-GAP-UI-02: every attribute of the selected element is reachable on
 * exactly one of the four inspector tabs. The attributes are enumerated from
 * the document, not from a hard-coded list, and each tab is opened by a real
 * pointer click on its tab widget. */
#include "viewer_editor.h"
#include "viewer_editor_internal.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TAB_COUNT 4U
#define ATTRIBUTE_CAP 32U

static const char source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' "
    "xmlns:xlink='http://www.w3.org/1999/xlink' width='64' height='64'>"
    "<rect id='probe' x='1' y='2' width='3' height='4' transform='translate(1 1)' "
    "fill='#336699' fill-opacity='0.5' stroke='#112233' stroke-width='2' "
    "stroke-opacity='0.75' stroke-linecap='round' stroke-linejoin='bevel' "
    "data-note='unknown' xlink:title='namespaced'/>"
    "</svg>";

static bool click_node(viewer_editor *editor, const ui_semantics_node *node)
{
    if (node == NULL || node->frame.width == 0U) return false;
    uint32_t x = node->frame.x + node->frame.width / 2U;
    uint32_t y = node->frame.y + node->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, x, y);
    bool handled = (viewer_editor_pointer_down(editor, x, y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U;
    viewer_editor_pointer_up(editor);
    return handled;
}

static const ui_semantics_node *find_id(const ui_semantics_snapshot *snapshot,
                                        uint32_t id)
{
    for (size_t index = 0U; index < snapshot->count; ++index)
        if (snapshot->nodes[index].id == id) return &snapshot->nodes[index];
    return NULL;
}

int main(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){(const uint8_t *)source, sizeof(source) - 1U},
        UI_THEME_DARK, 1280U, 800U, true);
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    if (editor == NULL || snapshot == NULL) return 1;
    const char *rect = strstr(source, "<rect");
    if (rect == NULL ||
        !viewer_editor_select_at_source_offset(editor, (size_t)(rect - source)) ||
        !editor->selection_active) return 2;

    /* Enumerate the selected element's attributes from the document. */
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    if (count < 14U || count > ATTRIBUTE_CAP) return 3;
    char names[ATTRIBUTE_CAP][48];
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name;
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) || !document_edit_span_bytes(editor->edit,
                attribute.name, &name) || name.len >= sizeof(names[0])) return 4;
        memcpy(names[index], name.ptr, name.len);
        names[index][name.len] = '\0';
    }

    /* shown[tab][attribute]: the attribute has a row (id: the Name field). */
    bool shown[TAB_COUNT][ATTRIBUTE_CAP] = {{false}};
    for (uint32_t tab = 0U; tab < TAB_COUNT; ++tab) {
        if (!viewer_editor_semantics(editor, snapshot)) return 5;
        if (!click_node(editor, find_id(snapshot,
                EDITOR_BUTTON_INSPECTOR_TAB_BASE + tab)) ||
            !viewer_editor_semantics(editor, snapshot)) return 6;
        const ui_semantics_node *active = find_id(snapshot,
            EDITOR_BUTTON_INSPECTOR_TAB_BASE + tab);
        if (active == NULL || (active->state & UI_STATE_CHECKED) == 0U) return 7;
        /* Collapsed Object sections are one pointer click away, never hidden
         * for good: open each so its attributes count as reachable. */
        for (uint32_t pass = 0U; pass < 3U; ++pass) {
            const ui_semantics_node *collapsed = NULL;
            for (uint32_t slot = 0U; slot < 3U && collapsed == NULL; ++slot) {
                const ui_semantics_node *header = find_id(snapshot,
                    EDITOR_INSPECTOR_SECTION_BASE + slot);
                if (header != NULL && (header->state & UI_STATE_EXPANDED) == 0U)
                    collapsed = header;
            }
            if (collapsed == NULL) break;
            if (!click_node(editor, collapsed) ||
                !viewer_editor_semantics(editor, snapshot)) return 8;
        }
        for (size_t index = 0U; index < count; ++index) {
            uint32_t row = strcmp(names[index], "id") == 0 ? EDITOR_RENAME_ID :
                EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)index;
            shown[tab][index] = find_id(snapshot, row) != NULL;
        }
    }

    int failures = 0;
    static const char *const tab_names[TAB_COUNT] = {
        "Object", "Fill", "Stroke", "Document"};
    for (size_t index = 0U; index < count; ++index) {
        unsigned found = 0U;
        for (uint32_t tab = 0U; tab < TAB_COUNT; ++tab) found += shown[tab][index];
        if (found == 1U) continue;
        ++failures;
        (void)fprintf(stderr, "attribute '%s' is on %u tabs:", names[index], found);
        for (uint32_t tab = 0U; tab < TAB_COUNT; ++tab)
            if (shown[tab][index]) (void)fprintf(stderr, " %s", tab_names[tab]);
        (void)fputc('\n', stderr);
    }
    free(snapshot);
    viewer_editor_destroy(editor);
    if (failures != 0) return 10;
    (void)puts("inspector tab reachability passed: each attribute on exactly one tab");
    return 0;
}
