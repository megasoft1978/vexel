#include "project_timeline_edit.h"

#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool source_has_track(base_bytes source, const char *track)
{
    if (!base_bytes_valid(source) || track == NULL) return false;
    size_t length = strlen(track);
    if (length > source.len) return false;
    for (size_t i = 0U; i <= source.len - length; ++i)
        if (memcmp(source.ptr + i, track, length) == 0) return true;
    return false;
}

static bool decode(base_bytes source, project_timeline_view *view)
{
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    return project_timeline_decode(source, &context, view).error == PROJECT_TIMELINE_OK;
}

static bool test_edit_undo_redo_and_branch(void)
{
    static const char initial[] =
        "{ \"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\" : [ ] }";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    project_timeline_edit_limits limits = project_timeline_edit_default_limits();
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context, limits, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(edit != NULL && project_timeline_edit_undo_count(edit) == 0U);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), bytes("Sketch \"A\"")).error ==
        PROJECT_TIMELINE_OK);
    base_bytes after_track = project_timeline_edit_source(edit);
    CHECK(source_has_track(after_track, "\"name\":\"Sketch \\\"A\\\"\""));
    CHECK(after_track.len > bytes(initial).len &&
        memcmp(after_track.ptr, "{ \"schema\"", 10U) == 0 &&
        source_has_track(after_track, "\"tracks\" : ["));

    CHECK(project_timeline_edit_set_onion(edit, &context, 2U, 4U, 0.25).error ==
        PROJECT_TIMELINE_OK);
    base_bytes onion_source = project_timeline_edit_source(edit);
    CHECK(source_has_track(onion_source, "\"before\":2,\"after\":4,\"opacity\":0.25"));
    project_timeline_view onion_view = {0};
    CHECK(decode(onion_source, &onion_view) && onion_view.onion_opacity == 0.25);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(source_has_track(project_timeline_edit_source(edit),
        "\"before\":0,\"after\":0,\"opacity\":1"));
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_onion(edit, &context, 9U, 0U, 0.25).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT);
    CHECK(project_timeline_edit_set_onion(edit, &context, 2U, 4U, NAN).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT);
    CHECK(project_timeline_edit_set_onion(edit, &context, 2U, 4U, 1.01).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT);

    static const char label[] = "pose \"B\"\nline";
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel1"), bytes(label)).error == PROJECT_TIMELINE_OK);
    project_timeline_view view = {0};
    CHECK(decode(project_timeline_edit_source(edit), &view));
    CHECK(view.track_count == 1U && view.item_counts[0] == 2U);
    project_timeline_item_view item;
    CHECK(project_timeline_item_at(&view, 0U, 1U, &item) && item.tick == 12U &&
        item.asset.len == 4U && memcmp(item.asset.ptr, "cel1", 4U) == 0);

    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_redo_count(edit) == 1U);
    CHECK(decode(project_timeline_edit_source(edit), &view) && view.item_counts[0] == 1U);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(decode(project_timeline_edit_source(edit), &view) && view.item_counts[0] == 2U);
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 12U).error ==
        PROJECT_TIMELINE_OK);
    CHECK(decode(project_timeline_edit_source(edit), &view) && view.item_counts[0] == 1U);
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 0U).error !=
        PROJECT_TIMELINE_OK);

    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_redo_count(edit) == 1U);
    base_bytes before_duplicate = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_DUPLICATE);
    base_bytes after_duplicate = project_timeline_edit_source(edit);
    CHECK(before_duplicate.len == after_duplicate.len &&
        memcmp(before_duplicate.ptr, after_duplicate.ptr, before_duplicate.len) == 0 &&
        project_timeline_edit_redo_count(edit) == 1U);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 6U,
        bytes("cel1"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_redo_count(edit) == 0U);
    CHECK(decode(project_timeline_edit_source(edit), &view) && view.item_counts[0] == 3U);
    CHECK(project_timeline_item_at(&view, 0U, 1U, &item) && item.tick == 6U);

    base_bytes before_failure = project_timeline_edit_source(edit);
    size_t undo_before = project_timeline_edit_undo_count(edit);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_DUPLICATE);
    base_bytes after_failure = project_timeline_edit_source(edit);
    CHECK(before_failure.len == after_failure.len &&
        memcmp(before_failure.ptr, after_failure.ptr, before_failure.len) == 0 &&
        project_timeline_edit_undo_count(edit) == undo_before);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_invalid_inputs_and_history_limit(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_edit_limits limits = project_timeline_edit_default_limits();
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context, limits, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("missing"), bytes("cel0"), (base_bytes){NULL, 0U}).error !=
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("bad/id"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_INVALID_VALUE);
    char oversized_id[129];
    memset(oversized_id, 'a', sizeof(oversized_id));
    CHECK(project_timeline_edit_add_drawing_track(edit, &context,
        (base_bytes){(const uint8_t *)oversized_id, sizeof(oversized_id)},
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("missing"), (base_bytes){NULL, 0U}).error !=
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 24U,
        bytes("cel0"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 12U).error ==
        PROJECT_TIMELINE_INVALID_REFERENCE);
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 0U).error ==
        PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("missing"), 1U,
        bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_INVALID_REFERENCE);
    base_bytes preserved = project_timeline_edit_source(edit);
    project_timeline_edit_destroy(edit);

    limits.history_bytes = bytes(initial).len;
    CHECK(project_timeline_edit_create(bytes(initial), &context, limits, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_LIMIT);
    base_bytes current = project_timeline_edit_source(edit);
    CHECK(current.len == bytes(initial).len &&
        memcmp(current.ptr, initial, current.len) == 0 &&
        project_timeline_edit_undo_count(edit) == 0U);
    project_timeline_edit_destroy(edit);

    static const char future[] = "{\"schema\":\"future\",\"version\":2}";
    CHECK(project_timeline_edit_create(bytes(future), &context, limits, &edit).error !=
        PROJECT_TIMELINE_OK && edit == NULL);
    CHECK(project_timeline_edit_create(bytes("{invalid"), &context, limits, &edit).error !=
        PROJECT_TIMELINE_OK && edit == NULL);
    static const base_bytes unsorted_ids[] = {
        {(const uint8_t *)"z", 1U}, {(const uint8_t *)"a", 1U}
    };
    project_timeline_context unsorted_context = {unsorted_ids, 2U, assets, 1U};
    CHECK(project_timeline_edit_create(bytes(initial), &unsorted_context, limits, &edit).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT && edit == NULL);
    uint8_t oversized_asset_name[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES + 1U];
    memset(oversized_asset_name, 'a', sizeof(oversized_asset_name));
    project_timeline_asset_ref oversized_asset = {
        {oversized_asset_name, sizeof(oversized_asset_name)},
        {(const uint8_t *)"image/svg+xml", 13U}
    };
    project_timeline_context oversized_asset_context = {ids, 1U, &oversized_asset, 1U};
    CHECK(project_timeline_edit_create(bytes(initial), &oversized_asset_context,
        limits, &edit).error == PROJECT_TIMELINE_INVALID_ARGUMENT && edit == NULL);
    (void)preserved;
    return true;
}

static bool test_history_eviction_and_invalid_utf8(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    project_timeline_edit_limits limits = {1U, 1024U * 1024U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context, limits, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel1"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_undo_count(edit) == 1U);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    project_timeline_view view = {0};
    CHECK(decode(project_timeline_edit_source(edit), &view));
    CHECK(view.track_count == 1U && view.item_counts[0] == 1U);
    CHECK(project_timeline_edit_redo_count(edit) == 1U);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(decode(project_timeline_edit_source(edit), &view) && view.item_counts[0] == 2U);

    static const project_timeline_context empty_context = {NULL, 0U, NULL, 0U};
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    base_bytes before_failed_redo = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_redo(edit, &empty_context).error != PROJECT_TIMELINE_OK);
    base_bytes after_failed_redo = project_timeline_edit_source(edit);
    CHECK(before_failed_redo.len == after_failed_redo.len &&
        memcmp(before_failed_redo.ptr, after_failed_redo.ptr, before_failed_redo.len) == 0 &&
        project_timeline_edit_redo_count(edit) == 1U);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel1"), bytes("猫")).error == PROJECT_TIMELINE_OK);
    CHECK(source_has_track(project_timeline_edit_source(edit), "\"label\":\"猫\""));

    const uint8_t invalid[] = {0xc0U, 0x80U};
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 6U,
        bytes("cel1"), (base_bytes){invalid, sizeof(invalid)}).error ==
        PROJECT_TIMELINE_INVALID_ARGUMENT);
    CHECK(project_timeline_edit_undo_count(edit) == 1U &&
        project_timeline_edit_redo_count(edit) == 0U);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_source_preserving_splices(void)
{
    static const char source[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\" : [ { \"id\":\"draw\", \"kind\":\"drawing\","
        " \"target\":\"layer\", \"visible\":true, \"cels\" : ["
        " { \"tick\" : 0 , \"asset\" : \"cel0\" } ,  "
        "{\"tick\":12,\"asset\":\"cel1\",\"label\":\"old\"}  ] },"
        " { \"id\":\"other\", \"kind\":\"drawing\", \"target\":\"layer\","
        " \"visible\":true, \"cels\":[{\"tick\":0,\"asset\":\"cel0\"}],"
        " \"name\":\"keep\" } ] }";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(source), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    base_bytes before_zero_remove = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 0U).error !=
        PROJECT_TIMELINE_OK);
    base_bytes after_zero_remove = project_timeline_edit_source(edit);
    CHECK(before_zero_remove.len == after_zero_remove.len &&
        memcmp(before_zero_remove.ptr, after_zero_remove.ptr, before_zero_remove.len) == 0);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel0"), bytes("replacement")).error == PROJECT_TIMELINE_OK);
    base_bytes current = project_timeline_edit_source(edit);
    CHECK(source_has_track(current,
        "\"cels\" : [ { \"tick\" : 0 , \"asset\" : \"cel0\" } ,  "));
    CHECK(source_has_track(current, "  ] },"));
    CHECK(source_has_track(current,
        " { \"id\":\"other\", \"kind\":\"drawing\", \"target\":\"layer\", \"visible\":true, \"cels\":[{\"tick\":0,\"asset\":\"cel0\"}], \"name\":\"keep\" } ] }"));
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 6U,
        bytes("cel1"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    current = project_timeline_edit_source(edit);
    CHECK(source_has_track(current, "{ \"tick\" : 0 , \"asset\" : \"cel0\" }"));
    CHECK(source_has_track(current, "\"label\":\"replacement\""));
    CHECK(project_timeline_edit_remove_cel(edit, &context, bytes("draw"), 6U).error ==
        PROJECT_TIMELINE_OK);
    current = project_timeline_edit_source(edit);
    CHECK(source_has_track(current, "{ \"tick\" : 0 , \"asset\" : \"cel0\" }"));
    CHECK(source_has_track(current, "\"label\":\"replacement\""));
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_byte_limited_history(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}},
        {{(const uint8_t *)"cel1", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    project_timeline_edit_limits defaults = project_timeline_edit_default_limits();
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context, defaults, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    size_t track_length = project_timeline_edit_source(edit).len;
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel1"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    size_t cel_length = project_timeline_edit_source(edit).len;
    project_timeline_edit_destroy(edit);

    project_timeline_edit_limits bounded = {16U, track_length + cel_length};
    CHECK(project_timeline_edit_create(bytes(initial), &context, bounded, &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 12U,
        bytes("cel1"), (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_undo_count(edit) == 1U);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    base_bytes before_failure = project_timeline_edit_source(edit);
    char huge_label[PROJECT_TIMELINE_MAX_STRING_BYTES];
    memset(huge_label, 'x', sizeof(huge_label));
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"), 6U,
        bytes("cel1"), (base_bytes){(const uint8_t *)huge_label,
                                     sizeof(huge_label)}).error == PROJECT_TIMELINE_LIMIT);
    base_bytes after_failure = project_timeline_edit_source(edit);
    CHECK(before_failure.len == after_failure.len &&
        memcmp(before_failure.ptr, after_failure.ptr, before_failure.len) == 0 &&
        project_timeline_edit_redo_count(edit) == 1U);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_track_count_limit(void)
{
    static const char prefix[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[";
    static const char track_format[] =
        "{\"id\":\"track-%u\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"cel0\"}]}";
    char timeline[65536];
    size_t length = sizeof(prefix) - 1U;
    memcpy(timeline, prefix, length);
    for (unsigned int i = 0U; i < PROJECT_TIMELINE_MAX_TRACKS; ++i) {
        if (i != 0U) timeline[length++] = ',';
        int written = snprintf(timeline + length, sizeof(timeline) - length,
            track_format, i);
        CHECK(written > 0 && (size_t)written < sizeof(timeline) - length);
        length += (size_t)written;
    }
    timeline[length++] = ']';
    timeline[length++] = '}';
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create((base_bytes){(const uint8_t *)timeline, length},
        &context, project_timeline_edit_default_limits(), &edit).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("extra"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_LIMIT);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_cel_count_limit(void)
{
    static const char prefix[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":100001,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":[";
    const size_t capacity = 8U * 1024U * 1024U;
    char *timeline = malloc(capacity);
    CHECK(timeline != NULL);
    size_t length = sizeof(prefix) - 1U;
    memcpy(timeline, prefix, length);
    for (uint32_t tick = 0U; tick < PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK - 1U; ++tick) {
        if (tick != 0U) timeline[length++] = ',';
        int written = snprintf(timeline + length, capacity - length,
            "{\"tick\":%u,\"asset\":\"cel0\"}", tick);
        CHECK(written > 0 && (size_t)written < capacity - length);
        length += (size_t)written;
    }
    static const char suffix[] = "]}]}";
    CHECK(sizeof(suffix) - 1U < capacity - length);
    memcpy(timeline + length, suffix, sizeof(suffix) - 1U);
    length += sizeof(suffix) - 1U;
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_edit *edit = NULL;
    project_timeline_edit_result created = project_timeline_edit_create(
        (base_bytes){(const uint8_t *)timeline, length}, &context,
        project_timeline_edit_default_limits(), &edit);
    free(timeline);
    CHECK(created.error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"),
        PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK - 1U, bytes("cel0"),
        (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_OK);
    base_bytes before_failure = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"),
        PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK, bytes("cel0"),
        (base_bytes){NULL, 0U}).error == PROJECT_TIMELINE_LIMIT);
    base_bytes after_failure = project_timeline_edit_source(edit);
    CHECK(before_failure.len == after_failure.len &&
        memcmp(before_failure.ptr, after_failure.ptr, before_failure.len) == 0 &&
        project_timeline_edit_undo_count(edit) == 1U);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("draw"),
        PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK - 1U, bytes("cel0"),
        bytes("replace")).error == PROJECT_TIMELINE_OK);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_aggregate_cel_count_limit(void)
{
    static const char prefix[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":50001,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[";
    const size_t capacity = PROJECT_BUNDLE_MAX_TIMELINE_BYTES;
    char *timeline = malloc(capacity);
    CHECK(timeline != NULL);
    size_t length = sizeof(prefix) - 1U;
    memcpy(timeline, prefix, length);
    for (unsigned int track = 0U; track < 20U; ++track) {
        if (track != 0U) timeline[length++] = ',';
        int written = snprintf(timeline + length, capacity - length,
            "{\"id\":\"track-%u\",\"kind\":\"drawing\","
            "\"target\":\"layer\",\"visible\":true,\"cels\":[", track);
        if (written <= 0 || (size_t)written >= capacity - length) {
            free(timeline);
            CHECK(false);
        }
        length += (size_t)written;
        for (uint32_t tick = 0U; tick < 50000U; ++tick) {
            if (tick != 0U) timeline[length++] = ',';
            written = snprintf(timeline + length, capacity - length,
                "{\"tick\":%u,\"asset\":\"cel0\"}", tick);
            if (written <= 0 || (size_t)written >= capacity - length) {
                free(timeline);
                CHECK(false);
            }
            length += (size_t)written;
        }
        static const char track_end[] = "]}";
        memcpy(timeline + length, track_end, sizeof(track_end) - 1U);
        length += sizeof(track_end) - 1U;
    }
    timeline[length++] = ']';
    timeline[length++] = '}';
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_edit *edit = NULL;
    project_timeline_edit_result created = project_timeline_edit_create(
        (base_bytes){(const uint8_t *)timeline, length}, &context,
        project_timeline_edit_default_limits(), &edit);
    free(timeline);
    CHECK(created.error == PROJECT_TIMELINE_OK);
    base_bytes before_failure = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_set_cel(edit, &context, bytes("track-0"),
        50000U, bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_LIMIT);
    base_bytes after_failure = project_timeline_edit_source(edit);
    CHECK(before_failure.len == after_failure.len &&
        memcmp(before_failure.ptr, after_failure.ptr, before_failure.len) == 0 &&
        project_timeline_edit_undo_count(edit) == 0U);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_timeline_size_limit(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    size_t length = PROJECT_BUNDLE_MAX_TIMELINE_BYTES - 1U;
    uint8_t *timeline = malloc(length);
    CHECK(timeline != NULL && sizeof(initial) - 1U < length);
    memcpy(timeline, initial, sizeof(initial) - 1U);
    memset(timeline + sizeof(initial) - 1U, ' ',
        length - (sizeof(initial) - 1U));
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create((base_bytes){timeline, length}, &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_drawing_track(edit, &context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_LIMIT);
    project_timeline_edit_destroy(edit);
    free(timeline);
    return true;
}

static bool test_owned_source_and_per_call_context(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    size_t input_length = sizeof(initial) - 1U;
    uint8_t *input = malloc(input_length);
    base_bytes *input_ids = malloc(sizeof(*input_ids));
    project_timeline_asset_ref *input_assets = malloc(sizeof(*input_assets));
    if (input == NULL || input_ids == NULL || input_assets == NULL) {
        free(input);
        free(input_ids);
        free(input_assets);
        CHECK(false);
    }
    memcpy(input, initial, input_length);
    *input_ids = bytes("layer");
    *input_assets = (project_timeline_asset_ref){bytes("cel0"), bytes("image/svg+xml")};
    project_timeline_context input_context = {input_ids, 1U, input_assets, 1U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create((base_bytes){input, input_length},
        &input_context, project_timeline_edit_default_limits(), &edit).error ==
        PROJECT_TIMELINE_OK);
    memset(input, 'x', input_length);
    free(input);
    free(input_ids);
    free(input_assets);
    base_bytes owned = project_timeline_edit_source(edit);
    CHECK(owned.len == input_length && memcmp(owned.ptr, initial, input_length) == 0);

    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context call_context = {ids, 1U, assets, 1U};
    CHECK(project_timeline_edit_add_drawing_track(edit, &call_context, bytes("draw"),
        bytes("layer"), bytes("cel0"), (base_bytes){NULL, 0U}).error ==
        PROJECT_TIMELINE_OK);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_property_track_and_key_authoring(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\" : [ ] }";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    static const project_timeline_asset_ref assets[] = {
        {{(const uint8_t *)"cel0", 4U}, {(const uint8_t *)"image/svg+xml", 13U}}
    };
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_property_track(edit, &context,
        bytes("fade"), bytes("layer"), bytes("opacity"), bytes("scalar"),
        bytes("linear"), bytes("1"), bytes("Fade")).error == PROJECT_TIMELINE_OK);
    base_bytes after_add = project_timeline_edit_source(edit);
    CHECK(source_has_track(after_add, "\"tracks\" : ["));
    CHECK(source_has_track(after_add, "\"loop\":\"once\""));
    CHECK(project_timeline_edit_set_property_key(edit, &context, bytes("fade"),
        12U, bytes("0.25")).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_property_key(edit, &context, bytes("fade"),
        6U, bytes("0.5")).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_property_key(edit, &context, bytes("fade"),
        6U, bytes("0.75")).error == PROJECT_TIMELINE_OK);
    project_timeline_view view = {0};
    CHECK(project_timeline_decode(project_timeline_edit_source(edit), &context,
        &view).error == PROJECT_TIMELINE_OK);
    CHECK(view.track_count == 1U);
    project_timeline_item_view key = {0};
    CHECK(project_timeline_item_at(&view, 0U, 0U, &key) && key.tick == 0U);
    CHECK(project_timeline_item_at(&view, 0U, 1U, &key) && key.tick == 6U);
    CHECK(key.value_json.len == 4U && memcmp(key.value_json.ptr, "0.75", 4U) == 0);
    CHECK(project_timeline_item_at(&view, 0U, 2U, &key) && key.tick == 12U);
    base_bytes before_bad = project_timeline_edit_source(edit);
    CHECK(project_timeline_edit_add_property_track(edit, &context,
        bytes("bad"), bytes("layer"), bytes("fill"), bytes("scalar"),
        bytes("linear"), bytes("1"), bytes("Bad")).error !=
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_property_track(edit, &context,
        bytes("fade"), bytes("layer"), bytes("opacity"), bytes("scalar"),
        bytes("linear"), bytes("1"), bytes("Fade")).error ==
        PROJECT_TIMELINE_DUPLICATE);
    base_bytes after_bad = project_timeline_edit_source(edit);
    CHECK(before_bad.len == after_bad.len &&
        memcmp(before_bad.ptr, after_bad.ptr, before_bad.len) == 0);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(project_timeline_edit_source(edit), &context,
        &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_item_at(&view, 0U, 1U, &key) && key.tick == 6U &&
        key.value_json.len == 3U && memcmp(key.value_json.ptr, "0.5", 3U) == 0);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(project_timeline_edit_source(edit), &context,
        &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_item_at(&view, 0U, 1U, &key) && key.tick == 6U &&
        key.value_json.len == 4U && memcmp(key.value_json.ptr, "0.75", 4U) == 0);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    project_timeline_edit_destroy(edit);
    return true;
}


static bool test_loop_mode_is_source_preserving_and_undoable(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\" : \"once\" ,\"onion\":{\"before\":0,"
        "\"after\":0,\"opacity\":1},\"tracks\":[]}";
    project_timeline_context context = {0};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_set_loop(edit, &context,
        PROJECT_TIMELINE_LOOP_PING_PONG).error == PROJECT_TIMELINE_OK);
    base_bytes changed = project_timeline_edit_source(edit);
    static const char expected[] = "\"loop\" : \"ping_pong\"";
    bool preserved_spacing = false;
    for (size_t index = 0U; index + sizeof(expected) - 1U <= changed.len; ++index)
        if (memcmp(changed.ptr + index, expected, sizeof(expected) - 1U) == 0)
            preserved_spacing = true;
    CHECK(preserved_spacing);
    project_timeline_view view = {0};
    CHECK(project_timeline_decode(changed, &context, &view).error ==
        PROJECT_TIMELINE_OK && view.loop_mode == PROJECT_TIMELINE_LOOP_PING_PONG);
    CHECK(project_timeline_edit_set_loop(edit, &context,
        PROJECT_TIMELINE_LOOP_PING_PONG).error == PROJECT_TIMELINE_OK &&
        project_timeline_edit_undo_count(edit) == 1U);
    CHECK(project_timeline_edit_set_loop(edit, &context,
        (project_timeline_loop_mode)99).error == PROJECT_TIMELINE_INVALID_ARGUMENT);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK);
    changed = project_timeline_edit_source(edit);
    CHECK(changed.len == bytes(initial).len &&
        memcmp(changed.ptr, initial, changed.len) == 0);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    project_timeline_edit_destroy(edit);
    return true;
}

static bool test_source_lease_survives_history_eviction(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,"
        "\"after\":0,\"opacity\":1},\"tracks\":[]}";
    project_timeline_context context = {0};
    project_timeline_edit_limits limits = {2U, 1024U * 1024U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context, limits,
        &edit).error == PROJECT_TIMELINE_OK);
    project_timeline_source_lease *lease = NULL;
    CHECK(project_timeline_edit_source_lease_create(edit, &lease).error ==
        PROJECT_TIMELINE_OK);
    base_bytes pinned = project_timeline_source_lease_bytes(lease);
    CHECK(pinned.len == sizeof(initial) - 1U &&
        memcmp(pinned.ptr, initial, pinned.len) == 0);
    for (uint32_t index = 0U; index < 5U; ++index)
        CHECK(project_timeline_edit_set_loop(edit, &context,
            index % 2U == 0U ? PROJECT_TIMELINE_LOOP_PING_PONG :
                PROJECT_TIMELINE_LOOP_ONCE).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_undo_count(edit) == 2U);
    project_timeline_edit_destroy(edit);
    pinned = project_timeline_source_lease_bytes(lease);
    CHECK(pinned.len == sizeof(initial) - 1U &&
        memcmp(pinned.ptr, initial, pinned.len) == 0);
    project_timeline_source_lease_destroy(lease);
    return true;
}

static bool test_property_track_and_key_is_one_history_step(void)
{
    static const char initial[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const base_bytes ids[] = {{(const uint8_t *)"layer", 5U}};
    project_timeline_context context = {ids, 1U, NULL, 0U};
    project_timeline_edit *edit = NULL;
    CHECK(project_timeline_edit_create(bytes(initial), &context,
        project_timeline_edit_default_limits(), &edit).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_add_property_track_key(edit, &context,
        bytes("fade"), bytes("layer"), bytes("opacity"), bytes("scalar"),
        bytes("linear"), bytes("0.75"), 12U, bytes("0.25"),
        bytes("Opacity")).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_edit_undo_count(edit) == 1U);
    project_timeline_view view = {0};
    CHECK(project_timeline_decode(project_timeline_edit_source(edit), &context,
        &view).error == PROJECT_TIMELINE_OK && view.track_count == 1U &&
        view.item_counts[0] == 2U);
    project_timeline_item_view key = {0};
    CHECK(project_timeline_item_at(&view, 0U, 0U, &key) && key.tick == 0U &&
        key.value_json.len == 4U && memcmp(key.value_json.ptr, "0.75", 4U) == 0);
    CHECK(project_timeline_item_at(&view, 0U, 1U, &key) && key.tick == 12U &&
        key.value_json.len == 4U && memcmp(key.value_json.ptr, "0.25", 4U) == 0);
    CHECK(project_timeline_edit_undo(edit, &context).error == PROJECT_TIMELINE_OK &&
        project_timeline_edit_source(edit).len == bytes(initial).len &&
        memcmp(project_timeline_edit_source(edit).ptr, initial, bytes(initial).len) == 0);
    CHECK(project_timeline_edit_redo(edit, &context).error == PROJECT_TIMELINE_OK);
    project_timeline_edit_destroy(edit);
    return true;
}

int main(void)
{
    return test_edit_undo_redo_and_branch() && test_invalid_inputs_and_history_limit() &&
        test_history_eviction_and_invalid_utf8() && test_source_preserving_splices() &&
        test_byte_limited_history() && test_track_count_limit() &&
        test_cel_count_limit() && test_aggregate_cel_count_limit() &&
        test_timeline_size_limit() &&
        test_owned_source_and_per_call_context() &&
        test_property_track_and_key_authoring() &&
        test_property_track_and_key_is_one_history_step() &&
        test_source_lease_survives_history_eviction() &&
        test_loop_mode_is_source_preserving_and_undoable() ?
        EXIT_SUCCESS : EXIT_FAILURE;
}
