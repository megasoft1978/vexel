#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platform/platform_window.h"
#include "test_driver/native_injection.h"

#define NATIVE_COMMAND_LIMIT 64U

typedef enum native_command_type {
    NATIVE_OPEN,
    NATIVE_RESIZE,
    NATIVE_KEY_DOWN,
    NATIVE_KEY_UP,
    NATIVE_POINTER_MOVE,
    NATIVE_POINTER_DOWN,
    NATIVE_POINTER_UP,
    NATIVE_QUIT
} native_command_type;

typedef struct native_command {
    native_command_type type;
    const char *text;
    size_t text_len;
    int32_t x;
    int32_t y;
} native_command;

static bool parse_integer(const char *text, uint32_t maximum, uint32_t *out)
{
    if (text == NULL || text[0] == '\0' || out == NULL) {
        return false;
    }
    errno = 0;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value > maximum) {
        return false;
    }
    *out = (uint32_t)value;
    return true;
}

static size_t bounded_length(const char *text, size_t limit)
{
    size_t length = 0;
    if (text == NULL) {
        return 0;
    }
    while (length <= limit && text[length] != '\0') {
        length++;
    }
    return length;
}

static bool append_command(native_command *commands, size_t *count, native_command value)
{
    if (*count >= NATIVE_COMMAND_LIMIT) {
        return false;
    }
    commands[*count] = value;
    (*count)++;
    return true;
}

static bool parse_native_key(int argc, char *argv[], int *index, native_command *command)
{
    if (*index + 1 >= argc) {
        return false;
    }
    const char *action = argv[(*index)++];
    command->text = argv[(*index)++];
    command->text_len = bounded_length(command->text, 1);
    if (strcmp(command->text, "a") != 0 || command->text_len != 1) {
        return false;
    }
    if (strcmp(action, "down") == 0) {
        command->type = NATIVE_KEY_DOWN;
    } else if (strcmp(action, "up") == 0) {
        command->type = NATIVE_KEY_UP;
    } else {
        return false;
    }
    return true;
}

static bool parse_native_pointer(int argc, char *argv[], int *index,
                                 native_command *command)
{
    if (*index + 2 >= argc) {
        return false;
    }
    const char *action = argv[(*index)++];
    uint32_t x = 0, y = 0;
    if (!parse_integer(argv[(*index)++], 4095, &x) ||
        !parse_integer(argv[(*index)++], 4095, &y)) {
        return false;
    }
    if (strcmp(action, "move") == 0) {
        command->type = NATIVE_POINTER_MOVE;
    } else if (strcmp(action, "down") == 0) {
        command->type = NATIVE_POINTER_DOWN;
    } else if (strcmp(action, "up") == 0) {
        command->type = NATIVE_POINTER_UP;
    } else {
        return false;
    }
    command->x = (int32_t)x;
    command->y = (int32_t)y;
    return true;
}

static bool parse_one_native_command(int argc, char *argv[], int *index,
                                     native_command *command)
{
    const char *name = argv[*index];
    (*index)++;
    *command = (native_command){0};
    if (strcmp(name, "--open") == 0 && *index < argc) {
        command->type = NATIVE_OPEN;
        command->text = argv[*index];
        (*index)++;
        command->text_len = bounded_length(command->text, 256);
        return command->text_len > 0 && command->text_len <= 256;
    }
    if (strcmp(name, "--resize") == 0 && *index + 1 < argc) {
        uint32_t width = 0, height = 0;
        if (!parse_integer(argv[(*index)++], 4096, &width) || width == 0 ||
            !parse_integer(argv[(*index)++], 4096, &height) || height == 0) {
            return false;
        }
        command->type = NATIVE_RESIZE;
        command->x = (int32_t)width;
        command->y = (int32_t)height;
        return true;
    }
    if (strcmp(name, "--key") == 0) {
        return parse_native_key(argc, argv, index, command);
    }
    if (strcmp(name, "--pointer") == 0) {
        return parse_native_pointer(argc, argv, index, command);
    }
    if (strcmp(name, "--quit") == 0) {
        command->type = NATIVE_QUIT;
        return true;
    }
    return false;
}

static bool native_sequence_valid(const native_command *commands, size_t count)
{
    bool resize = false, key_down = false, key_up = false;
    bool pointer_move = false, pointer_down = false, pointer_up = false;
    for (size_t index = 0; index < count; index++) {
        native_command_type type = commands[index].type;
        if ((type == NATIVE_OPEN && index != 0) ||
            (type == NATIVE_QUIT && index + 1 != count)) {
            return false;
        }
        resize |= type == NATIVE_RESIZE;
        key_down |= type == NATIVE_KEY_DOWN;
        key_up |= type == NATIVE_KEY_UP;
        pointer_move |= type == NATIVE_POINTER_MOVE;
        pointer_down |= type == NATIVE_POINTER_DOWN;
        pointer_up |= type == NATIVE_POINTER_UP;
    }
    return count >= 8 && commands[0].type == NATIVE_OPEN &&
        commands[count - 1].type == NATIVE_QUIT && resize && key_down && key_up &&
        pointer_move && pointer_down && pointer_up;
}

static bool parse_native_commands(int argc, char *argv[], unsigned *cycles,
                                  native_command *commands, size_t *count)
{
    uint32_t parsed = 0;
    if (argc < 5 || strcmp(argv[1], "--native-platform-smoke") != 0 ||
        strcmp(argv[2], "--cycles") != 0 || !parse_integer(argv[3], 10000, &parsed) ||
        parsed == 0 || cycles == NULL || commands == NULL || count == NULL) {
        return false;
    }
    *cycles = (unsigned)parsed;
    *count = 0;
    int index = 4;
    while (index < argc) {
        native_command command;
        if (!parse_one_native_command(argc, argv, &index, &command) ||
            !append_command(commands, count, command)) {
                return false;
        }
    }
    return native_sequence_valid(commands, *count);
}

static bool drain_for(platform_window *window, platform_event_type type, uint32_t width,
                      uint32_t height, int32_t expected_x, int32_t expected_y,
                      const char *expected_text)
{
    platform_event event;
    while (platform_poll_event(window, &event)) {
        if (event.type != type) {
            continue;
        }
        if (type == PLATFORM_EVENT_RESIZE || type == PLATFORM_EVENT_SCALE) {
            if (event.pixel_width == 0 || event.pixel_height == 0 ||
                !isfinite(event.backing_scale) || !(event.backing_scale > 0.0)) {
                return false;
            }
            if (type == PLATFORM_EVENT_RESIZE &&
                (fabs((double)event.pixel_width - (double)width * event.backing_scale) > 1.0 ||
                 fabs((double)event.pixel_height - (double)height * event.backing_scale) > 1.0)) {
                return false;
            }
        } else if (type == PLATFORM_EVENT_KEY_DOWN || type == PLATFORM_EVENT_KEY_UP) {
            size_t text_len = bounded_length(expected_text, sizeof(event.text) - 1);
            if (text_len >= sizeof(event.text) || event.key_code != 0 ||
                event.text_len != text_len || event.text[event.text_len] != '\0' ||
                memcmp(event.text, expected_text, text_len) != 0) {
                return false;
            }
        } else if (type == PLATFORM_EVENT_POINTER_MOVE || type == PLATFORM_EVENT_POINTER_DOWN ||
                   type == PLATFORM_EVENT_POINTER_UP) {
            if (event.logical_x != expected_x || event.logical_y != expected_y ||
                event.logical_x < 0 || event.logical_y < 0 ||
                (uint32_t)event.logical_x >= width || (uint32_t)event.logical_y >= height ||
                (type != PLATFORM_EVENT_POINTER_MOVE && event.button != 0)) {
                return false;
            }
        }
        return true;
    }
    return false;
}

static bool execute_native_command(platform_window **window, const native_command *command,
                                   uint32_t *width, uint32_t *height)
{
    platform_event_type event_type = PLATFORM_EVENT_NONE;
    if (command->type == NATIVE_OPEN) {
        platform_window_config config = {command->text, command->text_len, 320, 240, true};
        if (!platform_window_create(&config, window) || *window == NULL) {
            return false;
        }
        *width = 320;
        *height = 240;
        return drain_for(*window, PLATFORM_EVENT_OPEN, *width, *height, 0, 0, "") &&
            drain_for(*window, PLATFORM_EVENT_SCALE, *width, *height, 0, 0, "");
    }
    if (*window == NULL) {
        return false;
    }
    if (command->type == NATIVE_RESIZE) {
        *width = (uint32_t)command->x;
        *height = (uint32_t)command->y;
        return platform_window_resize(*window, command->x, command->y) &&
            drain_for(*window, PLATFORM_EVENT_RESIZE, *width, *height, 0, 0, "");
    }
    if (command->type == NATIVE_KEY_DOWN || command->type == NATIVE_KEY_UP) {
        event_type = command->type == NATIVE_KEY_DOWN ? PLATFORM_EVENT_KEY_DOWN : PLATFORM_EVENT_KEY_UP;
        return platform_test_inject_key(*window, command->type == NATIVE_KEY_DOWN) &&
            drain_for(*window, event_type, *width, *height, 0, 0,
                      command->type == NATIVE_KEY_DOWN ? command->text : "");
    }
    if (command->type >= NATIVE_POINTER_MOVE && command->type <= NATIVE_POINTER_UP) {
        event_type = (platform_event_type)(PLATFORM_EVENT_POINTER_MOVE +
            (command->type - NATIVE_POINTER_MOVE));
        return platform_test_inject_pointer(*window, event_type, command->x, command->y) &&
            drain_for(*window, event_type, *width, *height, command->x, command->y, "");
    }
    return command->type == NATIVE_QUIT && platform_test_request_close(*window) &&
        drain_for(*window, PLATFORM_EVENT_CLOSE_REQUEST, *width, *height, 0, 0, "");
}

static bool run_native_cycle(const native_command *commands, size_t count, unsigned cycle)
{
    platform_window *window = NULL;
    uint32_t width = 0, height = 0;
    for (size_t index = 0; index < count; index++) {
        if (!execute_native_command(&window, &commands[index], &width, &height)) {
            (void)fprintf(stderr, "native smoke: cycle %u command %zu failed\n", cycle, index + 1);
            if (window != NULL) {
                platform_window_destroy(window);
            }
            return false;
        }
        if (commands[index].type == NATIVE_QUIT) {
            platform_window_destroy(window);
            window = NULL;
        }
    }
    return window == NULL;
}

static int native_smoke(unsigned cycles, const native_command *commands, size_t count)
{
    if (!platform_init()) {
        (void)fputs("native smoke: AppKit initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    for (unsigned cycle = 1; cycle <= cycles; cycle++) {
        if (!run_native_cycle(commands, count, cycle)) {
            platform_shutdown();
            return EXIT_FAILURE;
        }
    }
    platform_shutdown();
    (void)puts("native smoke events: open resize scale key pointer close");
    (void)printf("completed cycles: %u\n", cycles);
    return EXIT_SUCCESS;
}

int main(int argc, char *argv[])
{
    if (argc >= 4 && strcmp(argv[1], "--native-platform-smoke") == 0) {
        unsigned cycles = 0;
        size_t count = 0;
        native_command commands[NATIVE_COMMAND_LIMIT] = {0};
        if (!parse_native_commands(argc, argv, &cycles, commands, &count)) {
            (void)fputs("native smoke: malformed lifecycle arguments\n", stderr);
            return EXIT_FAILURE;
        }
        return native_smoke(cycles, commands, count);
    }
    if (argc == 2 && strcmp(argv[1], "--version") == 0) {
        if (puts("svgworkbench 0.0.1") == EOF) {
            return EXIT_FAILURE;
        }
        return EXIT_SUCCESS;
    }

    (void)fputs("usage: svgworkbench --version\n", stderr);
    return EXIT_FAILURE;
}
