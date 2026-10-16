#include "viewer_timeline.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; \
} } while (0)

static bool snapshot_has_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *expected)
{
    size_t expected_length = strlen(expected);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *label = NULL;
        size_t length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &label, &length) && length == expected_length &&
            memcmp(label, expected, length) == 0) return true;
    }
    return false;
}

int main(void)
{
    static const uint8_t scene_source[] =
        "<svg width='8' height='8'><rect id='r' x='1' y='1' width='4' "
        "height='4' opacity='0.5'/></svg>";
    static const uint8_t timeline_source[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":0.25},"
        "\"tracks\":[]}";
    project_bundle_input input = {
        .manifest_json = {(const uint8_t *)"{}", 2U},
        .scene_svg = {scene_source, sizeof(scene_source) - 1U},
        .timeline_json = {timeline_source, sizeof(timeline_source) - 1U}
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
    project_timeline_integration_result validation =
        project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg,
            NULL, NULL, NULL, &context);
    CHECK(validation.validation.error == PROJECT_TIMELINE_OK &&
        validation.compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1);
    project_timeline_session *session = NULL;
    CHECK(project_timeline_session_create(bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &session).error ==
        PROJECT_TIMELINE_OK);
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {0};
    CHECK(svg_shape_decode(bundle.scene_svg, &scene, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    viewer_timeline timeline;
    viewer_timeline_init(&timeline);
    CHECK(viewer_timeline_reset_native(&timeline, &scene, session, &bundle) &&
        timeline.dock_open && viewer_timeline_track_count(&timeline) == 0U);

    viewer_timeline_layout layout;
    CHECK(viewer_timeline_layout_compute(&timeline, 900U, 720U, 1.0, &layout));
    CHECK(!layout.collapsed && layout.track_count == 0U &&
        layout.key_opacity_button.width > 0U &&
        layout.key_transform_button.width > 0U);
    ui_semantics_snapshot semantics = {0};
    CHECK(viewer_timeline_accessibility_snapshot(&timeline, &layout,
        UI_THEME_LIGHT, false, &semantics));
    CHECK(snapshot_has_label(&semantics, UI_ROLE_BUTTON,
        "Key selected object opacity at playhead"));
    CHECK(snapshot_has_label(&semantics, UI_ROLE_BUTTON,
        "Key selected object transform at playhead"));

    viewer_editor *editor = viewer_editor_create(
        bundle.scene_svg, UI_THEME_LIGHT, 900U, 720U);
    CHECK(editor != NULL);
    const uint8_t *element = (const uint8_t *)strstr((const char *)scene_source,
        "<rect");
    CHECK(element != NULL && viewer_editor_select_at_source_offset(editor,
        (size_t)(element - scene_source) + 1U));
    size_t canvas_size = 900U * 720U * 4U;
    uint8_t *canvas = calloc(canvas_size, 1U);
    CHECK(canvas != NULL);
    CHECK(viewer_timeline_paint_scalar_appearance(&timeline, editor, canvas,
        canvas_size, 900U, 720U, 900U * 4U, 1.0, UI_THEME_LIGHT, false,
        false));
    free(canvas);
    CHECK(viewer_timeline_pointer_ex(&timeline, &layout,
        VIEWER_TIMELINE_POINTER_DOWN,
        (double)layout.key_opacity_button.x + 2.0,
        (double)layout.key_opacity_button.y + 2.0, 0.0, 0.0, 0U, 0.0,
        editor));
    CHECK(viewer_timeline_take_native_mutation(&timeline) &&
        viewer_timeline_track_count(&timeline) == 1U &&
        project_timeline_session_is_dirty(session));
    project_timeline_track_view track;
    CHECK(viewer_timeline_native_track_at(&timeline, 0U, &track) &&
        track.property.len == sizeof("opacity") - 1U &&
        memcmp(track.property.ptr, "opacity", sizeof("opacity") - 1U) == 0);
    const raster_tile *frame = viewer_timeline_prepare_native_frame(&timeline,
        bundle.scene_svg);
    CHECK(frame != NULL && viewer_timeline_native_preview_is_current(&timeline));

    viewer_editor_destroy(editor);
    viewer_timeline_destroy(&timeline);
    svg_shape_document_destroy(&scene);
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    free(encoded);
    (void)puts("viewer timeline bootstrap: passed");
    return 0;
}
