#define _DARWIN_C_SOURCE 1
#define _USE_EXTENDED_LOCALES_ 1
#include "project_timeline.h"

#include <locale.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <pthread.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool test_valid_and_borrowed_view(void)
{
    static const char json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"loop\","
        "\"onion\":{\"before\":1,\"after\":2,\"opacity\":0.35},"
        "\"tracks\":[{\"id\":\"walk\",\"kind\":\"drawing\",\"target\":\"layer1\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    base_bytes ids[] = {bytes("layer1")};
    project_timeline_asset_ref assets[] = {
        {bytes("cel0"), bytes("image/svg+xml")},
        {bytes("provenance.png"), bytes("image/png")}
    };
    project_timeline_context context = {ids, 1U, assets, 2U};
    uint8_t before[sizeof(json)];
    memcpy(before, json, sizeof(json));
    project_timeline_view view;
    project_timeline_result decoded = project_timeline_decode(bytes(json), &context, &view);
    if (decoded.error != PROJECT_TIMELINE_OK)
        (void)fprintf(stderr, "decode: %s at %zu\n", project_timeline_error_name(decoded.error), decoded.error_offset);
    CHECK(decoded.error == PROJECT_TIMELINE_OK);
    CHECK(view.track_count == 1U && view.ticks_per_second == 24U && view.duration_ticks == 240U);
    CHECK(view.loop_mode == PROJECT_TIMELINE_LOOP_REPEAT);
    CHECK(view.onion_before == 1U && view.onion_after == 2U &&
        view.onion_opacity == 0.35);
    project_timeline_track_view track;
    CHECK(project_timeline_track_at(&view, 0U, &track));
    CHECK(track.item_count == 1U && track.visible);
    CHECK(track.id.len == 4U && memcmp(track.id.ptr, "walk", 4U) == 0);
    project_timeline_item_view item;
    CHECK(project_timeline_item_at(&view, 0U, 0U, &item));
    CHECK(item.tick == 0U && item.asset.len == 4U);
    CHECK(item.source_offset < sizeof(json) && item.source_length > 0U);
    CHECK(item.source_offset + item.source_length <= sizeof(json));
    CHECK(memcmp(json + item.source_offset, "{\"tick\":0,\"asset\":\"cel0\"}",
                 item.source_length) == 0);
    CHECK(memcmp(before, json, sizeof(json)) == 0);
    return true;
}

static bool test_property_track(void)
{
    static const char json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"opacity\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":1},"
        "{\"tick\":12,\"value\":0.25}]}]}";
    base_bytes ids[] = {bytes("shape1")};
    project_timeline_context context = {ids, 1U, NULL, 0U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(json), &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_track_view track;
    CHECK(project_timeline_track_at(&view, 0U, &track) && track.item_count == 2U);
    CHECK(memcmp(track.kind.ptr, "property", 8U) == 0);
    project_timeline_item_view item;
    project_timeline_iterator iterator = {0};
    CHECK(project_timeline_item_next(&view, 0U, &iterator, &item));
    CHECK(item.tick == 0U && iterator.next_index == 1U);
    CHECK(project_timeline_item_next(&view, 0U, &iterator, &item));
    CHECK(item.tick == 12U && item.value_json.len == 4U);
    CHECK(!project_timeline_item_next(&view, 0U, &iterator, &item));
    return true;
}

static bool test_escaped_strings_and_surrogates(void)
{
    static const char escaped[] =
        "{\"sch\\u0065ma\":\"vexel.animati\\u006fn\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"wa\\u006bk\",\"kind\":\"draw\\u0069ng\","
        "\"target\":\"lay\\u0065r1\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"ce\\u006c0\"}]}]}";
    static const char bad_surrogate[] = "\"\\uD800\\u0041\"";
    static const char good_surrogate[] = "\"\\uD83D\\uDE00\"";
    base_bytes ids[] = {bytes("layer1")};
    project_timeline_asset_ref assets[] = {{bytes("cel0"), bytes("image/svg+xml")}};
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(escaped), &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes(bad_surrogate), &context, &view).error == PROJECT_TIMELINE_MALFORMED_JSON);
    uint8_t output[16]; size_t length = 0U;
    CHECK(project_timeline_string_copy(bytes("\\uD83D\\uDE00"),
        (base_mut_bytes){output, sizeof(output)}, &length));
    CHECK(length == 4U);
    CHECK(project_timeline_decode(bytes(good_surrogate), &context, &view).error == PROJECT_TIMELINE_INVALID_SCHEMA);
    CHECK(project_timeline_string_copy(bytes("lay\\u0065r1"),
        (base_mut_bytes){output, sizeof(output)}, &length));
    CHECK(length == 6U && memcmp(output, "layer1", 6U) == 0);
    locale_t french = newlocale(LC_NUMERIC_MASK, "fr_FR.UTF-8", (locale_t)0);
    CHECK(french != (locale_t)0);
    locale_t prior = uselocale(french);
    CHECK(prior != (locale_t)0);
    static const char decimal[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":0.35},\"tracks\":[]}";
    CHECK(project_timeline_decode(bytes(decimal), &context, &view).error == PROJECT_TIMELINE_OK);
    (void)uselocale(prior);
    freelocale(french);
    return true;
}

static bool test_rgba_and_order_validation(void)
{
    static const char rgba_json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"color\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"fill\",\"value_type\":\"rgba\","
        "\"interpolation\":\"step\",\"keys\":[{\"tick\":0,\"value\":[1,0,0,1]}]}]}";
    static const char bad_ticks[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"opacity\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":1},"
        "{\"tick\":0,\"value\":0}]}]}";
    static const char bad_rgba[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"color\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"fill\",\"value_type\":\"rgba\","
        "\"interpolation\":\"step\",\"keys\":[{\"tick\":0,\"value\":[1.1,0,0,1]}]}]}";
    static const char wrong_pair[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"bad\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"fill\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":1}]}]}";
    static const char transform[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"move\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"transform\",\"value_type\":\"transform2d\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":[1,0,0,1,2,3]}]}]}";
    static const char boolean_linear[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"visible\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"visibility\",\"value_type\":\"boolean\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":true}]}]}";
    base_bytes ids[] = {bytes("shape1")};
    project_timeline_context context = {ids, 1U, NULL, 0U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(rgba_json), &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes(bad_rgba), &context, &view).error == PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_decode(bytes(wrong_pair), &context, &view).error == PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(project_timeline_decode(bytes(transform), &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes(boolean_linear), &context, &view).error == PROJECT_TIMELINE_INVALID_VALUE);
    static const char boolean_step[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"visible\",\"kind\":\"property\",\"target\":\"shape1\","
        "\"visible\":true,\"property\":\"visibility\",\"value_type\":\"boolean\","
        "\"interpolation\":\"step\",\"keys\":[{\"tick\":0,\"value\":true}]}]}";
    CHECK(project_timeline_decode(bytes(boolean_step), &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes(bad_ticks), &context, &view).error == PROJECT_TIMELINE_DUPLICATE);
    return true;
}

static bool test_numeric_overflow_and_item_offset(void)
{
    static const char fps_overflow[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":4294967296,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},\"tracks\":[]}";
    static const char late_tick[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"v\",\"kind\":\"property\",\"target\":\"shape\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":24,\"value\":0.5}]}]}";
    project_timeline_context context = {NULL, 0U, NULL, 0U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(fps_overflow), &context, &view).error == PROJECT_TIMELINE_LIMIT);
    base_bytes ids[] = {bytes("shape")};
    context = (project_timeline_context){ids, 1U, NULL, 0U};
    project_timeline_result decoded = project_timeline_decode(bytes(late_tick), &context, &view);
    const char *tick = strstr(late_tick, "\"tick\":24") + strlen("\"tick\":");
    CHECK(decoded.error == PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(decoded.error_offset == (size_t)(tick - late_tick));
    return true;
}

static bool test_rejections_and_empty(void)
{
    static const char valid[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"loop\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},\"tracks\":[]}";
    base_bytes ids[] = {bytes("shape")};
    project_timeline_context context = {ids, 1U, NULL, 0U};
    project_timeline_view view;
    CHECK(project_timeline_decode((base_bytes){NULL, 0U}, &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(view.empty);
    char changed[sizeof(valid)];
    memcpy(changed, valid, sizeof(valid));
    char *version = strstr(changed, "\"version\":1");
    CHECK(version != NULL); version[10] = '2';
    CHECK(project_timeline_decode(bytes(changed), &context, &view).error == PROJECT_TIMELINE_UNSUPPORTED_VERSION);
    memcpy(changed, valid, sizeof(valid));
    char *loop = strstr(changed, "\"loop\":\"loop\"");
    CHECK(loop != NULL); loop[8] = 'x';
    CHECK(project_timeline_decode(bytes(changed), &context, &view).error != PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes("{\"schema\":"), &context, &view).error == PROJECT_TIMELINE_MALFORMED_JSON);
    CHECK(project_timeline_decode(bytes("{\"schema\":\"future.timeline\",\"version\":2}"),
        &context, &view).error == PROJECT_TIMELINE_UNSUPPORTED_VERSION);
    CHECK(project_timeline_decode(bytes("{\"schema\":\"future.timeline\",\"version\":2,"
        "\"extra\":{\"a\":1,\"a\":2}}"), &context, &view).error == PROJECT_TIMELINE_DUPLICATE);
    CHECK(project_timeline_decode(bytes("{\"schema\" : \"vexel.animation\" , \"version\" : 1 ,"
        "\"ticks_per_second\" : 24 , \"duration_ticks\" : 24 ,"
        "\"fps\" : {\"numerator\" : 24 , \"denominator\" : 1},"
        "\"loop\" : \"once\" , \"onion\" : {\"before\" : 0, \"after\" : 0, \"opacity\" : 1},"
        "\"tracks\" : [] }"), &context, &view).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_decode(bytes("{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"num\\u0065rator\":12},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},\"tracks\":[]}"),
        &context, &view).error == PROJECT_TIMELINE_DUPLICATE);
    static const char duplicate[] =
        "{\"schema\":\"vexel.animation\",\"schema\":\"vexel.animation\","
        "\"version\":1,\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},\"tracks\":[]}";
    CHECK(project_timeline_decode(bytes(duplicate), &context, &view).error == PROJECT_TIMELINE_DUPLICATE);
    uint8_t oversized = 0U;
    CHECK(project_timeline_decode((base_bytes){&oversized,
        PROJECT_BUNDLE_MAX_TIMELINE_BYTES + 1U}, &context, &view).error == PROJECT_TIMELINE_LIMIT);
    uint8_t deep[40];
    memset(deep, '[', sizeof(deep));
    CHECK(project_timeline_decode((base_bytes){deep, sizeof(deep)}, &context, &view).error == PROJECT_TIMELINE_LIMIT);
    return true;
}

static bool test_invalid_references_and_duplicates(void)
{
    static const char json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},\"loop\":\"loop\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"drawing\",\"target\":\"missing\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"nope\"}]}]}";
    project_timeline_context context = {NULL, 0U, NULL, 0U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(json), &context, &view).error == PROJECT_TIMELINE_INVALID_REFERENCE);
    project_timeline_asset_ref unsafe_assets[] = {{bytes("../cel"), bytes("image/svg+xml")}};
    context = (project_timeline_context){NULL, 0U, unsafe_assets, 1U};
    CHECK(project_timeline_decode(bytes("{}"), &context, &view).error == PROJECT_TIMELINE_INVALID_ARGUMENT);
    project_timeline_asset_ref invalid_assets[] = {{{NULL, 1U}, bytes("image/svg+xml")}};
    context = (project_timeline_context){NULL, 0U, invalid_assets, 1U};
    CHECK(project_timeline_decode(bytes("{}"), &context, &view).error == PROJECT_TIMELINE_INVALID_ARGUMENT);
    static const char unknown_property[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":240,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"property\",\"target\":\"shape\","
        "\"visible\":true,\"property\":\"not-a-supported-property\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":0}]}]}";
    base_bytes shape_ids[] = {bytes("shape")};
    context = (project_timeline_context){shape_ids, 1U, NULL, 0U};
    CHECK(project_timeline_decode(bytes(unknown_property), &context, &view).error == PROJECT_TIMELINE_INVALID_VALUE);
    static const char missing_asset[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":240,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"drawing\",\"target\":\"shape\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"missing\"}]}]}";
    base_bytes valid_shape[] = {bytes("shape")};
    context = (project_timeline_context){valid_shape, 1U, NULL, 0U};
    CHECK(project_timeline_decode(bytes(missing_asset), &context, &view).error == PROJECT_TIMELINE_INVALID_REFERENCE);
    base_bytes dup_ids[] = {bytes("shape"), bytes("shape")};
    context = (project_timeline_context){dup_ids, 2U, NULL, 0U};
    CHECK(project_timeline_decode(bytes("{}"), &context, &view).error == PROJECT_TIMELINE_DUPLICATE);
    project_timeline_asset_ref invalid_empty_asset[] = {{bytes("../bad"), bytes("image/svg+xml")}};
    context = (project_timeline_context){NULL, 0U, invalid_empty_asset, 1U};
    CHECK(project_timeline_decode((base_bytes){NULL, 0U}, &context, &view).error == PROJECT_TIMELINE_INVALID_ARGUMENT);
    return true;
}

static bool test_labeled_cel(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"d\",\"kind\":\"drawing\",\"target\":\"shape\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"cel\",\"label\":\"key pose\"}]}]}";
    base_bytes ids[] = {bytes("shape")};
    project_timeline_asset_ref assets[] = {{bytes("cel"), bytes("image/svg+xml")}};
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(timeline), &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_item_view item;
    CHECK(project_timeline_item_at(&view, 0U, 0U, &item));
    CHECK(item.source_length > 0U && item.asset.len == 3U);
    char escaped_label_timeline[7500];
    const char *label_prefix = "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"d\",\"kind\":\"drawing\",\"target\":\"shape\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"cel\",\"label\":\"";
    size_t label_length = strlen(label_prefix);
    memcpy(escaped_label_timeline, label_prefix, label_length);
    for (size_t i = 0U; i < 1000U; i++) {
        memcpy(escaped_label_timeline + label_length + i * 6U, "\\u0061", 6U);
    }
    size_t label_end = label_length + 6000U;
    memcpy(escaped_label_timeline + label_end, "\"}]}]}", 6U);
    CHECK(project_timeline_decode((base_bytes){(const uint8_t *)escaped_label_timeline,
        label_end + 6U}, &context, &view).error == PROJECT_TIMELINE_OK);
    return true;
}

static bool test_duplicate_track_and_item_string_limits(void)
{
    static const char duplicate_track[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"same\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"a\"}]},"
        "{\"id\":\"same\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"a\"}]}]}";
    base_bytes ids[] = {bytes("layer")};
    project_timeline_asset_ref assets[] = {{bytes("a"), bytes("image/svg+xml")}};
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_view view;
    CHECK(project_timeline_decode(bytes(duplicate_track), &context, &view).error == PROJECT_TIMELINE_DUPLICATE);

    char large_string[4200];
    size_t offset = 0U;
    const char *large_prefix = "{\"schema\":\"";
    size_t large_prefix_length = strlen(large_prefix);
    memcpy(large_string + offset, large_prefix, large_prefix_length); offset += large_prefix_length;
    memset(large_string + offset, 'x', 4097U); offset += 4097U;
    memcpy(large_string + offset, "\"}", 2U); offset += 2U;
    CHECK(project_timeline_decode((base_bytes){(const uint8_t *)large_string, offset},
        &context, &view).error == PROJECT_TIMELINE_LIMIT);

    const char *prefix = "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":14400000,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"property\",\"target\":\"layer\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[";
    size_t prefix_length = strlen(prefix);
    size_t cap = prefix_length + 32U * (PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK + 1U) + 4U;
    char *many = malloc(cap);
    CHECK(many != NULL);
    memcpy(many, prefix, prefix_length);
    offset = prefix_length;
    for (size_t i = 0U; i <= PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK; i++) {
        int written = snprintf(many + offset, cap - offset,
            "{\"tick\":%zu,\"value\":1},", i);
        CHECK(written > 0 && (size_t)written < cap - offset);
        offset += (size_t)written;
    }
    many[offset - 1U] = ']';
    many[offset++] = '}';
    many[offset++] = ']';
    many[offset++] = '}';
    project_timeline_result too_many = project_timeline_decode(
        (base_bytes){(const uint8_t *)many, offset}, &context, &view);
    free(many);
    if (too_many.error != PROJECT_TIMELINE_LIMIT)
        (void)fprintf(stderr, "many items: %s at %zu\n",
            project_timeline_error_name(too_many.error), too_many.error_offset);
    CHECK(too_many.error == PROJECT_TIMELINE_LIMIT);

    const char *valid_prefix = "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":14400000,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"property\",\"target\":\"layer\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[";
    size_t valid_prefix_length = strlen(valid_prefix);
    size_t valid_capacity = valid_prefix_length + 32U * PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK + 4U;
    char *valid_many = malloc(valid_capacity);
    CHECK(valid_many != NULL);
    memcpy(valid_many, valid_prefix, valid_prefix_length);
    size_t valid_offset = valid_prefix_length;
    for (size_t i = 0U; i < PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK; i++) {
        int written = snprintf(valid_many + valid_offset, valid_capacity - valid_offset,
            "{\"tick\":%zu,\"value\":1}%s", i, i + 1U == PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK ? "" : ",");
        CHECK(written > 0 && (size_t)written < valid_capacity - valid_offset);
        valid_offset += (size_t)written;
    }
    memcpy(valid_many + valid_offset, "]}]}", 4U); valid_offset += 4U;
    base_bytes layer_ids[] = {bytes("layer")};
    context = (project_timeline_context){layer_ids, 1U, NULL, 0U};
    CHECK(project_timeline_decode((base_bytes){(const uint8_t *)valid_many, valid_offset},
        &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_iterator sequential_iterator = {0};
    project_timeline_item_view sequential_item;
    size_t traversed = 0U;
    uint32_t expected_tick = 0U;
    while (project_timeline_item_next(&view, 0U, &sequential_iterator,
            &sequential_item)) {
        CHECK(sequential_item.tick == expected_tick);
        CHECK(sequential_item.source_length > 0U);
        CHECK(sequential_item.source_offset + sequential_item.source_length <= valid_offset);
        expected_tick++;
        traversed++;
    }
    free(valid_many);
    CHECK(traversed == PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK);
    static const char two_tracks[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"a\",\"kind\":\"property\",\"target\":\"layer\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":1}]},"
        "{\"id\":\"b\",\"kind\":\"property\",\"target\":\"layer\","
        "\"visible\":true,\"property\":\"opacity\",\"value_type\":\"scalar\","
        "\"interpolation\":\"linear\",\"keys\":[{\"tick\":0,\"value\":0}]}]}";
    CHECK(project_timeline_decode(bytes(two_tracks), &context, &view).error == PROJECT_TIMELINE_OK);
    project_timeline_iterator mismatch_iterator = {0};
    project_timeline_item_view ignored_item;
    CHECK(project_timeline_item_next(&view, 0U, &mismatch_iterator, &ignored_item));
    CHECK(!project_timeline_item_next(&view, 1U, &mismatch_iterator, &ignored_item));
    return true;
}

static bool test_error_offsets_and_unknown_fields(void)
{
    static const char invalid_time[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":0,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},\"tracks\":[]}";
    static const char unknown_field[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[],\"future_field\":true}";
    uint8_t malformed_utf8[] = {'"', 0xc0U, 0x80U, '"'};
    project_timeline_context context = {NULL, 0U, NULL, 0U};
    project_timeline_view view;
    project_timeline_result result_value = project_timeline_decode(bytes(invalid_time), &context, &view);
    const char *value = strstr(invalid_time, "\"ticks_per_second\":0") + strlen("\"ticks_per_second\":");
    CHECK(result_value.error == PROJECT_TIMELINE_INVALID_VALUE);
    CHECK(result_value.error_offset == (size_t)(value - invalid_time));
    CHECK(project_timeline_decode(bytes(unknown_field), &context, &view).error == PROJECT_TIMELINE_INVALID_SCHEMA);
    CHECK(project_timeline_decode((base_bytes){malformed_utf8, sizeof(malformed_utf8)},
        &context, &view).error == PROJECT_TIMELINE_MALFORMED_JSON);
    return true;
}

typedef struct small_stack_test_context { bool passed; } small_stack_test_context;

static void *small_stack_test(void *opaque)
{
    small_stack_test_context *state = opaque;
    static const char json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,\"ticks_per_second\":24,"
        "\"duration_ticks\":24,\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"f\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"a\"}]}]}";
    base_bytes ids[] = {bytes("layer")};
    project_timeline_asset_ref assets[] = {{bytes("a"), bytes("image/svg+xml")}};
    project_timeline_context context = {ids, 1U, assets, 1U};
    project_timeline_view view;
    project_timeline_track_view track;
    project_timeline_item_view item;
    state->passed = project_timeline_decode(bytes(json), &context, &view).error == PROJECT_TIMELINE_OK &&
        project_timeline_track_at(&view, 0U, &track) &&
        project_timeline_item_at(&view, 0U, 0U, &item) && item.tick == 0U;
    return NULL;
}

static bool test_small_stack(void)
{
    pthread_attr_t attr;
    CHECK(pthread_attr_init(&attr) == 0);
    CHECK(pthread_attr_setstacksize(&attr, 128U * 1024U) == 0);
    pthread_t thread;
    small_stack_test_context context = {false};
    CHECK(pthread_create(&thread, &attr, small_stack_test, &context) == 0);
    (void)pthread_attr_destroy(&attr);
    CHECK(pthread_join(thread, NULL) == 0);
    CHECK(context.passed);
    return true;
}

int main(void)
{
    if (!test_valid_and_borrowed_view() || !test_property_track() || !test_labeled_cel() ||
        !test_rgba_and_order_validation() || !test_escaped_strings_and_surrogates() ||
        !test_numeric_overflow_and_item_offset() ||
        !test_rejections_and_empty() || !test_invalid_references_and_duplicates() ||
        !test_duplicate_track_and_item_string_limits() ||
        !test_error_offsets_and_unknown_fields() || !test_small_stack()) return 1;
    (void)puts("project timeline tests passed");
    return 0;
}
