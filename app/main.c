#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <CommonCrypto/CommonDigest.h>

#include "platform/platform_window.h"
#include "platform/platform_metal.h"
#include "raster/tile.h"
#include "test_driver/native_injection.h"
#include "test_driver/native_task_tests.h"
#include "index_replay.h"
#include "app/render_frame_cli.h"
#include "app/viewer.h"

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
        bool open_event = drain_for(*window, PLATFORM_EVENT_OPEN, *width, *height, 0, 0, "");
        bool scale_event = open_event && drain_for(*window, PLATFORM_EVENT_SCALE,
                                                    *width, *height, 0, 0, "");
        if (!open_event || !scale_event) {
            (void)fprintf(stderr, "native smoke: initial events open=%d scale=%d\n",
                          open_event ? 1 : 0, scale_event ? 1 : 0);
        }
        return open_event && scale_event;
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

static int tile_cpu_smoke(void)
{
    raster_tile tile = {0};
    raster_tile invalid = {0};
    if (!raster_tile_create(4U, 3U, &tile) ||
        raster_tile_create(0U, 1U, &invalid) ||
        raster_tile_create(4097U, 1U, &invalid) ||
        raster_tile_create(UINT32_MAX, UINT32_MAX, &invalid) ||
        tile.byte_count != 48U) {
        raster_tile_destroy(&tile);
        raster_tile_destroy(&invalid);
        return EXIT_FAILURE;
    }
    (void)printf("tile bytes: %zu\nfirst rgba: %u,%u,%u,%u\nlast rgba: %u,%u,%u,%u\n",
        tile.byte_count, tile.rgba[0], tile.rgba[1], tile.rgba[2], tile.rgba[3],
        tile.rgba[44], tile.rgba[45], tile.rgba[46], tile.rgba[47]);
    raster_tile_destroy(&tile);
    return EXIT_SUCCESS;
}

static int native_metal_smoke(void)
{
    if (!platform_init()) {
        (void)fputs("Metal smoke: AppKit initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    const char title[] = "Vexel Metal smoke";
    platform_window_config config = {title, sizeof(title) - 1U, 320, 240, true};
    platform_window *window = NULL;
    if (!platform_window_create(&config, &window)) {
        platform_shutdown();
        (void)fputs("Metal smoke: window creation failed\n", stderr);
        return EXIT_FAILURE;
    }
    char diagnostic[192] = {0};
    unsigned frames = 0;
    raster_tile tile = {0};
    bool valid = raster_tile_create(64U, 48U, &tile) &&
        platform_metal_present_and_verify(window, tile.rgba, tile.byte_count,
            tile.width, tile.height, diagnostic, sizeof(diagnostic));
    if (valid) {
        frames++;
    }
    raster_tile_destroy(&tile);
    if (valid) {
        platform_metal_reset(window);
        valid = platform_window_resize(window, 256, 192) &&
            raster_tile_create(96U, 64U, &tile) &&
            platform_metal_present_and_verify(window, tile.rgba, tile.byte_count,
                tile.width, tile.height, diagnostic, sizeof(diagnostic));
        if (valid) {
            frames++;
        }
    }
    raster_tile_destroy(&tile);
    platform_window_destroy(window);
    platform_shutdown();
    if (!valid) {
        (void)fprintf(stderr, "Metal smoke: %s\n", diagnostic[0] == '\0' ? "frame failed" : diagnostic);
        return EXIT_FAILURE;
    }
    (void)printf("presented frames: %u\nbyte-exact readback: pass\nresize/reset: pass\n", frames);
    return EXIT_SUCCESS;
}

static void print_sha256(const uint8_t digest[32])
{
    for (size_t index = 0; index < 32U; index++) {
        (void)printf("%02x", digest[index]);
    }
}

static int native_metal_replay_smoke(void)
{
    const unsigned repeats = 20U;
    raster_tile tile = {0};
    uint8_t expected_hash[CC_SHA256_DIGEST_LENGTH] = {0};
    char diagnostic[192] = {0};
    bool valid = raster_tile_create(64U, 48U, &tile);
    for (unsigned frame = 0; valid && frame < repeats; frame++) {
        uint8_t frame_hash[CC_SHA256_DIGEST_LENGTH] = {0};
        valid = platform_metal_copy_and_hash(tile.rgba, tile.byte_count,
            tile.width, tile.height, frame_hash, diagnostic, sizeof(diagnostic));
        if (valid && frame == 0U) {
            memcpy(expected_hash, frame_hash, sizeof(expected_hash));
        } else if (valid && memcmp(expected_hash, frame_hash, sizeof(expected_hash)) != 0) {
            (void)snprintf(diagnostic, sizeof(diagnostic), "frame %u readback hash changed", frame);
            valid = false;
        }
    }
    raster_tile_destroy(&tile);
    if (!valid) {
        (void)fprintf(stderr, "Metal replay: %s\n", diagnostic[0] == '\0' ? "setup failed" : diagnostic);
        return EXIT_FAILURE;
    }
    (void)printf("native replay frames: %u\nreadback hashes identical: ", repeats);
    print_sha256(expected_hash);
    (void)printf("\nstable offscreen frame order: frame-0000..frame-0019\n");
    return EXIT_SUCCESS;
}

static int native_navigation_event_smoke(void)
{
    const char title[] = "Vexel navigation event smoke";
    platform_window_config config = {title, sizeof(title) - 1U, 320, 240, true};
    platform_window *window = NULL;
    if (!platform_init() || !platform_window_create(&config, &window)) return EXIT_FAILURE;
    bool scroll = platform_test_inject_scroll(window, 2.0, -120.0, 42.5, 31.25);
    bool magnify = scroll && platform_test_inject_magnify(window, 0.125, 42.5, 31.25);
    bool saw_scroll = false, saw_magnify = false;
    platform_event event;
    while (magnify && platform_poll_event(window, &event)) {
        if (event.type == PLATFORM_EVENT_SCROLL) {
            saw_scroll = event.delta_x == 2.0 && event.delta_y == -120.0 &&
                fabs(event.precise_x - 42.5) < 1e-8 && fabs(event.precise_y - 31.25) < 1e-8 &&
                isfinite(event.backing_scale) && event.backing_scale > 0.0;
        } else if (event.type == PLATFORM_EVENT_MAGNIFY) {
            saw_magnify = fabs(event.magnification - 0.125) < 1e-8 &&
                fabs(event.precise_x - 42.5) < 1e-8 && fabs(event.precise_y - 31.25) < 1e-8 &&
                isfinite(event.backing_scale) && event.backing_scale > 0.0;
        }
    }
    if (!saw_scroll || !saw_magnify) {
        (void)fprintf(stderr, "native event smoke: scroll=%d magnify=%d\n", saw_scroll ? 1 : 0, saw_magnify ? 1 : 0);
        return EXIT_FAILURE;
    }
    (void)puts("native scroll: delta=(2,-120) cursor=(42.5,31.25)");
    (void)puts("native magnify: delta=0.125 cursor=(42.5,31.25)");
    /* This one-shot process exits immediately; lifecycle teardown is covered
     * independently by --native-platform-smoke. */
    return EXIT_SUCCESS;
}

int main(int argc, char *argv[])
{
    if (argc == 1) {
        return app_run_viewer(NULL);
    }
    if (argc >= 2 && strcmp(argv[1], "--render-frame") == 0) {
        return app_run_render_frame(argc, argv);
    }
    if (argc == 3 && strcmp(argv[1], "--native-viewer-smoke") == 0) {
        return app_run_native_viewer_smoke(argv[2]);
    }
    if (argc == 2 && argv[1][0] != '-' &&
        bounded_length(argv[1], 4096U) > 0U && bounded_length(argv[1], 4096U) <= 4096U) {
        return app_run_viewer(argv[1]);
    }
    if (argc == 3 && strcmp(argv[1], "--index-replay") == 0) {
        return app_run_index_replay(argv[2]);
    }
    if (argc == 5 && strcmp(argv[1], "--native-task-smoke") == 0 && strcmp(argv[2], "--cycles") == 0) {
        uint32_t cycles = 0U;
        if (!parse_integer(argv[3], 10000U, &cycles) || cycles == 0U || strcmp(argv[4], "--run") != 0) {
            return EXIT_FAILURE;
        }
        return native_task_smoke((unsigned)cycles, false) ? EXIT_SUCCESS : EXIT_FAILURE;
    }
    if (argc == 2 && strcmp(argv[1], "--native-task-destroy-assert") == 0) {
        return native_task_smoke(0U, true) ? EXIT_SUCCESS : EXIT_FAILURE;
    }
    if (argc == 2 && strcmp(argv[1], "--native-metal-smoke") == 0) {
        return native_metal_smoke();
    }
    if (argc == 2 && strcmp(argv[1], "--native-metal-replay-smoke") == 0) {
        return native_metal_replay_smoke();
    }
    if (argc == 2 && strcmp(argv[1], "--native-navigation-event-smoke") == 0) {
        return native_navigation_event_smoke();
    }
    if (argc == 2 && strcmp(argv[1], "--tile-cpu-smoke") == 0) {
        return tile_cpu_smoke();
    }
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

    (void)fputs("usage: svgworkbench [<file.svg>]\n"
                "       svgworkbench --version\n"
                "       svgworkbench --render-frame <file.svg> --size WxH --out <file.ppm> "
                "[--zoom N] [--pan X,Y] [--region X,Y,W,H]\n", stderr);
    return 2;
}
