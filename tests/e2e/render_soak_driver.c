#include "app/frame_compose.h"
#include "app/viewer_load.h"
#include "ui/viewer_controls.h"

#include <errno.h>
#include <mach/mach.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define INPUT_PATH_LIMIT 4096U
#define FRAME_WIDTH 32U
#define FRAME_HEIGHT 32U
#define ITERATION_LIMIT 10000U
#define RSS_GROWTH_LIMIT_BYTES (64U * 1024U * 1024U)
#define RSS_ABSOLUTE_LIMIT_BYTES (512U * 1024U * 1024U)

static size_t bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) {
        return 0U;
    }
    while (length <= limit && text[length] != '\0') {
        length++;
    }
    return length;
}

static bool parse_iterations(const char *text, uint32_t *out)
{
    if (text == NULL || text[0] == '\0' || text[0] == '-' || out == NULL) {
        return false;
    }
    errno = 0;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value == 0UL ||
        value > ITERATION_LIMIT) {
        return false;
    }
    *out = (uint32_t)value;
    return true;
}

static bool resident_bytes(uint64_t *out)
{
    if (out == NULL) {
        return false;
    }
    mach_task_basic_info_data_t info = {0};
    mach_msg_type_number_t count = MACH_TASK_BASIC_INFO_COUNT;
    kern_return_t status = task_info(mach_task_self(), MACH_TASK_BASIC_INFO,
                                     (task_info_t)&info, &count);
    if (status != KERN_SUCCESS) {
        return false;
    }
    *out = (uint64_t)info.resident_size;
    return true;
}

static bool render_once(const char *path, uint8_t *frame, size_t frame_size,
                        uint64_t *pixel_checksum)
{
    viewer_loaded_document document = {0};
    if (viewer_load_document(path, NULL, &document) != VIEWER_LOAD_OK) {
        viewer_loaded_document_destroy(&document);
        return false;
    }
    ui_viewer_navigation navigation = {0};
    bool rendered = ui_viewer_navigation_init(&navigation, FRAME_WIDTH, FRAME_HEIGHT,
            (double)document.tile.width, (double)document.tile.height) &&
        ui_viewer_navigation_fit(&navigation) &&
        viewer_compose_frame(&document.tile, &navigation, NULL, VIEWER_FRAME_DOCUMENT,
                             NULL, frame, frame_size, FRAME_WIDTH, FRAME_HEIGHT);
    if (rendered && pixel_checksum != NULL) {
        size_t sample_count = 0U;
        for (size_t offset = 0U; offset < frame_size; offset += 257U) {
            *pixel_checksum = (*pixel_checksum * UINT64_C(1099511628211)) ^ frame[offset];
            sample_count++;
        }
        rendered = sample_count > 0U;
    }
    viewer_loaded_document_destroy(&document);
    return rendered;
}

static bool parse_arguments(int argc, char *argv[], const char **path, uint32_t *iterations)
{
    if (argc < 3 || argv == NULL || path == NULL || iterations == NULL ||
        strcmp(argv[1], "--input") != 0) {
        return false;
    }
    size_t path_length = bounded_length(argv[2], INPUT_PATH_LIMIT);
    if (path_length == 0U || path_length > INPUT_PATH_LIMIT) {
        return false;
    }
    *path = argv[2];
    *iterations = ITERATION_LIMIT;
    if (argc == 3) {
        return true;
    }
    return argc == 5 && strcmp(argv[3], "--iterations") == 0 &&
        parse_iterations(argv[4], iterations);
}

int main(int argc, char *argv[])
{
    const char *path = NULL;
    uint32_t iterations = 0U;
    if (!parse_arguments(argc, argv, &path, &iterations)) {
        (void)fputs("usage: render_soak_driver --input FILE [--iterations 1..10000]\n", stderr);
        return 2;
    }

    uint8_t frame[FRAME_WIDTH * FRAME_HEIGHT * 4U] = {0};
    uint64_t baseline_rss = 0U;
    uint64_t peak_rss = 0U;
    uint64_t checksum = UINT64_C(1469598103934665603);
    for (uint32_t iteration = 0U; iteration < iterations; iteration++) {
        if (!render_once(path, frame, sizeof(frame), &checksum)) {
            (void)fprintf(stderr, "render_soak_driver: render failed at iteration %u for %s\n",
                          iteration, path);
            return 1;
        }
        uint64_t rss = 0U;
        if (!resident_bytes(&rss)) {
            (void)fputs("render_soak_driver: could not read process RSS\n", stderr);
            return 1;
        }
        if (iteration == 0U) {
            baseline_rss = rss;
        }
        if (rss > peak_rss) {
            peak_rss = rss;
        }
    }

    uint64_t growth = peak_rss >= baseline_rss ? peak_rss - baseline_rss : 0U;
    bool passed = baseline_rss > 0U && peak_rss <= RSS_ABSOLUTE_LIMIT_BYTES &&
        growth <= RSS_GROWTH_LIMIT_BYTES;
    (void)printf("{\"schema_version\":1,\"iterations\":%u,\"completed_iterations\":%u,"
                 "\"baseline_rss_bytes\":%llu,\"peak_rss_bytes\":%llu,"
                 "\"growth_bytes\":%llu,\"growth_limit_bytes\":%u,"
                 "\"absolute_limit_bytes\":%u,\"pixel_checksum\":%llu,\"result\":\"%s\"}\n",
                 iterations, iterations, (unsigned long long)baseline_rss,
                 (unsigned long long)peak_rss, (unsigned long long)growth,
                 RSS_GROWTH_LIMIT_BYTES, RSS_ABSOLUTE_LIMIT_BYTES,
                 (unsigned long long)checksum, passed ? "pass" : "fail");
    return passed ? 0 : 1;
}
