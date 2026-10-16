#define _POSIX_C_SOURCE 200809L
#include "mcp_export_job.h"

#include "project/project_animation_render.h"
#include "project/project_bundle.h"
#include "project/project_timeline_integration.h"
#include "project/project_timeline_session.h"
#include "raster/tile.h"
#include "svg/shape/shape.h"

svg_shape_media_providers svg_app_media_providers(void);

#include <math.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define JOB_MAX_AXIS 512U
#define JOB_TOTAL_PIXELS 16000000ULL
#define JOB_TIMEOUT_SECONDS 900.0
#ifdef MCP_EXPORT_JOB_TESTING
static pthread_mutex_t test_barrier_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t test_barrier_cond = PTHREAD_COND_INITIALIZER;
static uint32_t test_barrier_frame = UINT32_MAX;
static bool test_barrier_paused;
static bool test_barrier_released;

void mcp_export_job_test_pause_after_frame(uint32_t frame_index)
{
    (void)pthread_mutex_lock(&test_barrier_mutex);
    test_barrier_frame = frame_index;
    test_barrier_paused = false;
    test_barrier_released = false;
    (void)pthread_mutex_unlock(&test_barrier_mutex);
}

bool mcp_export_job_test_wait_until_paused(void)
{
    struct timespec deadline;
    if (clock_gettime(CLOCK_REALTIME, &deadline) != 0) return false;
    deadline.tv_sec += 10;
    (void)pthread_mutex_lock(&test_barrier_mutex);
    int result = 0;
    while (!test_barrier_paused && result == 0)
        result = pthread_cond_timedwait(&test_barrier_cond,
            &test_barrier_mutex, &deadline);
    bool paused = test_barrier_paused;
    (void)pthread_mutex_unlock(&test_barrier_mutex);
    return paused;
}

void mcp_export_job_test_release_frame(void)
{
    (void)pthread_mutex_lock(&test_barrier_mutex);
    test_barrier_released = true;
    test_barrier_frame = UINT32_MAX;
    (void)pthread_cond_broadcast(&test_barrier_cond);
    (void)pthread_mutex_unlock(&test_barrier_mutex);
}

static void test_frame_barrier(uint32_t frame_index)
{
    (void)pthread_mutex_lock(&test_barrier_mutex);
    if (frame_index == test_barrier_frame) {
        test_barrier_paused = true;
        (void)pthread_cond_broadcast(&test_barrier_cond);
        while (!test_barrier_released)
            (void)pthread_cond_wait(&test_barrier_cond, &test_barrier_mutex);
        test_barrier_paused = false;
    }
    (void)pthread_mutex_unlock(&test_barrier_mutex);
}
#endif

typedef struct job_clock { struct timespec start; } job_clock;
struct mcp_export_job {
    pthread_mutex_t mutex;
    mcp_export_snapshot *snapshot;
    double fps;
    uint32_t fps_integer;
    double duration;
    uint32_t frame_count;
    struct timespec started_at;
    bool started_at_valid;
    atomic_bool canceled;
    bool executed;
    mcp_export_job_status status;
    platform_animation_bytes encoded;
};

static double elapsed_seconds(const job_clock *clock)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return JOB_TIMEOUT_SECONDS + 1.0;
    return (double)(now.tv_sec - clock->start.tv_sec) +
        (double)(now.tv_nsec - clock->start.tv_nsec) / 1000000000.0;
}

static bool job_timed_out(const mcp_export_job *job)
{
    if (!job->started_at_valid) return false;
    job_clock clock = {job->started_at};
    return elapsed_seconds(&clock) > JOB_TIMEOUT_SECONDS;
}

static bool is_canceled(const mcp_export_job *job,
                        const platform_task_cancel_token *token)
{
    return atomic_load_explicit(&job->canceled, memory_order_relaxed) ||
        (token != NULL && atomic_load_explicit(&token->canceled,
                                               memory_order_relaxed));
}

typedef struct validation_context {
    const mcp_export_job *job;
    const platform_task_cancel_token *token;
} validation_context;

static bool validation_canceled(void *context)
{
    const validation_context *validation = context;
    return is_canceled(validation->job, validation->token) ||
        job_timed_out(validation->job);
}

static void set_status(mcp_export_job *job, mcp_export_job_state state,
                       mcp_export_job_error error, uint16_t progress)
{
    (void)pthread_mutex_lock(&job->mutex);
    if (progress > job->status.progress_permille)
        job->status.progress_permille = progress;
    if (job->status.progress_permille > 1000U)
        job->status.progress_permille = 1000U;
    job->status.state = state;
    job->status.error = error;
    (void)pthread_mutex_unlock(&job->mutex);
}

static void free_frames(platform_animation_frame *frames, size_t count)
{
    if (frames == NULL) return;
    for (size_t i = 0U; i < count; ++i) free((void *)frames[i].rgba);
    free(frames);
}

bool mcp_export_job_create(mcp_export_snapshot *snapshot, double fps,
                           double duration_seconds, mcp_export_job **out_job)
{
    if (out_job == NULL) return false;
    *out_job = NULL;
    if (snapshot == NULL || !isfinite(fps) || !isfinite(duration_seconds) ||
        fps <= 0.0 || fps > 30.0 || duration_seconds <= 0.0) return false;
    double frames = ceil(duration_seconds * fps);
    if (!isfinite(frames) || frames < 1.0 || frames > 120.0) return false;
    uint32_t fps_integer = (uint32_t)llround(fps);
    if ((double)fps_integer != fps || fps_integer == 0U) return false;
    mcp_export_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) return false;
    if (pthread_mutex_init(&job->mutex, NULL) != 0) { free(job); return false; }
    job->snapshot = snapshot;
    job->fps = fps;
    job->fps_integer = fps_integer;
    job->duration = duration_seconds;
    job->frame_count = (uint32_t)frames;
    atomic_init(&job->canceled, false);
    job->status.state = MCP_EXPORT_JOB_READY;
    *out_job = job;
    return true;
}

static bool run_job(mcp_export_job *job, const platform_task_cancel_token *token,
                    int64_t *out_value)
{
    job_clock clock;
    if (clock_gettime(CLOCK_MONOTONIC, &clock.start) != 0) {
        set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_TIMEOUT, 0U);
        return false;
    }
    job->started_at = clock.start;
    job->started_at_valid = true;
    const project_bundle_view *bundle = mcp_export_snapshot_bundle(job->snapshot);
    if (bundle == NULL || is_canceled(job, token)) goto canceled;
    base_bytes source = bundle->scene_svg;
    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode_with_media(source, &providers, &scene, &diagnostic) !=
        SVG_SHAPE_ERROR_NONE) goto decode_failed;
    if (job_timed_out(job)) goto timeout_scene;
    if (is_canceled(job, token)) goto canceled_scene;
    project_timeline_view timeline = {0};
    project_timeline_context_owner context = {0};
    validation_context validation_state = {job, token};
    project_timeline_integration_result validation =
        project_timeline_validate_bundle_owned(bundle, source,
            validation_canceled, &validation_state, &timeline, &context);
    if (job_timed_out(job)) goto timeout_context;
    if (validation.canceled || is_canceled(job, token)) goto canceled_context;
    if (validation.compatibility != PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 ||
        validation.validation.error != PROJECT_TIMELINE_OK || timeline.empty ||
        timeline.ticks_per_second == 0U || timeline.duration_ticks == 0U)
        goto timeline_failed;
    project_timeline_session *session = NULL;
    project_timeline_edit_result session_result = project_timeline_session_create(
        bundle->timeline_json, &context, project_timeline_edit_default_limits(),
        &session);
    if (session_result.error != PROJECT_TIMELINE_OK || session == NULL)
        goto timeline_failed;
    project_timeline_view session_timeline = {0};
    if (project_timeline_session_decode(session, &session_timeline).error !=
        PROJECT_TIMELINE_OK) goto timeline_failed_session;
    if (job_timed_out(job)) goto timeout_session;
    timeline = session_timeline;
    if (job->duration > (double)timeline.duration_ticks /
                        (double)timeline.ticks_per_second)
        goto duration_failed_session;
    if (scene.width == 0U || scene.height == 0U) goto limit_failed;
    double scale = fmin(1.0, fmin((double)JOB_MAX_AXIS / scene.width,
                                  (double)JOB_MAX_AXIS / scene.height));
    uint32_t width = (uint32_t)ceil((double)scene.width * scale);
    uint32_t height = (uint32_t)ceil((double)scene.height * scale);
    if (width == 0U || height == 0U || width > JOB_MAX_AXIS ||
        height > JOB_MAX_AXIS ||
        (uint64_t)width * height > JOB_TOTAL_PIXELS / job->frame_count)
        goto limit_failed;
    scene.width = width;
    scene.height = height;
    (void)pthread_mutex_lock(&job->mutex);
    job->status.width = width;
    job->status.height = height;
    job->status.frame_count = job->frame_count;
    (void)pthread_mutex_unlock(&job->mutex);
    platform_animation_frame *frames = calloc(job->frame_count, sizeof(*frames));
    if (frames == NULL) goto oom_failed;
    for (uint32_t index = 0U; index < job->frame_count; ++index) {
        if (is_canceled(job, token)) { free_frames(frames, index); goto canceled_session; }
        if (elapsed_seconds(&clock) > JOB_TIMEOUT_SECONDS) {
            free_frames(frames, index); goto timeout_session;
        }
        uint64_t tick = (uint64_t)index * timeline.ticks_per_second /
                        job->fps_integer;
        if (tick >= timeline.duration_ticks || tick > UINT32_MAX) {
            free_frames(frames, index); goto duration_failed_session;
        }
        raster_tile tile = {0};
        project_animation_render_result rendered =
            project_timeline_session_render_frame(session, &scene, bundle,
                                                   (uint32_t)tick, &tile);
        if (rendered.error != PROJECT_ANIMATION_RENDER_OK || tile.rgba == NULL) {
            raster_tile_destroy(&tile);
            free_frames(frames, index);
            goto render_failed_session;
        }
        frames[index] = (platform_animation_frame){tile.rgba, tile.byte_count,
            tile.width, tile.height};
        tile.rgba = NULL;
        tile.byte_count = 0U;
        raster_tile_destroy(&tile);
        uint16_t progress = (uint16_t)(((uint64_t)(index + 1U) * 800U) /
                                       job->frame_count);
        set_status(job, MCP_EXPORT_JOB_RUNNING, MCP_EXPORT_JOB_ERROR_NONE, progress);
#ifdef MCP_EXPORT_JOB_TESTING
        test_frame_barrier(index);
#endif
        if (is_canceled(job, token)) { free_frames(frames, index + 1U); goto canceled_session; }
    }
    if (is_canceled(job, token)) { free_frames(frames, job->frame_count); goto canceled_session; }
    platform_animation_bytes encoded = {0};
    platform_media_status encoded_status = platform_animation_encode_gif(frames,
        job->frame_count, job->fps_integer, &encoded);
    free_frames(frames, job->frame_count);
    if (job_timed_out(job)) {
        platform_animation_bytes_destroy(&encoded); goto timeout_session;
    }
    if (is_canceled(job, token)) {
        platform_animation_bytes_destroy(&encoded); goto canceled_session;
    }
    if (encoded_status != PLATFORM_MEDIA_OK || encoded.bytes == NULL ||
        encoded.length == 0U) { platform_animation_bytes_destroy(&encoded); goto encode_failed_session; }
    (void)pthread_mutex_lock(&job->mutex);
    if (is_canceled(job, token)) {
        (void)pthread_mutex_unlock(&job->mutex);
        platform_animation_bytes_destroy(&encoded);
        goto canceled_session;
    }
    job->encoded = encoded;
    job->status.progress_permille = 1000U;
    job->status.state = MCP_EXPORT_JOB_ENCODED;
    job->status.error = MCP_EXPORT_JOB_ERROR_NONE;
    (void)pthread_mutex_unlock(&job->mutex);
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    svg_shape_document_destroy(&scene);
    if (out_value != NULL) *out_value = (int64_t)encoded.length;
    return true;

encode_failed_session: set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_ENCODE, 0U); goto cleanup_session;
render_failed_session: set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_RENDER, 0U); goto cleanup_session;
duration_failed_session: set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_DURATION, 0U); goto cleanup_session;
timeout_session: set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_TIMEOUT, 0U); goto cleanup_session;
canceled_session: set_status(job, MCP_EXPORT_JOB_CANCELLED, MCP_EXPORT_JOB_ERROR_CANCELLED, 0U); goto cleanup_session;
cleanup_session:
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&context);
    svg_shape_document_destroy(&scene);
    return false;
oom_failed: project_timeline_session_destroy(session); project_timeline_context_owner_destroy(&context); svg_shape_document_destroy(&scene); set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_OUT_OF_MEMORY, 0U); return false;
limit_failed: project_timeline_session_destroy(session); project_timeline_context_owner_destroy(&context); svg_shape_document_destroy(&scene); set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_LIMIT, 0U); return false;
timeline_failed_session: project_timeline_session_destroy(session);
timeline_failed: project_timeline_context_owner_destroy(&context); svg_shape_document_destroy(&scene); set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_TIMELINE, 0U); return false;
timeout_context: project_timeline_context_owner_destroy(&context);
timeout_scene: svg_shape_document_destroy(&scene); set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_TIMEOUT, 0U); return false;
canceled_context: project_timeline_context_owner_destroy(&context);
canceled_scene: svg_shape_document_destroy(&scene);
canceled: set_status(job, MCP_EXPORT_JOB_CANCELLED, MCP_EXPORT_JOB_ERROR_CANCELLED, 0U); return false;
decode_failed: svg_shape_document_destroy(&scene); set_status(job, MCP_EXPORT_JOB_FAILED, MCP_EXPORT_JOB_ERROR_DECODE, 0U); return false;
}

bool mcp_export_job_execute(void *context, const platform_task_cancel_token *token,
                            int64_t *out_value)
{
    mcp_export_job *job = context;
    if (out_value != NULL) *out_value = 0;
    if (job == NULL) return false;
    (void)pthread_mutex_lock(&job->mutex);
    if (job->executed || job->status.state != MCP_EXPORT_JOB_READY) {
        (void)pthread_mutex_unlock(&job->mutex);
        return false;
    }
    job->executed = true;
    job->status.state = MCP_EXPORT_JOB_RUNNING;
    (void)pthread_mutex_unlock(&job->mutex);
    return run_job(job, token, out_value);
}

void mcp_export_job_request_cancel(mcp_export_job *job)
{
    if (job == NULL) return;
    (void)pthread_mutex_lock(&job->mutex);
    atomic_store_explicit(&job->canceled, true, memory_order_relaxed);
    if (job->status.state == MCP_EXPORT_JOB_ENCODED) {
        platform_animation_bytes_destroy(&job->encoded);
        job->status.state = MCP_EXPORT_JOB_CANCELLED;
        job->status.error = MCP_EXPORT_JOB_ERROR_CANCELLED;
    } else if (job->status.state == MCP_EXPORT_JOB_READY) {
        job->status.state = MCP_EXPORT_JOB_CANCELLED;
        job->status.error = MCP_EXPORT_JOB_ERROR_CANCELLED;
    }
    (void)pthread_mutex_unlock(&job->mutex);
}

bool mcp_export_job_get_status(mcp_export_job *job,
                               mcp_export_job_status *out_status)
{
    if (job == NULL || out_status == NULL) return false;
    (void)pthread_mutex_lock(&job->mutex);
    *out_status = job->status;
    (void)pthread_mutex_unlock(&job->mutex);
    return true;
}

bool mcp_export_job_take_encoded(mcp_export_job *job,
                                 platform_animation_bytes *out_bytes)
{
    if (out_bytes == NULL) return false;
    *out_bytes = (platform_animation_bytes){0};
    if (job == NULL) return false;
    (void)pthread_mutex_lock(&job->mutex);
    bool okay = job->status.state == MCP_EXPORT_JOB_ENCODED &&
        !atomic_load_explicit(&job->canceled, memory_order_relaxed) &&
        job->encoded.bytes != NULL && job->encoded.length != 0U;
    if (okay) {
        *out_bytes = job->encoded;
        job->encoded = (platform_animation_bytes){0};
    }
    (void)pthread_mutex_unlock(&job->mutex);
    return okay;
}

void mcp_export_job_destroy(mcp_export_job *job)
{
    if (job == NULL) return;
    platform_animation_bytes_destroy(&job->encoded);
    mcp_export_snapshot_destroy(job->snapshot);
    (void)pthread_mutex_destroy(&job->mutex);
    free(job);
}
