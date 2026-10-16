#include "project_timeline_session.h"

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

static bool test_session_decode_and_render_tracks_edits(void)
{
    static const char manifest[] = "{}";
    static const char scene_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><g id='layer'/></svg>";
    static const char timeline_json[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":4,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"red\"}]}]}";
    static const char red_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#ff0000'/></svg>";
    static const char green_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'>"
        "<rect width='2' height='2' fill='#00ff00'/></svg>";
    project_bundle_asset assets[] = {
        {{(const uint8_t *)"red", 3U}, {(const uint8_t *)"image/svg+xml", 13U},
         {(const uint8_t *)red_svg, sizeof(red_svg) - 1U}},
        {{(const uint8_t *)"green", 5U}, {(const uint8_t *)"image/svg+xml", 13U},
         {(const uint8_t *)green_svg, sizeof(green_svg) - 1U}}
    };
    project_bundle_input input = {
        .manifest_json = bytes(manifest), .scene_svg = bytes(scene_svg),
        .timeline_json = bytes(timeline_json), .assets = assets,
        .asset_count = 2U
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK);
    uint8_t *encoded = malloc(measured.encoded_size);
    CHECK(encoded != NULL);
    CHECK(project_bundle_encode(&input,
        (base_mut_bytes){encoded, measured.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){encoded, measured.encoded_size},
        &bundle).error == PROJECT_BUNDLE_OK);
    project_timeline_context_owner context = {0};
    CHECK(project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg,
        NULL, NULL, NULL, &context).validation.error == PROJECT_TIMELINE_OK);
    project_timeline_session *session = NULL;
    CHECK(project_timeline_session_create(bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &session).error == PROJECT_TIMELINE_OK);
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {0};
    CHECK(svg_shape_decode(bundle.scene_svg, &scene, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    project_timeline_view view = {0};
    CHECK(project_timeline_session_decode(session, &view).error ==
        PROJECT_TIMELINE_OK);
    CHECK(view.track_count == 1U && view.duration_ticks == 4U &&
        view.ticks_per_second == 24U);
    base_bytes before = project_timeline_session_source(session);
    uint8_t *before_copy = malloc(before.len);
    CHECK(before_copy != NULL);
    memcpy(before_copy, before.ptr, before.len);
    raster_tile frame = {0};
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 0U,
        &frame).error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(frame.rgba != NULL && frame.rgba[0] == 255U &&
        frame.rgba[1] == 0U && frame.rgba[3] == 255U);
    raster_tile_destroy(&frame);
    CHECK(memcmp(before_copy, project_timeline_session_source(session).ptr,
        before.len) == 0);
    CHECK(project_timeline_session_set_cel(session, bytes("draw"), 2U,
        bytes("green"), bytes("green pose")).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 1U,
        &frame).error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(frame.rgba[0] == 255U && frame.rgba[1] == 0U);
    raster_tile_destroy(&frame);
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 2U,
        &frame).error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(frame.rgba[0] == 0U && frame.rgba[1] == 255U);
    raster_tile_destroy(&frame);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 2U,
        &frame).error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(frame.rgba[0] == 255U && frame.rgba[1] == 0U);
    raster_tile_destroy(&frame);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 2U,
        &frame).error == PROJECT_ANIMATION_RENDER_OK);
    CHECK(frame.rgba[0] == 0U && frame.rgba[1] == 255U);
    raster_tile_destroy(&frame);
    CHECK(project_timeline_session_render_frame(session, &scene, &bundle, 4U,
        &frame).error == PROJECT_ANIMATION_RENDER_INVALID_TIMELINE);
    CHECK(frame.rgba == NULL && frame.byte_count == 0U);
    project_timeline_context_owner_destroy(&context);
    svg_shape_document_destroy(&scene);
    project_timeline_session_destroy(session);
    free(before_copy);
    free(encoded);
    return true;
}

static bool test_session_edit_and_context_refresh(void)
{
    static const uint8_t manifest[] = "{\"label\":\"preserve\"}";
    static const uint8_t scene[] = "<svg><g id='layer'/></svg>";
    static const uint8_t timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t asset_name[] = "cel0";
    static const uint8_t media_type[] = "image/svg+xml";
    static const uint8_t asset_data[] =
        "<svg width='2' height='2'><path d='M0 0L2 2'/></svg>";
    project_bundle_asset asset = {{asset_name, sizeof(asset_name) - 1U},
        {media_type, sizeof(media_type) - 1U},
        {asset_data, sizeof(asset_data) - 1U}};
    project_bundle_input input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {scene, sizeof(scene) - 1U},
        .timeline_json = {timeline, sizeof(timeline) - 1U},
        .assets = &asset,
        .asset_count = 1U
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK);
    uint8_t *owner = malloc(measured.encoded_size);
    CHECK(owner != NULL);
    CHECK(project_bundle_encode(&input,
        (base_mut_bytes){owner, measured.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){owner, measured.encoded_size},
        &bundle).error == PROJECT_BUNDLE_OK);

    project_timeline_context_owner context = {0};
    project_timeline_view view = {0};
    project_timeline_integration_result validation =
        project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg, NULL,
            NULL, &view, &context);
    CHECK(validation.compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 &&
        validation.validation.error == PROJECT_TIMELINE_OK && context.scene_ids != NULL);
    project_timeline_session *session = NULL;
    CHECK(project_timeline_session_create(bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &session).error == PROJECT_TIMELINE_OK);
    CHECK(context.scene_ids == NULL);
    CHECK(!project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_set_loop(session,
        PROJECT_TIMELINE_LOOP_PING_PONG).error == PROJECT_TIMELINE_OK);
    project_timeline_view loop_view = {0};
    CHECK(project_timeline_session_decode(session, &loop_view).error ==
        PROJECT_TIMELINE_OK &&
        loop_view.loop_mode == PROJECT_TIMELINE_LOOP_PING_PONG &&
        project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_decode(session, &loop_view).error ==
        PROJECT_TIMELINE_OK && loop_view.loop_mode == PROJECT_TIMELINE_LOOP_ONCE);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_set_onion(session, 1U, 2U, 0.5).error ==
        PROJECT_TIMELINE_OK);
    project_timeline_view onion_view = {0};
    CHECK(project_timeline_session_decode(session, &onion_view).error ==
        PROJECT_TIMELINE_OK && onion_view.onion_before == 1U &&
        onion_view.onion_after == 2U && onion_view.onion_opacity == 0.5);
    CHECK(project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_decode(session, &onion_view).error ==
        PROJECT_TIMELINE_OK && onion_view.onion_before == 0U &&
        onion_view.onion_after == 0U);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_mark_saved(session));
    CHECK(!project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_set_cel(session, bytes("draw"), 12U,
        bytes("cel0"), bytes("pose")).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_is_dirty(session));
    base_bytes edited = project_timeline_session_source(session);
    CHECK(edited.len > bundle.timeline_json.len);

    static const uint8_t missing_scene[] = "<svg><g id='other'/></svg>";
    CHECK(project_timeline_session_refresh_context(session, &bundle,
        (base_bytes){missing_scene, sizeof(missing_scene) - 1U}, NULL, NULL).error ==
        PROJECT_TIMELINE_INVALID_REFERENCE);
    CHECK(project_timeline_session_set_cel(session, bytes("draw"), 6U,
        bytes("cel0"), bytes("held")).error == PROJECT_TIMELINE_OK);

    static const uint8_t expanded_scene[] =
        "<svg><g id='layer'/><g id='new-layer'/></svg>";
    CHECK(project_timeline_session_refresh_context(session, &bundle,
        (base_bytes){expanded_scene, sizeof(expanded_scene) - 1U}, NULL, NULL).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_add_drawing_track(session, bytes("second"),
        bytes("new-layer"), bytes("cel0"), bytes("Second")).error ==
        PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_redo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_mark_saved(session));
    CHECK(!project_timeline_session_is_dirty(session));
    CHECK(project_timeline_session_undo(session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_is_dirty(session));

    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    free(owner);
    return true;
}

static bool test_opaque_passthrough_has_no_session(void)
{
    static const uint8_t manifest[] = "{}";
    static const uint8_t scene[] = "<svg><g id='layer'/></svg>";
    static const uint8_t timeline[] = "{\"schema\":\"future\",\"version\":9}";
    project_bundle_input input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {scene, sizeof(scene) - 1U},
        .timeline_json = {timeline, sizeof(timeline) - 1U}
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    CHECK(measured.error == PROJECT_BUNDLE_OK);
    uint8_t *owner = malloc(measured.encoded_size);
    CHECK(owner != NULL);
    CHECK(project_bundle_encode(&input,
        (base_mut_bytes){owner, measured.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){owner, measured.encoded_size},
        &bundle).error == PROJECT_BUNDLE_OK);
    project_timeline_context_owner context = {0};
    project_timeline_integration_result validation =
        project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg, NULL,
            NULL, NULL, &context);
    CHECK(validation.compatibility == PROJECT_TIMELINE_COMPATIBILITY_OPAQUE &&
        validation.validation.error == PROJECT_TIMELINE_OK &&
        context.scene_ids == NULL && context.assets == NULL);
    CHECK(bundle.timeline_json.len == sizeof(timeline) - 1U &&
        memcmp(bundle.timeline_json.ptr, timeline, sizeof(timeline) - 1U) == 0);
    project_timeline_context_owner_destroy(&context);
    free(owner);
    return true;
}

int main(void)
{
    return test_session_decode_and_render_tracks_edits() &&
        test_session_edit_and_context_refresh() &&
        test_opaque_passthrough_has_no_session() ? EXIT_SUCCESS : EXIT_FAILURE;
}
