#include "shape.h"
#include "shape_render.h"
#include "shape_render_internal.h"

#include <CommonCrypto/CommonDigest.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef VEXEL_W8_FIXTURE_PATH
#error "VEXEL_W8_FIXTURE_PATH must name the pinned large real-world fixture"
#endif

#define W8_FIXTURE_MAX_BYTES (1024U * 1024U)
#define W8_RENDER_SIZE 4096U
#define W8_WORK_LIMIT 200000000U

static const uint8_t W8_PREVIEW_SHA256[CC_SHA256_DIGEST_LENGTH] = {
    0x9eU, 0x0aU, 0xf1U, 0xa5U, 0xd0U, 0x14U, 0xafU, 0x71U,
    0x0fU, 0xabU, 0xb3U, 0x9fU, 0xa1U, 0x52U, 0x73U, 0x3fU,
    0x50U, 0xc6U, 0x62U, 0xaeU, 0x1fU, 0x2dU, 0xc4U, 0xbfU,
    0xe4U, 0xa1U, 0xd7U, 0xcbU, 0x3aU, 0x34U, 0x33U, 0xa4U
};

static bool read_fixture(uint8_t **out_bytes, size_t *out_length)
{
    *out_bytes = NULL;
    *out_length = 0U;
    FILE *file = fopen(VEXEL_W8_FIXTURE_PATH, "rb");
    if (file == NULL) return false;
    bool ok = fseek(file, 0, SEEK_END) == 0;
    long length = ok ? ftell(file) : -1L;
    if (length < 0L || (unsigned long)length > W8_FIXTURE_MAX_BYTES) ok = false;
    if (ok && fseek(file, 0, SEEK_SET) != 0) ok = false;
    size_t size = ok ? (size_t)length : 0U;
    uint8_t *bytes = ok ? malloc(size == 0U ? 1U : size) : NULL;
    if (ok && bytes == NULL) ok = false;
    if (ok && fread(bytes, 1U, size, file) != size) ok = false;
    if (fclose(file) != 0) ok = false;
    if (!ok) {
        free(bytes);
        return false;
    }
    *out_bytes = bytes;
    *out_length = size;
    return true;
}

int main(void)
{
    uint8_t *source = NULL;
    size_t source_length = 0U;
    if (!read_fixture(&source, &source_length)) {
        (void)fprintf(stderr, "large render fixture could not be read\n");
        return 1;
    }
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode((base_bytes){source, source_length}, &document,
                         &diagnostic) != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "large render fixture decode failed: %s:%zu\n",
                      svg_shape_error_name(diagnostic.category),
                      diagnostic.offset);
        free(source);
        return 2;
    }
    free(source);
    if (document.width != 512U || document.height != 512U) {
        (void)fprintf(stderr, "large render fixture dimensions changed\n");
        svg_shape_document_destroy(&document);
        return 3;
    }
    raster_tile preview = {0};
    uint8_t preview_sha256[CC_SHA256_DIGEST_LENGTH] = {0};
    bool preview_rendered = raster_shape_render(&document, &preview, &diagnostic);
    bool preview_matches = preview_rendered &&
        preview.byte_count == (size_t)512U * 512U * 4U;
    bool preview_hashed = preview_matches &&
        CC_SHA256(preview.rgba, (CC_LONG)preview.byte_count,
                  preview_sha256) != NULL;
    if (preview_hashed)
        preview_matches =
            memcmp(preview_sha256, W8_PREVIEW_SHA256,
                   sizeof(W8_PREVIEW_SHA256)) == 0;
    if (!preview_matches) {
        (void)fprintf(stderr, "512x512 map pixels differ from frozen output: %s:%zu; bytes=%zu hash=",
                      svg_shape_error_name(diagnostic.category),
                      diagnostic.offset, preview.byte_count);
        if (preview_hashed) {
            for (size_t index = 0U; index < sizeof(preview_sha256); index++)
                (void)fprintf(stderr, "%02x", preview_sha256[index]);
            for (size_t index = 0U; index < sizeof(preview_sha256); index++) {
                if (preview_sha256[index] != W8_PREVIEW_SHA256[index]) {
                    (void)fprintf(stderr, " mismatch[%zu]=%02x/%02x", index,
                                  preview_sha256[index],
                                  W8_PREVIEW_SHA256[index]);
                    break;
                }
            }
        } else {
            (void)fputs("unavailable", stderr);
        }
        (void)fputc('\n', stderr);
        raster_tile_destroy(&preview);
        svg_shape_document_destroy(&document);
        return 4;
    }
    raster_tile_destroy(&preview);
    document.width = W8_RENDER_SIZE;
    document.height = W8_RENDER_SIZE;
    raster_tile tile = {0};
    raster_shape_work_stats work = {0};
    bool rendered = raster_shape_render_with_stats(&document, &tile,
                                                   &diagnostic, &work);
    size_t expected_bytes = (size_t)W8_RENDER_SIZE * W8_RENDER_SIZE * 4U;
    bool ok = rendered && tile.byte_count == expected_bytes &&
              work.total_work <= W8_WORK_LIMIT;
    if (!ok) {
        (void)fprintf(stderr, "4096x4096 render failed: %s:%zu; work=%llu\n",
                      svg_shape_error_name(diagnostic.category),
                      diagnostic.offset,
                      (unsigned long long)work.total_work);
        raster_tile_destroy(&tile);
        svg_shape_document_destroy(&document);
        return 5;
    }
    (void)printf("4096x4096 map render: work=%llu samples=%llu edges=%llu\n",
                 (unsigned long long)work.total_work,
                 (unsigned long long)work.shape_samples,
                 (unsigned long long)work.fill_edge_tests);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&document);
    return 0;
}
