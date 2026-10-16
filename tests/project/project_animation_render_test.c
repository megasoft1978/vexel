#include "project_animation_render.h"
#include "project_timeline_integration.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

typedef struct fixture {
    uint8_t *encoded;
    size_t encoded_length;
    project_bundle_view bundle;
    project_timeline_view timeline;
    project_timeline_context_owner context;
    svg_shape_document scene;
    uint8_t *bundle_copy;
    uint8_t *timeline_copy;
    uint8_t *scene_copy;
} fixture;

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static void fixture_destroy(fixture *state)
{
    svg_shape_document_destroy(&state->scene);
    project_timeline_context_owner_destroy(&state->context);
    free(state->bundle_copy);
    free(state->timeline_copy);
    free(state->scene_copy);
    free(state->encoded);
    *state = (fixture){0};
}

static bool fixture_create_with_scene(const char *timeline_json,
                           const project_bundle_asset *assets,
                           size_t asset_count, const char *scene_text,
                           fixture *out)
{
    static const char manifest_text[] = "{}";
    project_bundle_input input = {
        .manifest_json = bytes(manifest_text),
        .scene_svg = bytes(scene_text),
        .timeline_json = bytes(timeline_json),
        .assets = assets,
        .asset_count = asset_count
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) return false;
    out->encoded = malloc(measured.encoded_size);
    if (out->encoded == NULL) return false;
    out->encoded_length = measured.encoded_size;
    if (project_bundle_encode(&input,
            (base_mut_bytes){out->encoded, out->encoded_length}).error !=
            PROJECT_BUNDLE_OK ||
        project_bundle_decode((base_bytes){out->encoded, out->encoded_length},
                              &out->bundle).error != PROJECT_BUNDLE_OK)
        return false;
    project_timeline_integration_result checked =
        project_timeline_validate_bundle_owned(&out->bundle,
            out->bundle.scene_svg, NULL, NULL, &out->timeline, &out->context);
    if (checked.validation.error != PROJECT_TIMELINE_OK ||
        checked.compatibility != PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1)
        return false;
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode(out->bundle.scene_svg, &out->scene, &diagnostic) !=
        SVG_SHAPE_ERROR_NONE) return false;
    out->timeline_copy = malloc(out->bundle.timeline_json.len);
    out->scene_copy = malloc(out->bundle.scene_svg.len);
    out->bundle_copy = malloc(out->encoded_length);
    if (out->timeline_copy == NULL || out->scene_copy == NULL ||
        out->bundle_copy == NULL) return false;
    memcpy(out->bundle_copy, out->encoded, out->encoded_length);
    memcpy(out->timeline_copy, out->bundle.timeline_json.ptr,
           out->bundle.timeline_json.len);
    memcpy(out->scene_copy, out->bundle.scene_svg.ptr,
           out->bundle.scene_svg.len);
    return true;
}

static bool fixture_create(const char *timeline_json,
                           const project_bundle_asset *assets,
                           size_t asset_count, fixture *out)
{
    static const char scene[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><g id='layer'/></svg>";
    return fixture_create_with_scene(timeline_json, assets, asset_count,
        scene, out);
}

static bool alternate_bundle_create(const fixture *source,
    const project_bundle_asset *assets, size_t asset_count,
    uint8_t **out_encoded, project_bundle_view *out_bundle)
{
    project_bundle_input input = {
        .manifest_json = source->bundle.manifest_json,
        .scene_svg = source->bundle.scene_svg,
        .timeline_json = source->bundle.timeline_json,
        .assets = assets,
        .asset_count = asset_count
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) return false;
    uint8_t *encoded = malloc(measured.encoded_size);
    if (encoded == NULL) return false;
    if (project_bundle_encode(&input,
            (base_mut_bytes){encoded, measured.encoded_size}).error !=
            PROJECT_BUNDLE_OK ||
        project_bundle_decode((base_bytes){encoded, measured.encoded_size},
                              out_bundle).error != PROJECT_BUNDLE_OK) {
        free(encoded);
        return false;
    }
    *out_encoded = encoded;
    return true;
}

static project_bundle_asset asset(const char *name, const char *svg)
{
    static const uint8_t media[] = "image/svg+xml";
    return (project_bundle_asset){
        bytes(name), {media, sizeof(media) - 1U}, bytes(svg)
    };
}

static bool pixel(const raster_tile *tile, uint8_t expected_r,
                  uint8_t expected_g, uint8_t expected_b, uint8_t expected_a)
{
    if (tile->rgba == NULL || tile->byte_count < 4U ||
        tile->rgba[0] != expected_r || tile->rgba[1] != expected_g ||
        tile->rgba[2] != expected_b || tile->rgba[3] != expected_a) {
        (void)fprintf(stderr, "pixel got %u,%u,%u,%u expected %u,%u,%u,%u\n",
            tile->rgba == NULL ? 0U : tile->rgba[0],
            tile->rgba == NULL ? 0U : tile->rgba[1],
            tile->rgba == NULL ? 0U : tile->rgba[2],
            tile->rgba == NULL ? 0U : tile->rgba[3],
            expected_r, expected_g, expected_b, expected_a);
        return false;
    }
    return true;
}

static bool test_exposure_hold_transition_and_immutability(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":4,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"red\"},"
        "{\"tick\":2,\"asset\":\"green\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#ff0000'/></svg>";
    static const char green_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#00ff00'/></svg>";
    project_bundle_asset assets[] = {asset("red", red_svg),
                                     asset("green", green_svg)};
    fixture state = {0};
    CHECK(fixture_create(timeline, assets, 2U, &state));
    base_bytes timeline_source_before = state.timeline.source;
    size_t timeline_tracks_before = state.timeline.track_count;
    size_t scene_edges_before = state.scene.edge_count;
    double scene_view_width_before = state.scene.view_width;
    raster_tile frame = {0};
    project_animation_render_result result = project_animation_render_frame(
        &state.scene, &state.bundle, &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 255U, 0U, 0U, 255U));
    raster_tile_destroy(&frame);

    result = project_animation_render_frame(&state.scene, &state.bundle,
        &state.timeline, 1U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 255U, 0U, 0U, 255U));
    raster_tile_destroy(&frame);

    result = project_animation_render_frame(&state.scene, &state.bundle,
        &state.timeline, 2U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 0U, 255U, 0U, 255U));
    raster_tile_destroy(&frame);
    CHECK(memcmp(state.timeline_copy, state.bundle.timeline_json.ptr,
                 state.bundle.timeline_json.len) == 0);
    CHECK(memcmp(state.scene_copy, state.bundle.scene_svg.ptr,
                 state.bundle.scene_svg.len) == 0);
    CHECK(memcmp(state.bundle_copy, state.bundle.encoded.ptr,
                 state.bundle.encoded.len) == 0);
    CHECK(state.timeline.source.ptr == timeline_source_before.ptr &&
        state.timeline.source.len == timeline_source_before.len &&
        state.timeline.track_count == timeline_tracks_before &&
        state.scene.edge_count == scene_edges_before &&
        state.scene.view_width == scene_view_width_before);
    fixture_destroy(&state);
    return true;
}

static bool test_track_order_and_alpha_composition(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":2,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":["
        "{\"id\":\"first\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"red\"}]},"
        "{\"id\":\"second\",\"kind\":\"drawing\",\"target\":\"layer\","
        "\"visible\":true,\"cels\":[{\"tick\":0,\"asset\":\"blue-half\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#ff0000'/></svg>";
    static const char blue_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#0000ff' fill-opacity='.5'/></svg>";
    project_bundle_asset assets[] = {asset("red", red_svg),
                                     asset("blue-half", blue_svg)};
    fixture state = {0};
    CHECK(fixture_create(timeline, assets, 2U, &state));
    raster_tile frame = {0};
    project_animation_render_result result = project_animation_render_frame(
        &state.scene, &state.bundle, &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 127U, 0U, 128U, 255U));
    uint8_t *original_tile = frame.rgba;
    result = project_animation_render_frame(&state.scene, &state.bundle,
        &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT &&
        frame.rgba == original_tile);
    raster_tile_destroy(&frame);
    fixture_destroy(&state);
    return true;
}

static bool test_property_linear_interpolation_and_source_preservation(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":3,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"color\",\"kind\":\"property\","
        "\"target\":\"shape\",\"visible\":true,\"property\":\"fill\","
        "\"value_type\":\"rgba\",\"interpolation\":\"linear\",\"keys\":["
        "{\"tick\":0,\"value\":[1,0,0,1]},{\"tick\":2,\"value\":[0,0,1,1]}]}]}";
    static const char scene[] = "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect id='shape' width='2' height='2' fill='#000000'/></svg>";
    fixture state = {0};
    CHECK(fixture_create_with_scene(timeline, NULL, 0U, scene, &state));
    raster_tile frame = {0};
    project_animation_render_result result = project_animation_render_frame(
        &state.scene, &state.bundle, &state.timeline, 1U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 128U, 0U, 128U, 255U));
    raster_tile_destroy(&frame);
    CHECK(memcmp(state.scene_copy, state.bundle.scene_svg.ptr,
        state.bundle.scene_svg.len) == 0);
    CHECK(memcmp(state.timeline_copy, state.bundle.timeline_json.ptr,
        state.bundle.timeline_json.len) == 0);
    result = project_animation_render_frame(&state.scene, &state.bundle,
        &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(pixel(&frame, 255U, 0U, 0U, 255U));
    raster_tile_destroy(&frame);
    fixture_destroy(&state);
    return true;
}

static bool test_bundle_mismatch_failures(void)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":2,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"red\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#ff0000'/></svg>";
    static const char invalid_svg[] = "<svg><path d='M0 0L'/></svg>";
    static const uint8_t png_data[] = {0x89U, 'P', 'N', 'G'};
    static const uint8_t png_type[] = "image/png";
    project_bundle_asset valid = asset("red", red_svg);
    fixture state = {0};
    CHECK(fixture_create(timeline, &valid, 1U, &state));
    raster_tile frame = {0};
    uint8_t *alternate_encoded = NULL;
    project_bundle_view alternate = {0};
    CHECK(alternate_bundle_create(&state, NULL, 0U, &alternate_encoded,
                                  &alternate));
    project_animation_render_result result = project_animation_render_frame(
        &state.scene, &alternate, &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_INVALID_REFERENCE &&
        frame.rgba == NULL);
    free(alternate_encoded);

    project_bundle_asset wrong_media = {
        bytes("red"), {png_type, sizeof(png_type) - 1U},
        {png_data, sizeof(png_data)}
    };
    alternate_encoded = NULL;
    CHECK(alternate_bundle_create(&state, &wrong_media, 1U,
                                  &alternate_encoded, &alternate));
    result = project_animation_render_frame(&state.scene, &alternate,
        &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_UNSUPPORTED_MEDIA &&
        frame.rgba == NULL);
    free(alternate_encoded);

    project_bundle_asset bad_svg = asset("red", invalid_svg);
    alternate_encoded = NULL;
    CHECK(alternate_bundle_create(&state, &bad_svg, 1U, &alternate_encoded,
                                  &alternate));
    result = project_animation_render_frame(&state.scene, &alternate,
        &state.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_DECODE_FAILED &&
        frame.rgba == NULL);
    free(alternate_encoded);
    fixture_destroy(&state);
    return true;
}

static bool test_empty_frames_and_failures(void)
{
    static const char no_tracks[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":2,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[]}";
    fixture empty = {0};
    CHECK(fixture_create(no_tracks, NULL, 0U, &empty));
    raster_tile frame = {0};
    project_animation_render_result result = project_animation_render_frame(
        &empty.scene, &empty.bundle, &empty.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK &&
        frame.rgba != NULL && frame.rgba[3] == 0U);
    raster_tile_destroy(&frame);
    result = project_animation_render_frame(&empty.scene, &empty.bundle,
        &empty.timeline, 2U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_INVALID_TIMELINE &&
        frame.rgba == NULL);
    fixture_destroy(&empty);

    static const char invisible_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":2,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":false,"
        "\"cels\":[{\"tick\":0,\"asset\":\"red\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#ff0000'/></svg>";
    project_bundle_asset red = asset("red", red_svg);
    fixture invisible = {0};
    CHECK(fixture_create(invisible_timeline, &red, 1U, &invisible));
    result = project_animation_render_frame(&invisible.scene, &invisible.bundle,
        &invisible.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_OK &&
        frame.rgba != NULL && frame.rgba[3] == 0U);
    raster_tile_destroy(&frame);
    size_t saved_shape_count = invisible.scene.shape_count;
    invisible.scene.shape_count = 1U;
    result = project_animation_render_frame(&invisible.scene, &invisible.bundle,
        &invisible.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_RASTER_FAILED &&
        frame.rgba == NULL);
    invisible.scene.shape_count = saved_shape_count;
    fixture_destroy(&invisible);

    static const char one_cel[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":2,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"red\"}]}]}";
    static const char wrong_size[] =
        "<svg width='3' height='2' viewBox='0 0 3 2'>"
        "<rect width='3' height='2' fill='#ff0000'/></svg>";
    project_bundle_asset mismatched = asset("red", wrong_size);
    fixture bad_size = {0};
    CHECK(fixture_create(one_cel, &mismatched, 1U, &bad_size));
    result = project_animation_render_frame(&bad_size.scene, &bad_size.bundle,
        &bad_size.timeline, 0U, &frame);
    CHECK(result.error == PROJECT_ANIMATION_RENDER_DIMENSION_MISMATCH &&
        frame.rgba == NULL);
    fixture_destroy(&bad_size);
    return true;
}

int main(void)
{
    if (!test_exposure_hold_transition_and_immutability() ||
        !test_property_linear_interpolation_and_source_preservation() ||
        !test_track_order_and_alpha_composition() ||
        !test_bundle_mismatch_failures() ||
        !test_empty_frames_and_failures()) return EXIT_FAILURE;
    (void)puts("native project animation renderer tests passed");
    return EXIT_SUCCESS;
}
