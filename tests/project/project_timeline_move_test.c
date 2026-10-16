#include "project_timeline_edit.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool contains(base_bytes source, const char *text)
{
    size_t length = strlen(text);
    if (length > source.len) return false;
    for (size_t i = 0U; i <= source.len - length; ++i)
        if (memcmp(source.ptr + i, text, length) == 0) return true;
    return false;
}

static bool test_move_drawing_cel_preserves_payload_and_history(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":["
        "{\"id\":\"draw\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"name\":\"preserve track\",\"cels\":[{\"tick\":0,\"asset\":\"cel0\"},  \n\t "
        "{\"tick\":4,\"asset\":\"cel1\",\"label\":\"pose\"}]}]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("draw"), 4U, 12U).error ==
        PROJECT_TIMELINE_OK);
    base_bytes moved = project_timeline_edit_source(edit);
    CHECK(contains(moved, "\"name\":\"preserve track\""));
    CHECK(contains(moved, "},  \n\t {\"tick\":12,\"asset\":\"cel1\",\"label\":\"pose\""));
    project_timeline_view view = {0};
    CHECK(project_timeline_decode(moved, &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_item_view item;
    CHECK(project_timeline_item_at(&view, 0U, 1U, &item) && item.tick == 12U);
    CHECK(project_timeline_edit_undo_count(edit) == 1U);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_source(edit).len == bytes(initial).len &&
        memcmp(project_timeline_edit_source(edit).ptr, initial, bytes(initial).len) == 0);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    moved = project_timeline_edit_source(edit);
    CHECK(contains(moved, "\"tick\":12,\"asset\":\"cel1\""));
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_move_property_key_and_rejections(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"fade\",\"kind\":\"property\",\"target\":\"layer\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":1},"
        "{\"tick\":5,\"value\":0.5},"
        "{\"tick\":10,\"value\":0}]}]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    project_timeline_context context = {ids, 1U, NULL, 0U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    base_bytes before = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 0U, 2U).error ==
        PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 5U, 10U).error ==
        PROJECT_TIMELINE_DUPLICATE);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 5U, 24U).error ==
        PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("missing"), 5U, 8U).error ==
        PROJECT_TIMELINE_INVALID_REFERENCE);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 5U, 5U).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 0U, 0U).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_undo_count(edit) == 0U &&
        project_timeline_edit_source(edit).len == before.len &&
        memcmp(project_timeline_edit_source(edit).ptr, before.ptr, before.len) == 0);
    CHECK(project_timeline_edit_move_item(edit, &context, bytes("fade"), 5U, 2U).error ==
        PROJECT_TIMELINE_OK);
    base_bytes moved = project_timeline_edit_source(edit);
    CHECK(contains(moved, "\"tick\":2,\"value\":0.5"));
    project_timeline_view view = {0};
    CHECK(project_timeline_decode(moved, &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_item_view item;
    CHECK(project_timeline_item_at(&view, 0U, 1U, &item) && item.tick == 2U);
    CHECK(project_timeline_edit_undo_count(edit) == 1U);
    project_timeline_edit_destroy(edit);
    return true;
}

int main(void)
{
    return test_move_drawing_cel_preserves_payload_and_history() &&
        test_move_property_key_and_rejections() ? EXIT_SUCCESS : EXIT_FAILURE;
}
