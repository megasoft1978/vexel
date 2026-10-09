#include "shape.h"
#include "shape_render.h"

#include <errno.h>
#include <time.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define INPUT_CAP 4096U

static uint32_t next_u32(uint32_t *state)
{
    uint32_t x = *state; x ^= x << 13U; x ^= x >> 17U; x ^= x << 5U; *state = x; return x;
}

int main(int argc, char **argv)
{
    if (argc != 4) return 2;
    char *end = NULL; errno = 0; unsigned long seed_long = strtoul(argv[1], &end, 10);
    if (errno != 0 || end == argv[1] || *end != '\0' || seed_long > UINT32_MAX) return 2;
    uint32_t state = (uint32_t)seed_long;
    errno = 0; unsigned long count = strtoul(argv[2], &end, 10);
    if (errno != 0 || end == argv[2] || *end != '\0' || count > 100000000UL) return 2;
    errno = 0; unsigned long duration = strtoul(argv[3], &end, 10);
    if (errno != 0 || end == argv[3] || *end != '\0' || duration > 600UL) return 2;
    clock_t started = clock();
    static const char *parts[] = {"M", "m", "L", "H", "V", "C", "Q", "A", "Z", " ", ",", "1e999", "nan", "-1000001", "0", "1", "2", "3", "4", "9"};
    static const char prefix[] = "<svg width=\"16\" height=\"16\"><path d=\"";
    static const char suffix[] = "\"/></svg>";
    unsigned long completed = 0;
    for (unsigned long c = 0; c < count; c++) {
        uint8_t input[INPUT_CAP]; size_t n = 0;
        memcpy(input, prefix, sizeof(prefix)-1U); n = sizeof(prefix)-1U;
        size_t tokens = (size_t)(next_u32(&state) % 64U) + 1U;
        for (size_t i = 0; i < tokens; i++) {
            const char *part = parts[next_u32(&state) % (sizeof(parts)/sizeof(parts[0]))]; size_t len = strlen(part);
            if (len > INPUT_CAP - n - (sizeof(suffix)-1U)) break;
            memcpy(input+n, part, len); n += len;
        }
        memcpy(input+n, suffix, sizeof(suffix)-1U); n += sizeof(suffix)-1U;
        svg_shape_document doc; svg_shape_diagnostic diag;
        svg_shape_error error = svg_shape_decode((base_bytes){input,n}, &doc, &diag);
        if (error == SVG_SHAPE_ERROR_NONE) {
            if (doc.width == 0U || doc.width > 4096U || doc.height == 0U || doc.height > 4096U || doc.edge_count > SVG_SHAPE_MAX_EDGES) return 3;
            raster_tile tile;
            if (raster_shape_render(&doc, &tile, &diag)) raster_tile_destroy(&tile);
            svg_shape_document_destroy(&doc);
        } else if (diag.category == SVG_SHAPE_ERROR_NONE || diag.offset > n) return 4;
        completed++;
        if ((completed & 1023UL) == 0UL && duration != 0UL &&
            (double)(clock() - started) / (double)CLOCKS_PER_SEC >= (double)duration) break;
    }
    double seconds = (double)(clock() - started) / (double)CLOCKS_PER_SEC;
    printf("seed=%lu cases=%lu duration=%.3f\n", seed_long, completed, seconds);
    return 0;
}
