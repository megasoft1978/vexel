#include "index.h"
#include "platform/platform_task.h"
#include "raster/shape_render.h"

#include <mach/mach_time.h>
#include <sys/resource.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct preview_task {
    base_mut_bytes source;
    svg_shape_document document;
    raster_tile tile;
} preview_task;

static bool render_preview(void *context, const platform_task_cancel_token *token,
                           int64_t *out_value)
{
    preview_task *task = context;
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    svg_shape_diagnostic diagnostic;
    svg_shape_error error = svg_shape_decode((base_bytes){task->source.ptr, task->source.len},
                                              &task->document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE ||
        !raster_shape_render(&task->document, &task->tile, &diagnostic)) return false;
    *out_value = (int64_t)task->tile.byte_count;
    return true;
}

static uint64_t now_ns(void)
{
    static mach_timebase_info_data_t scale;
    if (scale.denom == 0U) (void)mach_timebase_info(&scale);
    uint64_t ticks = mach_absolute_time();
    return ticks * (uint64_t)scale.numer / (uint64_t)scale.denom;
}

int main(int argc, char **argv)
{
    if (argc != 2) { (void)fprintf(stderr, "usage: svg_index_bench FILE\n"); return 2; }
    FILE *file = fopen(argv[1], "rb");
    if (file == NULL) return 2;
    if (fseek(file, 0, SEEK_END) != 0) { (void)fclose(file); return 2; }
    long file_size = ftell(file);
    if (file_size < 0 || fseek(file, 0, SEEK_SET) != 0) { (void)fclose(file); return 2; }
    size_t size = (size_t)file_size;
    uint8_t *bytes = malloc(size == 0U ? 1U : size);
    if (bytes == NULL) { (void)fclose(file); return 2; }
    uint64_t read_start = now_ns();
    size_t read_count = fread(bytes, 1U, size, file);
    uint64_t read_end = now_ns();
    (void)fclose(file);
    if (read_count != size) { free(bytes); return 2; }
    uint64_t start = now_ns();
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){bytes, size}, 1U, &session) != SVG_INDEX_OK) {
        free(bytes); return 2;
    }
    platform_task_config config = {1U, 1U, NULL, NULL, NULL};
    platform_task_queue *queue = NULL;
    if (platform_task_queue_create(&config, &queue) != PLATFORM_TASK_OK) {
        svg_index_destroy(session); free(bytes); return 2;
    }
    preview_task preview = {0};
    bool submitted = false, rendered = false, task_finished = false;
    bool progress_10 = false, progress_50 = false, progress_90 = false;
    uint64_t first_ns = 0U, max_step_ns = 0U;
    svg_index_status status = SVG_INDEX_OK;
    while (status == SVG_INDEX_OK) {
        uint64_t step_start = now_ns();
        status = svg_index_step(session, SVG_INDEX_MAX_STEP_BYTES);
        svg_index_progress progress = svg_index_get_progress(session);
        size_t percent = progress.source_bytes == 0U ? 0U :
                         progress.bytes_read * 100U / progress.source_bytes;
        progress_10 = progress_10 || percent >= 10U;
        progress_50 = progress_50 || percent >= 50U;
        progress_90 = progress_90 || percent >= 90U;
        if (!submitted && progress.first_preview_ready) {
            if (!svg_index_make_preview_source(session, &preview.source) ||
                platform_task_submit(queue, 1U, 1U, render_preview, &preview) != PLATFORM_TASK_OK) {
                status = SVG_INDEX_INVALID;
            } else submitted = true;
        }
        platform_task_result task_result;
        if (!task_finished && platform_task_poll(queue, &task_result)) {
            task_finished = true;
            rendered = task_result.succeeded;
            if (rendered) first_ns = now_ns() - start;
            else status = SVG_INDEX_INVALID;
        }
        uint64_t elapsed = now_ns() - step_start;
        if (elapsed > max_step_ns) max_step_ns = elapsed;
    }
    uint64_t total_ns = now_ns() - start;
    svg_index_progress progress = svg_index_get_progress(session);
    while (submitted && !task_finished) {
        platform_task_result task_result;
        if (platform_task_poll(queue, &task_result)) {
            task_finished = true;
            rendered = task_result.succeeded;
            if (rendered) first_ns = now_ns() - start;
            break;
        }
    }
    platform_task_shutdown(queue);
    if (status != SVG_INDEX_COMPLETE || first_ns == 0U || first_ns > total_ns || !rendered ||
        !progress_10 || !progress_50 || !progress_90) {
        (void)fprintf(stderr, "index failed: status=%d diagnostic=%d offset=%zu\n",
                      (int)status, (int)progress.diagnostic.category,
                      progress.diagnostic.offset);
        platform_task_destroy(queue);
        raster_tile_destroy(&preview.tile);
        svg_shape_document_destroy(&preview.document);
        free(preview.source.ptr); svg_index_destroy(session); free(bytes); return 1;
    }
    struct rusage usage = {0};
    if (getrusage(RUSAGE_SELF, &usage) != 0) {
        platform_task_destroy(queue);
        raster_tile_destroy(&preview.tile);
        svg_shape_document_destroy(&preview.document);
        free(preview.source.ptr); svg_index_destroy(session); free(bytes);
        return 1;
    }
    (void)printf("{\"source_bytes\":%zu,\"read_ns\":%llu,\"first_preview_ns\":%llu,"
                 "\"complete_index_ns\":%llu,\"max_step_ns\":%llu,"
                 "\"preview_before_index_complete\":true,"
                 "\"progress_10_50_90_complete\":true,"
                 "\"element_count\":%zu,\"index_bytes\":%zu,\"peak_rss_bytes\":%llu}\n",
                 size, (unsigned long long)(read_end - read_start),
                 (unsigned long long)first_ns, (unsigned long long)total_ns,
                 (unsigned long long)max_step_ns, progress.element_count,
                 progress.element_count * sizeof(svg_index_element),
                 (unsigned long long)usage.ru_maxrss);
    svg_index_destroy(session); free(bytes);
    platform_task_destroy(queue);
    raster_tile_destroy(&preview.tile);
    svg_shape_document_destroy(&preview.document);
    free(preview.source.ptr);
    return 0;
}
