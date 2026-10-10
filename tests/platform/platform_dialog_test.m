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
    platform_unsaved_choice unsaved_choice;
    bool error_result;
    size_t save_calls;
    size_t unsaved_calls;
    size_t error_calls;
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
    static const char path[] = "/tmp/chosen.svg";
    memcpy(out_path, path, sizeof(path));
    *out_length = sizeof(path) - 1U;
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
    result->save_status = platform_dialog_save_svg("draft", 5U, path,
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
        .error_result = true
    };
    platform_dialog_test_hooks hooks = {
        .context = &state,
        .save_svg = stub_save,
        .unsaved_changes = stub_unsaved,
        .show_error = stub_error
    };
    CHECK(platform_dialog_set_test_hooks(&hooks));

    char path[PLATFORM_DIALOG_PATH_CAP];
    size_t path_length = 0U;
    static const char name[] = "Draft";
    CHECK(platform_dialog_save_svg(name, sizeof(name) - 1U, path,
        sizeof(path), &path_length) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, "Draft.svg") == 0);
    CHECK(path_length == strlen("/tmp/chosen.svg") &&
        strcmp(path, "/tmp/chosen.svg") == 0);

    static const char other_name[] = "image.png";
    CHECK(platform_dialog_save_svg(other_name, sizeof(other_name) - 1U,
        path, sizeof(path), &path_length) == PLATFORM_DIALOG_ACCEPT);
    CHECK(strcmp(state.suggested_name, "image.png.svg") == 0);

    state.save_status = PLATFORM_DIALOG_CANCEL;
    CHECK(platform_dialog_save_svg(NULL, 0U, path, sizeof(path),
        &path_length) == PLATFORM_DIALOG_CANCEL);
    CHECK(path[0] == '\0' && path_length == 0U);
    state.save_status = PLATFORM_DIALOG_ACCEPT;
    CHECK(platform_dialog_save_svg(name, sizeof(name) - 1U, path, 4U,
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
    CHECK(state.save_calls == 4U && state.unsaved_calls == 3U &&
        state.error_calls == 1U);
    CHECK(platform_dialog_set_test_hooks(NULL));
    (void)puts("platform dialog tests passed: SVG suggestion, injected responses, and main-thread enforcement");
    return 0;
}
