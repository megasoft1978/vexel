#include "viewer_internal.h"

#include "platform/macos/platform_dialog.h"
#include "platform/media/platform_image.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    goto done; } } while (0)

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='4' height='3' "
    "viewBox='0 0 4 3'><rect id='target' x='1' y='1' width='2' height='1' "
    "fill='#ff0000'/></svg>";

typedef struct export_dialog_fixture {
    char path[PLATFORM_DIALOG_PATH_CAP];
    uint8_t scale;
    bool selection_only;
    size_t calls;
} export_dialog_fixture;

static platform_dialog_status choose_png(void *context,
    const char *suggested_name_utf8, size_t suggested_name_length,
    uint8_t maximum_scale, char *out_path, size_t out_path_capacity,
    size_t *out_path_length, uint8_t *out_scale, bool *out_selection_only)
{
    export_dialog_fixture *fixture = context;
    size_t path_length = fixture == NULL ? 0U : strlen(fixture->path);
    (void)suggested_name_utf8;
    (void)suggested_name_length;
    if (fixture == NULL || out_path == NULL || out_path_length == NULL ||
        out_scale == NULL || out_selection_only == NULL ||
        path_length >= out_path_capacity || fixture->scale > maximum_scale)
        return PLATFORM_DIALOG_ERROR;
    memcpy(out_path, fixture->path, path_length + 1U);
    *out_path_length = path_length;
    *out_scale = fixture->scale;
    *out_selection_only = fixture->selection_only;
    ++fixture->calls;
    return PLATFORM_DIALOG_ACCEPT;
}

static bool exported_image_matches(const char *path, uint32_t width,
                                   uint32_t height)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    bool okay = false;
    if (fseek(file, 0L, SEEK_END) != 0) goto done;
    long size = ftell(file);
    if (size <= 0L || fseek(file, 0L, SEEK_SET) != 0) goto done;
    uint8_t *bytes = malloc((size_t)size);
    if (bytes == NULL) goto done;
    if (fread(bytes, 1U, (size_t)size, file) == (size_t)size) {
        platform_image image = PLATFORM_IMAGE_INIT;
        if (platform_image_decode(bytes, (size_t)size, &image) ==
                PLATFORM_MEDIA_OK && image.width == width &&
                image.height == height) {
            size_t x = width / 2U, y = height / 2U;
            size_t pixel = y * image.row_stride + x * 4U;
            okay = pixel + 3U < image.byte_count && image.rgba[pixel] > 200U &&
                image.rgba[pixel + 3U] > 0U;
        }
        platform_image_destroy(&image);
    }
    free(bytes);
done:
    (void)fclose(file);
    return okay;
}

int main(void)
{
    viewer_editor *editor = NULL;
    viewer_app app = {0};
    export_dialog_fixture fixture = {.scale = 2U};
    platform_dialog_test_hooks hooks = {
        .context = &fixture,
        .save_png = choose_png
    };
    int result = 1;
    int amount = snprintf(fixture.path, sizeof(fixture.path),
        "/tmp/vexel-viewer-export-%ld.png", (long)getpid());
    CHECK(amount > 0 && (size_t)amount < sizeof(fixture.path));
    (void)unlink(fixture.path);
    editor = viewer_editor_create((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_DARK, 640U, 480U);
    CHECK(editor != NULL && platform_dialog_set_test_hooks(&hooks));
    app.editor = editor;
    app.document_visible = true;
    memcpy(app.display_name, "export-test.svg", sizeof("export-test.svg"));

    if (!viewer_export_png(&app, false)) {
        (void)fprintf(stderr, "export failed: %s; dialog calls %zu\n",
            viewer_editor_status(editor), fixture.calls);
        goto done;
    }
    CHECK(fixture.calls == 1U && exported_image_matches(fixture.path, 8U, 6U));

    size_t target_offset = (size_t)(strstr((const char *)source, "<rect") -
        (const char *)source);
    CHECK(viewer_editor_select_at_source_offset(editor, target_offset));
    fixture.selection_only = true;
    CHECK(viewer_export_png(&app, false));
    CHECK(fixture.calls == 2U && exported_image_matches(fixture.path, 4U, 2U));
    result = 0;
done:
    (void)platform_dialog_set_test_hooks(NULL);
    viewer_editor_destroy(editor);
    (void)unlink(fixture.path);
    if (result == 0) (void)puts("viewer PNG export: document scale and selection crop passed");
    return result;
}
