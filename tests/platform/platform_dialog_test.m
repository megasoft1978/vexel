#import <Cocoa/Cocoa.h>

#include "platform_dialog.h"

#include <pthread.h>
#include <stdio.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

typedef struct stub_state {
    platform_dialog_status save_status;
    char selected_document_path[PLATFORM_DIALOG_PATH_CAP];
    platform_unsaved_choice unsaved_choice;
    platform_recovery_choice recovery_choice;
    bool error_result;
    size_t save_calls;
    size_t save_png_calls;
    size_t unsaved_calls;
    size_t error_calls;
    size_t recovery_calls;
    char suggested_name[128];
} stub_state;

static platform_dialog_status stub_save(void *context, const char *name,
    size_t name_length, char *out_path, size_t capacity, size_t *out_length)
{
    stub_state *state = context;
    state->save_calls++;
    if (name_length >= sizeof(state->suggested_name) ||
        capacity < sizeof("/tmp/chosen.svg")) return PLATFORM_DIALOG_ERROR;
    memcpy(state->suggested_name, name, name_length);
    state->suggested_name[name_length] = '\0';
    if (state->save_status != PLATFORM_DIALOG_ACCEPT) return state->save_status;
    const char *path = state->selected_document_path[0] == '\0' ?
        "/tmp/chosen.svg" : state->selected_document_path;
    size_t length = strlen(path);
    if (length >= capacity) return PLATFORM_DIALOG_ERROR;
    memcpy(out_path, path, length + 1U);
    *out_length = length;
    return PLATFORM_DIALOG_ACCEPT;
}

static platform_dialog_status stub_save_png(void *context, const char *name,
    size_t name_length, uint8_t maximum_scale, char *out_path, size_t capacity,
    size_t *out_length, uint8_t *out_scale, bool *selection_only)
{
    stub_state *state = context;
    state->save_png_calls++;
    if (name_length >= sizeof(state->suggested_name) ||
        capacity < sizeof("/tmp/chosen.png")) return PLATFORM_DIALOG_ERROR;
    memcpy(state->suggested_name, name, name_length);
    state->suggested_name[name_length] = '\0';
    if (state->save_status != PLATFORM_DIALOG_ACCEPT) return state->save_status;
    static const char path[] = "/tmp/chosen.png";
    memcpy(out_path, path, sizeof(path));
    *out_length = sizeof(path) - 1U;
    *out_scale = maximum_scale >= 2U ? 2U : 1U;
    *selection_only = true;
    return PLATFORM_DIALOG_ACCEPT;
}

static platform_unsaved_choice stub_unsaved(void *context, const char *title,
    size_t title_length, const char *message, size_t message_length)
{
    stub_state *state = context;
    state->unsaved_calls++;
    return title_length == 6U && message_length == 7U &&
        memcmp(title, "Close?", 6U) == 0 &&
        memcmp(message, "Changed", 7U) == 0 ?
        state->unsaved_choice : PLATFORM_UNSAVED_ERROR;
}

static platform_recovery_choice stub_recovery(void *context, const char *title,
    size_t title_length, const char *message, size_t message_length,
    bool conflict)
{
    stub_state *state = context;
    state->recovery_calls++;
    bool expected = conflict ?
        title_length == 17U && memcmp(title, "Recovery Conflict", 17U) == 0 :
        title_length == 21U && memcmp(title, "Recover Unsaved Work?", 21U) == 0;
    return expected && message_length > 0U && message != NULL ?
        state->recovery_choice : PLATFORM_RECOVERY_ERROR;
}

static bool stub_error(void *context, const char *title, size_t title_length,
    const char *message, size_t message_length)
{
    stub_state *state = context;
    state->error_calls++;
    return title_length == 5U && message_length == 7U &&
        memcmp(title, "Error", 5U) == 0 &&
        memcmp(message, "Details", 7U) == 0 && state->error_result;
}

typedef struct off_thread_result {
    bool shown;
    platform_dialog_status save_status;
    platform_unsaved_choice unsaved_choice;
} off_thread_result;

static void *call_error_off_main(void *context)
{
    off_thread_result *result = context;
    static const char title[] = "Error";
    static const char message[] = "Details";
    result->shown = platform_dialog_show_error(title, sizeof(title) - 1U,
        message, sizeof(message) - 1U);
    char path[PLATFORM_DIALOG_PATH_CAP];
    size_t path_length = 0U;
    result->save_status = platform_dialog_save_document("draft", 5U, path,
        sizeof(path), &path_length);
    result->unsaved_choice = platform_dialog_unsaved_changes("Close?", 6U,
        "Changed", 7U);
    return NULL;
}

int main(void)
{
    stub_state state = {
        .save_status = PLATFORM_DIALOG_ACCEPT,
        .unsaved_choice = PLATFORM_UNSAVED_SAVE,
        .recovery_choice = PLATFORM_RECOVERY_RESTORE,
        .error_result = true
    };
    platform_dialog_test_hooks hooks = {
        .context = &state,
        .save_document = stub_save,
        .save_png = stub_save_png,
        .unsaved_changes = stub_unsaved,
        .recovery_choice = stub_recovery,
        .show_error = stub_error
    };
    CHECK(platform_dialog_set_test_hooks(&hooks));

    CHECK(platform_dialog_recovery_choice("Recover Unsaved Work?", 21U,
        "A validated snapshot is available.", 34U, false) ==
        PLATFORM_RECOVERY_RESTORE);
    state.recovery_choice = PLATFORM_RECOVERY_DISCARD;
    CHECK(platform_dialog_recovery_choice("Recovery Conflict", 17U,
        "The source changed.", 19U, true) == PLATFORM_RECOVERY_DISCARD);
    CHECK(state.recovery_calls == 2U);

    char path[PLATFORM_DIALOG_PATH_CAP];
    size_t path_length = 0U;
    static const char name[] = "Draft";
    CHECK(platform_dialog_save_document(name, sizeof(name) - 1U, path,
        sizeof(path), &path_length) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, "Draft.svg") == 0);
    CHECK(path_length == strlen("/tmp/chosen.svg") &&
        strcmp(path, "/tmp/chosen.svg") == 0);

    uint8_t scale = 0U;
    bool selection_only = false;
    CHECK(platform_dialog_save_png("Draft.svg", 9U, 4U, path, sizeof(path),
        &path_length, &scale, &selection_only) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, "Draft.png") == 0 && scale == 2U &&
        selection_only && strcmp(path, "/tmp/chosen.png") == 0);

    static const char other_name[] = "image.png";
    CHECK(platform_dialog_save_document(other_name, sizeof(other_name) - 1U,
        path, sizeof(path), &path_length) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, "image.png.svg") == 0);

    static const char project_name[] = "Cartoon.vexel";
    strcpy(state.selected_document_path, "/tmp/chosen.vexel");
    CHECK(platform_dialog_save_document(project_name, sizeof(project_name) - 1U,
        path, sizeof(path), &path_length) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, project_name) == 0 &&
        strcmp(path, "/tmp/chosen.vexel") == 0);
    state.selected_document_path[0] = '\0';

    state.save_status = PLATFORM_DIALOG_CANCEL;
    CHECK(platform_dialog_save_document(NULL, 0U, path, sizeof(path),
        &path_length) == PLATFORM_DIALOG_CANCEL);
    CHECK(path[0] == '\0' && path_length == 0U);
    state.save_status = PLATFORM_DIALOG_ACCEPT;
    CHECK(platform_dialog_save_document(name, sizeof(name) - 1U, path, 4U,
        &path_length) == PLATFORM_DIALOG_ERROR);

    static const char title[] = "Close?";
    static const char message[] = "Changed";
    const platform_unsaved_choice expected[] = {
        PLATFORM_UNSAVED_SAVE, PLATFORM_UNSAVED_DONT_SAVE,
        PLATFORM_UNSAVED_CANCEL
    };
    for (size_t i = 0U; i < sizeof(expected) / sizeof(expected[0]); ++i) {
        state.unsaved_choice = expected[i];
        CHECK(platform_dialog_unsaved_changes(title, sizeof(title) - 1U,
            message, sizeof(message) - 1U) == expected[i]);
    }
    static const char error_title[] = "Error";
    static const char details[] = "Details";
    CHECK(platform_dialog_show_error(error_title, sizeof(error_title) - 1U,
        details, sizeof(details) - 1U));
    off_thread_result thread_result = {
        .shown = true,
        .save_status = PLATFORM_DIALOG_ACCEPT,
        .unsaved_choice = PLATFORM_UNSAVED_SAVE
    };
    pthread_t thread;
    CHECK(pthread_create(&thread, NULL, call_error_off_main, &thread_result) == 0);
    CHECK(pthread_join(thread, NULL) == 0 && !thread_result.shown &&
        thread_result.save_status == PLATFORM_DIALOG_ERROR &&
        thread_result.unsaved_choice == PLATFORM_UNSAVED_ERROR);
    CHECK(state.save_calls == 5U && state.save_png_calls == 1U &&
        state.unsaved_calls == 3U &&
        state.error_calls == 1U);
    CHECK(platform_dialog_set_test_hooks(NULL));
    (void)puts("platform dialog tests passed: SVG suggestion, injected responses, and main-thread enforcement");
    return 0;
}
