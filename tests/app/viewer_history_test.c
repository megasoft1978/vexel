#include "viewer_internal.h"
#include "viewer_editor_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; \
} } while (0)

bool viewer_set_window_title(viewer_app *app)
{
    return app != NULL;
}

int main(void)
{
    static const uint8_t scene_bytes[] =
        "<svg width='8' height='8'><rect id='r' width='2' height='2'/></svg>";
    static const uint8_t timeline_bytes[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,"
        "\"opacity\":0.25},\"tracks\":[]}";
    project_bundle_input input = {
        .manifest_json = {(const uint8_t *)"{}", 2U},
        .scene_svg = {scene_bytes, sizeof(scene_bytes) - 1U},
        .timeline_json = {timeline_bytes, sizeof(timeline_bytes) - 1U}
    };
    project_bundle_result size = project_bundle_encoded_size(&input);
    CHECK(size.error == PROJECT_BUNDLE_OK);
    uint8_t *encoded = malloc(size.encoded_size);
    CHECK(encoded != NULL && project_bundle_encode(&input,
        (base_mut_bytes){encoded, size.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){encoded, size.encoded_size},
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
    viewer_app app = {0};
    app.document.timeline_session = session;
    app.document.project = bundle;
    app.editor = viewer_editor_create(bundle.scene_svg, UI_THEME_LIGHT, 320U, 240U);
    CHECK(app.editor != NULL);
    CHECK(viewer_editor_set_workspace(app.editor, VIEWER_WORKSPACE_ANIMATE));
    viewer_editor_set_history_callback(app.editor, viewer_history_callback, &app);
    viewer_timeline_init(&app.timeline);
    CHECK(viewer_timeline_reset_native(&app.timeline, &scene, session, &bundle));
    app.timeline.preview_valid = true;
    CHECK(project_timeline_session_set_loop(session,
        PROJECT_TIMELINE_LOOP_REPEAT).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_is_dirty(session));
    CHECK(viewer_editor_can_undo(app.editor));
    CHECK(!viewer_editor_can_redo(app.editor));
    uint32_t result = viewer_editor_key(app.editor, 0U,
        PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(project_timeline_session_redo_count(session) == 1U);
    CHECK(!project_timeline_session_is_dirty(session));
    CHECK(!app.timeline.preview_valid);
    CHECK(app.frame_dirty);
    CHECK(viewer_editor_can_redo(app.editor));
    result = editor_apply_button(app.editor, EDITOR_BUTTON_REDO);
    CHECK((result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(project_timeline_session_is_dirty(session));
    CHECK(viewer_editor_can_undo(app.editor));
    result = editor_apply_button(app.editor, EDITOR_BUTTON_UNDO);
    CHECK((result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(!project_timeline_session_is_dirty(session));
    result = viewer_editor_key(app.editor, 0U,
        PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U);
    CHECK((result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(project_timeline_session_is_dirty(session));

    document_handle root = {0};
    CHECK(document_edit_root(app.editor->edit, &root));
    CHECK(document_edit_add_attribute(app.editor->edit, root,
        (base_bytes){(const uint8_t *)"data-test", 9U},
        (base_bytes){(const uint8_t *)"1", 1U}) == DOCUMENT_EDIT_OK);
    CHECK(viewer_editor_set_workspace(app.editor, VIEWER_WORKSPACE_DRAW));
    CHECK(viewer_editor_can_undo(app.editor));
    size_t undo_count = project_timeline_session_undo_count(session);
    result = viewer_editor_key(app.editor, 0U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
    CHECK((result & VIEWER_EDITOR_MUTATED) != 0U);
    CHECK(project_timeline_session_undo_count(session) == undo_count);
    CHECK(project_timeline_session_is_dirty(session));

    viewer_timeline_destroy(&app.timeline);
    viewer_editor_destroy(app.editor);
    svg_shape_document_destroy(&scene);
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    free(encoded);
    (void)puts("viewer history routing: passed");
    return 0;
}
