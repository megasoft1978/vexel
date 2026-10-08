#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#include "platform/platform_window.h"
#include "test_driver/native_injection.h"

static bool drain_for(platform_window *window, platform_event_type type, uint32_t width,
                      uint32_t height, unsigned *seen)
{
    platform_event event;
    while (platform_poll_event(window, &event)) {
        if (event.type == type) {
            if ((type == PLATFORM_EVENT_RESIZE || type == PLATFORM_EVENT_SCALE) &&
                (event.pixel_width == 0 || event.pixel_height == 0 ||
                 !isfinite(event.backing_scale) || !(event.backing_scale > 0.0))) {
                return false;
            }
            if (type == PLATFORM_EVENT_KEY_DOWN &&
                (event.text_len == 0 || event.text_len >= sizeof(event.text) ||
                 event.text[event.text_len] != '\0' || event.key_code != 0 ||
                 strcmp(event.text, "a") != 0)) {
                return false;
            }
            if ((type == PLATFORM_EVENT_POINTER_MOVE || type == PLATFORM_EVENT_POINTER_DOWN ||
                 type == PLATFORM_EVENT_POINTER_UP) &&
                (event.logical_x < 0 || event.logical_y < 0 ||
                 (width > 0 && (uint32_t)event.logical_x >= width) ||
                 (height > 0 && (uint32_t)event.logical_y >= height) ||
                 event.logical_x != 12 || event.logical_y != 18 ||
                 (type != PLATFORM_EVENT_POINTER_MOVE && event.button != 0))) {
                return false;
            }
            (*seen)++;
            if (type == PLATFORM_EVENT_RESIZE &&
                fabs((double)event.pixel_width - (double)width * event.backing_scale) <= 1.0 &&
                fabs((double)event.pixel_height - (double)height * event.backing_scale) <= 1.0) {
                return true;
            }
            if (type != PLATFORM_EVENT_RESIZE) {
                return true;
            }
        }
    }
    return *seen != 0;
}

static bool verify_native_window(platform_window *window, unsigned cycle)
{
    unsigned seen = 0;
    if (!drain_for(window, PLATFORM_EVENT_OPEN, 0, 0, &seen) ||
        !drain_for(window, PLATFORM_EVENT_SCALE, 0, 0, &seen)) {
        (void)fprintf(stderr, "native smoke: cycle %u missing open or scale event\n", cycle);
        return false;
    }
    if (!platform_window_resize(window, 640, 480)) {
        (void)fprintf(stderr, "native smoke: cycle %u resize request failed\n", cycle);
        return false;
    }
    seen = 0;
    if (!drain_for(window, PLATFORM_EVENT_RESIZE, 640, 480, &seen)) {
        (void)fprintf(stderr, "native smoke: cycle %u missing or malformed resize event\n", cycle);
        return false;
    }
    if (!platform_test_inject_key(window)) {
        (void)fputs("native smoke: key injection failed\n", stderr);
        return false;
    }
    seen = 0;
    if (!drain_for(window, PLATFORM_EVENT_KEY_DOWN, 0, 0, &seen) ||
        !drain_for(window, PLATFORM_EVENT_KEY_UP, 0, 0, &seen)) {
        (void)fprintf(stderr, "native smoke: cycle %u missing key event\n", cycle);
        return false;
    }
    if (!platform_test_inject_pointer(window)) {
        (void)fputs("native smoke: pointer injection failed\n", stderr);
        return false;
    }
    seen = 0;
    if (!drain_for(window, PLATFORM_EVENT_POINTER_MOVE, 640, 480, &seen) ||
        !drain_for(window, PLATFORM_EVENT_POINTER_DOWN, 640, 480, &seen) ||
        !drain_for(window, PLATFORM_EVENT_POINTER_UP, 640, 480, &seen)) {
        (void)fprintf(stderr, "native smoke: cycle %u missing pointer event\n", cycle);
        return false;
    }
    if (!platform_test_request_close(window)) {
        (void)fputs("native smoke: close request failed\n", stderr);
        return false;
    }
    seen = 0;
    if (!drain_for(window, PLATFORM_EVENT_CLOSE_REQUEST, 0, 0, &seen)) {
        (void)fprintf(stderr, "native smoke: cycle %u missing close request\n", cycle);
        return false;
    }
    return true;
}

static bool native_smoke_cycle(unsigned cycle)
{
    static const char title[] = "Vexel native smoke";
    platform_window_config config = {title, sizeof(title) - 1, 320, 240, true};
    platform_window *window = NULL;
    if (!platform_window_create(&config, &window) || window == NULL) {
        (void)fprintf(stderr, "native smoke: cycle %u window creation failed\n", cycle);
        return false;
    }
    bool result = verify_native_window(window, cycle);
    platform_window_destroy(window);
    return result;
}

static int native_smoke(unsigned cycles)
{
    if (cycles == 0 || cycles > 10000 || !platform_init()) {
        (void)fputs("native smoke: invalid cycle count or AppKit initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    for (unsigned cycle = 1; cycle <= cycles; cycle++) {
        if (!native_smoke_cycle(cycle)) {
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
    if (argc == 4 && strcmp(argv[1], "--native-platform-smoke") == 0 &&
        strcmp(argv[2], "--cycles") == 0) {
        char *end = NULL;
        unsigned long parsed = strtoul(argv[3], &end, 10);
        if (end == argv[3] || *end != '\0' || parsed > 10000) {
            (void)fputs("native smoke: --cycles requires an integer from 1 to 10000\n", stderr);
            return EXIT_FAILURE;
        }
        return native_smoke((unsigned)parsed);
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
