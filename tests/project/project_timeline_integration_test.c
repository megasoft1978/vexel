#include "project_timeline_integration.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static base_bytes bytes(const char *value)
{
    return (base_bytes){(const uint8_t *)value, strlen(value)};
}

static project_timeline_integration_result validate(const char *scene,
    const char *timeline, const project_bundle_asset *assets, size_t asset_count,
    svg_xml_cancel_check_fn canceled, void *cancel_context,
    project_timeline_view *view)
{
    static const uint8_t manifest[] = "{}";
    project_bundle_input input = {bytes("{}"), bytes(scene), bytes(timeline),
        assets, asset_count};
    project_bundle_result size = project_bundle_encoded_size(&input);
    if (size.error != PROJECT_BUNDLE_OK) return (project_timeline_integration_result){
        .validation = {PROJECT_TIMELINE_INVALID_ARGUMENT, 0U}};
    uint8_t *encoded = malloc(size.encoded_size);
    if (encoded == NULL) return (project_timeline_integration_result){
        .validation = {PROJECT_TIMELINE_LIMIT, 0U}};
    project_bundle_result result = project_bundle_encode(&input,
        (base_mut_bytes){encoded, size.encoded_size});
    project_bundle_view bundle = {0};
    if (result.error != PROJECT_BUNDLE_OK ||
        project_bundle_decode((base_bytes){encoded, size.encoded_size},
                              &bundle).error != PROJECT_BUNDLE_OK) {
        free(encoded);
        return (project_timeline_integration_result){
            .validation = {PROJECT_TIMELINE_INVALID_ARGUMENT, 0U}};
    }
    project_timeline_integration_result checked = project_timeline_validate_bundle(
        &bundle, bytes(scene), canceled, cancel_context, view);
    if (checked.compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE &&
        (bundle.timeline_json.len != strlen(timeline) ||
         memcmp(bundle.timeline_json.ptr, timeline, bundle.timeline_json.len) != 0)) {
        checked.validation.error = PROJECT_TIMELINE_INVALID_VALUE;
    }
    free(encoded);
    (void)manifest;
    return checked;
}

static bool always_cancel(void *context)
{
    (void)context;
    return true;
}

typedef struct cancel_counter {
    size_t calls;
    size_t cancel_at;
} cancel_counter;

static bool count_cancellation_checks(void *context)
{
    cancel_counter *counter = context;
    counter->calls++;
    return false;
}

static bool cancel_after_second_pass_starts(void *context)
{
    cancel_counter *counter = context;
    counter->calls++;
    return counter->calls >= counter->cancel_at;
}

static bool cancel_on_long_json_scan(void *context)
{
    cancel_counter *counter = context;
    counter->calls++;
    return counter->calls >= counter->cancel_at;
}

static bool test_native_sorted_ids_and_assets(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer&1\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t svg_asset[] =
        "<svg width='16' height='16'><path d='M0 0L1 1'/></svg>";
    static const uint8_t png_asset[] = {0x89U, 'P', 'N', 'G'};
    static const uint8_t svg_name[] = "cel0", png_name[] = "source";
    static const uint8_t svg_type[] = "image/svg+xml", png_type[] = "image/png";
    project_bundle_asset assets[] = {
        {{svg_name, sizeof(svg_name) - 1U}, {svg_type, sizeof(svg_type) - 1U},
            {svg_asset, sizeof(svg_asset) - 1U}},
        {{png_name, sizeof(png_name) - 1U}, {png_type, sizeof(png_type) - 1U},
            {png_asset, sizeof(png_asset)}}
    };
    static const char scene[] =
        "<svg><g id='z'/><path id='layer&amp;1'/><rect id='a'/></svg>";
    project_timeline_view view = {0};
    project_timeline_integration_result result = validate(scene, timeline,
        assets, 2U, NULL, NULL, &view);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK);
    CHECK(result.compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1);
    CHECK(view.track_count == 1U && view.source.len == strlen(timeline));
    return true;
}

static bool test_duplicate_and_dangling_ids(void)
{
    static const char empty_v1[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    static const char duplicate_scene[] =
        "<svg><g id='a&amp;b'/><rect id='a&#38;b'/></svg>";
    project_timeline_integration_result result = validate(duplicate_scene,
        empty_v1, NULL, 0U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_DUPLICATE);
    CHECK(result.validation.error_offset == (size_t)(strstr(duplicate_scene,
        "id='a&#38;b'") - duplicate_scene) + 4U);

    static const char dangling[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"p\",\"kind\":\"property\","
        "\"target\":\"missing\",\"visible\":true,\"property\":\"opacity\","
        "\"value_type\":\"scalar\",\"interpolation\":\"linear\","
        "\"keys\":[{\"tick\":0,\"value\":1}]}]}";
    result = validate("<svg><g id='present'/></svg>", dangling,
        NULL, 0U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_INVALID_REFERENCE);
    return true;
}

static bool test_unknown_opaque_and_cancel(void)
{
    static const char unknown[] = "{\"version\":1,\"future\":[1,2]}";
    static const char duplicate_scene[] =
        "<svg><g id='same'/><rect id='same'/></svg>";
    project_timeline_integration_result result = validate(duplicate_scene,
        unknown, NULL, 0U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK);
    CHECK(result.compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE);

    static const char broad_future[] =
        "{\"schema\":\"future.timeline\",\"value\":1e400,"
        "\"a0\":0,\"a1\":1,\"a2\":2,\"a3\":3,\"a4\":4,"
        "\"a5\":5,\"a6\":6,\"a7\":7,\"a8\":8,\"a9\":9,"
        "\"a10\":10,\"a11\":11,\"a12\":12,\"a13\":13,"
        "\"a14\":14,\"a15\":15,\"a16\":16,\"a17\":17}";
    result = validate(duplicate_scene, broad_future, NULL, 0U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK &&
        result.compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE);
    result = validate("<svg/>", "{\"schema\":\"future\",", NULL, 0U,
        NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_MALFORMED_JSON);

    static const char known[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    result = validate("<svg><g>", known, NULL, 0U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_INVALID_SCHEMA &&
        result.validation.error_offset == strlen("<svg><g>"));
    result = validate("<svg><g id='x'/></svg>", known, NULL, 0U,
        always_cancel, NULL, NULL);
    CHECK(result.canceled && result.validation.error != PROJECT_TIMELINE_OK);
    const char prefix[] = "{\"schema\":\"future\",\"payload\":\"";
    char *large_future = malloc(sizeof(prefix) + 10003U);
    CHECK(large_future != NULL);
    size_t prefix_length = sizeof(prefix) - 1U;
    memcpy(large_future, prefix, prefix_length);
    memset(large_future + prefix_length, 'x', 10000U);
    memcpy(large_future + prefix_length + 10000U, "\"}", 2U);
    large_future[prefix_length + 10002U] = '\0';
    cancel_counter json_cancel = {0U, 1U};
    result = validate("<svg/>", large_future, NULL, 0U,
        cancel_on_long_json_scan, &json_cancel, NULL);
    free(large_future);
    CHECK(result.canceled && json_cancel.calls >= 1U);
    cancel_counter count = {0U, SIZE_MAX};
    result = validate("<svg><g id='x'/></svg>", known, NULL, 0U,
        count_cancellation_checks, &count, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK && count.calls >= 2U);
    cancel_counter second_pass = {0U, count.calls / 2U + 1U};
    result = validate("<svg><g id='x'/></svg>", known, NULL, 0U,
        cancel_after_second_pass_starts, &second_pass, NULL);
    CHECK(result.canceled && result.validation.error != PROJECT_TIMELINE_OK);
    return true;
}

static bool test_asset_media_mismatch(void)
{
    static const char drawing[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t data[] = "not svg";
    static const uint8_t name[] = "cel0", media[] = "image/png";
    project_bundle_asset asset = {{name, sizeof(name) - 1U},
        {media, sizeof(media) - 1U}, {data, sizeof(data) - 1U}};
    project_timeline_integration_result result = validate(
        "<svg><g id='layer'/></svg>", drawing, &asset, 1U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_INVALID_REFERENCE);
    return true;
}

static bool test_drawing_asset_content_validation(void)
{
    static const char repeated_cels[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"cel0\"},"
        "{\"tick\":12,\"asset\":\"cel0\"}]}]}";
    static const uint8_t name[] = "cel0", media[] = "image/svg+xml";
    static const uint8_t valid[] =
        "<svg width='16' height='16'><defs><path id='p' d='M0 0L1 1'/></defs>"
        "<use href='#p'/></svg>";
    project_bundle_asset asset = {{name, sizeof(name) - 1U},
        {media, sizeof(media) - 1U}, {valid, sizeof(valid) - 1U}};
    project_timeline_integration_result result = validate(
        "<svg><g id='layer'/></svg>", repeated_cels, &asset, 1U,
        NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK);
    static const uint8_t cdata_style[] =
        "<svg width='16' height='16'><style><![CDATA[/* & */]]></style></svg>";
    project_drawing_asset_result cdata_result = project_drawing_asset_validate(
        (base_bytes){cdata_style, sizeof(cdata_style) - 1U}, NULL, NULL, NULL);
    CHECK(cdata_result.error == SVG_SHAPE_ERROR_NONE);
    result = validate("<svg><g id='layer'/></svg>", repeated_cels,
        &asset, 1U, always_cancel, NULL, NULL);
    CHECK(result.canceled && result.validation.error != PROJECT_TIMELINE_OK);
    cancel_counter stop_asset_scan = {0U, 2U};
    bool asset_canceled = false;
    project_drawing_asset_result asset_result = project_drawing_asset_validate(
        (base_bytes){valid, sizeof(valid) - 1U},
        cancel_after_second_pass_starts, &stop_asset_scan, &asset_canceled);
    CHECK(asset_canceled && asset_result.error == SVG_SHAPE_ERROR_LIMIT);
    svg_shape_document token_canceled_document = {0};
    svg_shape_diagnostic token_canceled_diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    bool token_canceled = false;
    CHECK(svg_shape_decode_cancellable(
        (base_bytes){valid, sizeof(valid) - 1U}, always_cancel, NULL,
        &token_canceled, &token_canceled_document,
        &token_canceled_diagnostic) == SVG_SHAPE_ERROR_LIMIT);
    CHECK(token_canceled && token_canceled_document.edges == NULL);
    svg_shape_document_destroy(&token_canceled_document);

    uint8_t traversal_svg[8192];
    size_t traversal_length = 0U;
    static const char prefix[] = "<svg width='16' height='16'>";
    static const char shape[] = "<rect x='0' y='0' width='1' height='1'/>";
    static const char suffix[] = "</svg>";
    memcpy(traversal_svg, prefix, sizeof(prefix) - 1U);
    traversal_length = sizeof(prefix) - 1U;
    for (size_t i = 0U; i < 100U; ++i) {
        memcpy(traversal_svg + traversal_length, shape, sizeof(shape) - 1U);
        traversal_length += sizeof(shape) - 1U;
    }
    memcpy(traversal_svg + traversal_length, suffix, sizeof(suffix) - 1U);
    traversal_length += sizeof(suffix) - 1U;
    cancel_counter traversal_baseline = {0U, 0U};
    svg_shape_document baseline_document = {0};
    svg_shape_diagnostic baseline_diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    CHECK(svg_shape_decode_cancellable(
        (base_bytes){traversal_svg, traversal_length},
        count_cancellation_checks, &traversal_baseline, NULL,
        &baseline_document, &baseline_diagnostic) == SVG_SHAPE_ERROR_NONE);
    svg_shape_document_destroy(&baseline_document);
    CHECK(traversal_baseline.calls > 10U);
    cancel_counter stop_traversal = {0U, traversal_baseline.calls - 10U};
    bool traversal_canceled = false;
    svg_shape_document canceled_document = {0};
    svg_shape_diagnostic canceled_diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_error canceled_error = svg_shape_decode_cancellable(
        (base_bytes){traversal_svg, traversal_length},
        cancel_after_second_pass_starts, &stop_traversal,
        &traversal_canceled, &canceled_document, &canceled_diagnostic);
    CHECK(traversal_canceled && canceled_error == SVG_SHAPE_ERROR_LIMIT &&
        canceled_document.edges == NULL && canceled_document.shape_count == 0U);
    svg_shape_document_destroy(&canceled_document);

    cancel_counter post_animation_stop = {0U, traversal_baseline.calls};
    bool post_animation_canceled = false;
    svg_shape_document post_animation_document = {0};
    CHECK(svg_shape_decode_cancellable(
        (base_bytes){traversal_svg, traversal_length},
        cancel_after_second_pass_starts, &post_animation_stop,
        &post_animation_canceled, &post_animation_document,
        &canceled_diagnostic) == SVG_SHAPE_ERROR_LIMIT);
    CHECK(post_animation_canceled && post_animation_document.edges == NULL);
    svg_shape_document_destroy(&post_animation_document);

    uint8_t whitespace_svg[8192];
    static const char whitespace_prefix[] = "<svg width='16' height='16'>";
    static const char whitespace_suffix[] = "</svg>";
    memcpy(whitespace_svg, whitespace_prefix, sizeof(whitespace_prefix) - 1U);
    size_t whitespace_length = sizeof(whitespace_prefix) - 1U;
    memset(whitespace_svg + whitespace_length, ' ', 4096U);
    whitespace_length += 4096U;
    memcpy(whitespace_svg + whitespace_length, whitespace_suffix,
        sizeof(whitespace_suffix) - 1U);
    whitespace_length += sizeof(whitespace_suffix) - 1U;
    cancel_counter whitespace_baseline = {0U, 0U};
    svg_shape_document whitespace_document = {0};
    CHECK(svg_shape_decode_cancellable(
        (base_bytes){whitespace_svg, whitespace_length},
        count_cancellation_checks, &whitespace_baseline, NULL,
        &whitespace_document, &canceled_diagnostic) == SVG_SHAPE_ERROR_NONE);
    svg_shape_document_destroy(&whitespace_document);
    CHECK(whitespace_baseline.calls > 3U);
    cancel_counter stop_whitespace = {0U, whitespace_baseline.calls - 2U};
    bool whitespace_canceled = false;
    CHECK(svg_shape_decode_cancellable(
        (base_bytes){whitespace_svg, whitespace_length},
        cancel_after_second_pass_starts, &stop_whitespace,
        &whitespace_canceled, &whitespace_document,
        &canceled_diagnostic) == SVG_SHAPE_ERROR_LIMIT);
    CHECK(whitespace_canceled && whitespace_document.edges == NULL);
    svg_shape_document_destroy(&whitespace_document);

    static const char *const invalid_svgs[] = {
        "<svg width='16' height='16'><path id='p' d='M0 0'/><path id='p' d='M1 1'/></svg>",
        "<svg width='16' height='16'><use href='#missing'/></svg>",
        "<svg width='16' height='16'><script>alert(1)</script></svg>",
        "<svg width='16' height='16' onload='active()'><path d='M0 0'/></svg>",
        "<svg width='16' height='16'><defs><g onclick='active()'/></defs></svg>",
        "<svg width='16' height='16'><image href='https://example.invalid/x.png'/></svg>",
        "<svg width='16' height='16'><defs><use href='#missing'/></defs></svg>",
        "<svg width='16' height='16'><style>.x{fill:url(#missing)}</style></svg>",
        "<svg width='16' height='16'><path style='fill:url(https://example.invalid/x)' d='M0 0'/></svg>",
        "<svg width='16' height='16'><path style='fill:u\\72l(https://example.invalid/x)' d='M0 0'/></svg>",
        "<svg width='16' height='16'><a href=' javascript:alert(1)'><rect width='1' height='1'/></a></svg>",
        "<svg width='16' height='16'><a href='java&#x09;script:alert(1)'><rect width='1' height='1'/></a></svg>",
        "<svg width='16' height='16'><style>@\\69mport url(https://example.invalid/a.css);</style></svg>",
        "<svg width='16' height='16'><style>@import url(https://example.invalid/a.css);</style></svg>",
        "<svg width='16' height='16'><path d='M0 0'></svg>"
    };
    for (size_t i = 0U; i < sizeof(invalid_svgs) / sizeof(invalid_svgs[0]); ++i) {
        asset.data = bytes(invalid_svgs[i]);
        result = validate("<svg><g id='layer'/></svg>", repeated_cels,
            &asset, 1U, NULL, NULL, NULL);
        CHECK(result.validation.error != PROJECT_TIMELINE_OK &&
            result.asset_name.len == sizeof(name) - 1U &&
            result.asset_validation.error != SVG_SHAPE_ERROR_NONE);
    }

    static const uint8_t unused_name[] = "imported-source";
    static const uint8_t bad_source[] = "not svg";
    project_bundle_asset unused = {{unused_name, sizeof(unused_name) - 1U},
        {media, sizeof(media) - 1U}, {bad_source, sizeof(bad_source) - 1U}};
    asset.data = (base_bytes){valid, sizeof(valid) - 1U};
    project_bundle_asset both[] = {asset, unused};
    result = validate("<svg><g id='layer'/></svg>", repeated_cels,
        both, 2U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK);

    static const char future[] = "{\"schema\":\"future\",\"version\":8}";
    asset.data = bytes("not svg");
    result = validate("<svg><g id='layer'/></svg>", future,
        &asset, 1U, NULL, NULL, NULL);
    CHECK(result.validation.error == PROJECT_TIMELINE_OK &&
        result.compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE);
    return true;
}

int main(void)
{
    if (!test_native_sorted_ids_and_assets() || !test_duplicate_and_dangling_ids() ||
        !test_unknown_opaque_and_cancel() || !test_asset_media_mismatch() ||
        !test_drawing_asset_content_validation()) return EXIT_FAILURE;
    return EXIT_SUCCESS;
}
