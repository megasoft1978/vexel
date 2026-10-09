#include "index_replay.h"

#include "index.h"
#include "index_progress.h"
#include "platform/platform_metal.h"
#include "platform/platform_task.h"
#include "shape.h"
#include "raster/shape_render.h"

#include <stdio.h>
#include <sched.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef struct preview_job {
    base_mut_bytes source;
    svg_shape_document document;
    raster_tile tile;
    atomic_bool tile_ready;
    atomic_bool worker_started;
    atomic_bool allow_render;
    uint64_t generation;
} preview_job;

static bool render_tile(void *opaque, const platform_task_cancel_token *token,
                        int64_t *out_value)
{
    preview_job *job = opaque;
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    svg_shape_diagnostic diagnostic;
    if (svg_shape_decode((base_bytes){job->source.ptr, job->source.len},
                         &job->document, &diagnostic) != SVG_SHAPE_ERROR_NONE ||
        !raster_shape_render(&job->document, &job->tile, &diagnostic)) return false;
    atomic_store_explicit(&job->tile_ready, true, memory_order_release);
    *out_value = (int64_t)job->tile.byte_count;
    return true;
}

static bool render_tile_gated(void *opaque, const platform_task_cancel_token *token,
                              int64_t *out_value)
{
    preview_job *job = opaque;
    atomic_store_explicit(&job->worker_started, true, memory_order_release);
    while (!atomic_load_explicit(&job->allow_render, memory_order_acquire) &&
           !atomic_load_explicit(&token->canceled, memory_order_acquire)) sched_yield();
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    atomic_store_explicit(&job->tile_ready, true, memory_order_release);
    *out_value = 1;
    return true;
}

static bool wait_for_worker(const preview_job *job)
{
    struct timespec start = {0}, now = {0};
    (void)timespec_get(&start, TIME_UTC);
    do {
        if (atomic_load_explicit(&job->worker_started, memory_order_acquire)) return true;
        (void)timespec_get(&now, TIME_UTC);
        if ((now.tv_sec - start.tv_sec) > 5) return false;
        sched_yield();
    } while (true);
}

static bool read_svg(const char *path, uint8_t **out_bytes, size_t *out_length)
{
    *out_bytes = NULL; *out_length = 0U;
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    bool okay = fseek(file, 0, SEEK_END) == 0;
    long size = okay ? ftell(file) : -1L;
    if (size < 0L || (unsigned long)size > SVG_INDEX_MAX_SOURCE_BYTES ||
        fseek(file, 0, SEEK_SET) != 0) okay = false;
    size_t length = okay ? (size_t)size : 0U;
    uint8_t *bytes = okay ? malloc(length == 0U ? 1U : length) : NULL;
    if (bytes == NULL || fread(bytes, 1U, length, file) != length) okay = false;
    (void)fclose(file);
    if (!okay) { free(bytes); return false; }
    *out_bytes = bytes; *out_length = length;
    return true;
}

static void release_preview(preview_job *job)
{
    raster_tile_destroy(&job->tile);
    svg_shape_document_destroy(&job->document);
    free(job->source.ptr);
}

static bool verify_preview_with_metal(const preview_job *job)
{
    char diagnostic[192] = {0};
    uint8_t digest[32] = {0};
    bool valid = platform_metal_copy_and_hash(job->tile.rgba, job->tile.byte_count,
        job->tile.width, job->tile.height, digest, diagnostic, sizeof(diagnostic));
    if (valid) (void)puts("first preview rendered and read back by Metal");
    else (void)fprintf(stderr, "index replay preview: %s\n", diagnostic);
    return valid;
}

static bool verify_stale_worker_rejected(void)
{
    static const uint8_t fixture[] = "<svg><rect width='2' height='2'/></svg>";
    svg_index_session *session = NULL;
    if (svg_index_start((base_bytes){fixture, sizeof(fixture) - 1U}, 41U, &session) != SVG_INDEX_OK)
        return false;
    svg_index_status status = svg_index_step(session, sizeof(fixture));
    preview_job job = {0};
    atomic_init(&job.tile_ready, false);
    atomic_init(&job.worker_started, false);
    atomic_init(&job.allow_render, false);
    platform_task_config config = {1U, 1U, NULL, NULL, NULL};
    platform_task_queue *queue = NULL;
    bool okay = status == SVG_INDEX_COMPLETE ||
        (status == SVG_INDEX_OK && svg_index_get_progress(session).first_preview_ready);
    okay = okay && svg_index_make_preview_source(session, &job.source);
    uint64_t old_generation = svg_index_get_progress(session).generation;
    job.generation = old_generation;
    okay = okay && platform_task_queue_create(&config, &queue) == PLATFORM_TASK_OK;
    okay = okay && platform_task_advance_generation(queue, old_generation) == PLATFORM_TASK_OK;
    okay = okay && platform_task_submit(queue, 1U, old_generation, render_tile_gated, &job) == PLATFORM_TASK_OK;
    okay = okay && wait_for_worker(&job);
    okay = okay && svg_index_advance_generation(session, old_generation + 1U) == SVG_INDEX_OK;
    atomic_store_explicit(&job.allow_render, true, memory_order_release);
    bool finished = false;
    platform_task_result result = {0};
    for (size_t attempt = 0U; queue != NULL && attempt < 1000000U && !finished; ++attempt) {
        finished = platform_task_poll(queue, &result);
        if (!finished) sched_yield();
    }
    bool presented = finished && result.succeeded &&
        svg_index_preview_generation_is_current(session, job.generation);
    okay = okay && finished && result.succeeded && !presented;
    if (queue != NULL) { platform_task_shutdown(queue); platform_task_destroy(queue); }
    release_preview(&job);
    svg_index_destroy(session);
    return okay;
}

static bool verify_canceled_worker_rejected(void)
{
    preview_job job = {0};
    atomic_init(&job.tile_ready, false);
    atomic_init(&job.worker_started, false);
    atomic_init(&job.allow_render, false);
    platform_task_config config = {1U, 1U, NULL, NULL, NULL};
    platform_task_queue *queue = NULL;
    if (platform_task_queue_create(&config, &queue) != PLATFORM_TASK_OK) return false;
    bool okay = platform_task_submit(queue, 1U, 1U, render_tile_gated, &job) == PLATFORM_TASK_OK;
    okay = okay && wait_for_worker(&job);
    okay = okay && platform_task_cancel(queue, 1U) == PLATFORM_TASK_OK;
    platform_task_shutdown(queue);
    platform_task_result result = {0};
    bool presented = platform_task_poll(queue, &result) && result.succeeded;
    platform_task_destroy(queue);
    return okay && !presented && !atomic_load_explicit(&job.tile_ready, memory_order_acquire);
}

int app_run_index_replay(const char *path)
{
    uint8_t *bytes = NULL;
    size_t length = 0U;
    if (!read_svg(path, &bytes, &length)) {
        (void)fprintf(stderr, "index replay: cannot read bounded SVG: %s\n", path);
        return EXIT_FAILURE;
    }
    svg_index_session *session = NULL;
    svg_index_status status = svg_index_start((base_bytes){bytes, length}, 1U, &session);
    platform_task_config queue_config = {1U, 1U, NULL, NULL, NULL};
    platform_task_queue *render_queue = NULL;
    if (status != SVG_INDEX_OK ||
        platform_task_queue_create(&queue_config, &render_queue) != PLATFORM_TASK_OK) {
        svg_index_destroy(session);
        free(bytes); return EXIT_FAILURE;
    }
    preview_job job = {0};
    atomic_init(&job.tile_ready, false);
    ui_index_progress view = {0};
    bool submitted = false, task_finished = false, presented = false, okay = true;
    bool preview_before_complete = false;
    unsigned last_mark = 0U;
    while (status == SVG_INDEX_OK && !view.failed) {
        size_t budget = length / 10U;
        if (budget == 0U) budget = 1U;
        if (budget > SVG_INDEX_MAX_STEP_BYTES) budget = SVG_INDEX_MAX_STEP_BYTES;
        status = svg_index_step(session, budget);
        svg_index_progress progress = svg_index_get_progress(session);
        if (!progress.complete && atomic_load_explicit(&job.tile_ready, memory_order_acquire))
            preview_before_complete = true;
        (void)ui_index_progress_update(&view, &progress);
        unsigned mark = view.percent >= 90U ? 90U : view.percent >= 50U ? 50U : view.percent >= 10U ? 10U : 0U;
        if (mark > last_mark) { (void)printf("progress: %u%%\n", mark); last_mark = mark; }
        if (!submitted && progress.first_preview_ready) {
            if (svg_index_make_preview_source(session, &job.source)) {
                job.generation = progress.generation;
                submitted = platform_task_submit(render_queue, 1U, job.generation,
                                                 render_tile, &job) == PLATFORM_TASK_OK;
            }
            okay = okay && submitted;
        }
        platform_task_result result;
        if (!task_finished && platform_task_poll(render_queue, &result)) {
            task_finished = true;
            if (!result.succeeded || !svg_index_preview_generation_is_current(session, job.generation)) okay = false;
            else {
                presented = verify_preview_with_metal(&job);
                okay = okay && presented;
            }
        }
    }
    while (submitted && !task_finished) {
        platform_task_result result;
        if (platform_task_poll(render_queue, &result)) {
            task_finished = true;
            if (result.succeeded && svg_index_preview_generation_is_current(session, job.generation))
                presented = verify_preview_with_metal(&job);
            okay = okay && result.succeeded && presented;
        }
    }
    platform_task_shutdown(render_queue);
    platform_task_destroy(render_queue);
    bool complete = status == SVG_INDEX_COMPLETE && svg_index_get_progress(session).complete;
    (void)printf("index replay: complete=%s preview=%s before_complete=%s percent=%u\n",
                 complete ? "true" : "false", presented ? "presented" : "none",
                 preview_before_complete ? "true" : "false", view.percent);
    svg_index_destroy(session);
    release_preview(&job);
    free(bytes);
    bool first_preview_gate = preview_before_complete || length < 1024U * 1024U;
    bool stale_rejected = verify_stale_worker_rejected();
    (void)printf("stale_generation_rejected=%s\n", stale_rejected ? "true" : "false");
    bool canceled_rejected = verify_canceled_worker_rejected();
    (void)printf("canceled_worker_rejected=%s\n", canceled_rejected ? "true" : "false");
    (void)printf("preview_state: submitted=%s finished=%s ready=%s\n",
        submitted ? "true" : "false", task_finished ? "true" : "false",
        atomic_load_explicit(&job.tile_ready, memory_order_acquire) ? "true" : "false");
    return okay && complete && first_preview_gate && stale_rejected && canceled_rejected ? EXIT_SUCCESS : EXIT_FAILURE;
}
