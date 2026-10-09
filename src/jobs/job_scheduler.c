#include "jobs/job_scheduler.h"

#include "platform/platform_task.h"

#include <pthread.h>
#include <stdlib.h>

typedef struct job_context {
    jobs_scheduler *scheduler;
    jobs_source_snapshot *snapshot;
    size_t bytes;
} job_context;

struct jobs_scheduler {
    platform_task_queue *queue;
    pthread_t owner;
    pthread_mutex_t accounting_mutex;
    size_t inflight_bytes;
    size_t inflight_limit;
    uint64_t generation;
};

static bool is_owner(const jobs_scheduler *scheduler)
{
    return pthread_equal(scheduler->owner, pthread_self()) != 0;
}

static bool scan_snapshot(void *opaque, const platform_task_cancel_token *token,
                          int64_t *out_value)
{
    const job_context *context = opaque;
    const uint8_t *bytes = jobs_source_snapshot_bytes(context->snapshot);
    const size_t length = jobs_source_snapshot_length(context->snapshot);
    uint32_t digest = UINT32_C(2166136261);
    for (size_t i = 0U; i < length; ++i) {
        if ((i & (64U * 1024U - 1U)) == 0U &&
            atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
        digest = (digest ^ bytes[i]) * UINT32_C(16777619);
    }
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    *out_value = (int64_t)digest;
    return true;
}

static void release_job_context(void *opaque)
{
    job_context *context = opaque;
    jobs_scheduler *scheduler = context->scheduler;
    (void)pthread_mutex_lock(&scheduler->accounting_mutex);
    scheduler->inflight_bytes -= context->bytes;
    (void)pthread_mutex_unlock(&scheduler->accounting_mutex);
    jobs_source_snapshot_release(context->snapshot);
    free(context);
}

jobs_status jobs_scheduler_create(const jobs_scheduler_config *config,
                                  jobs_scheduler **out_scheduler)
{
    if (out_scheduler == NULL) return JOBS_INVALID;
    *out_scheduler = NULL;
    if (config == NULL || config->capacity == 0U || config->capacity > JOBS_SCHEDULER_MAX_CAPACITY ||
        config->worker_count == 0U || config->worker_count > JOBS_SCHEDULER_MAX_WORKERS ||
        config->inflight_bytes_limit == 0U ||
        config->inflight_bytes_limit > JOBS_SNAPSHOT_MAX_BYTES) return JOBS_INVALID;
    jobs_scheduler *scheduler = calloc(1U, sizeof(*scheduler));
    if (scheduler == NULL) return JOBS_NOMEM;
    if (pthread_mutex_init(&scheduler->accounting_mutex, NULL) != 0) {
        free(scheduler);
        return JOBS_NOMEM;
    }
    platform_task_config queue_config = {config->capacity, config->worker_count, NULL, NULL, NULL};
    platform_task_status queue_status = platform_task_queue_create(&queue_config, &scheduler->queue);
    if (queue_status != PLATFORM_TASK_OK) {
        (void)pthread_mutex_destroy(&scheduler->accounting_mutex);
        free(scheduler);
        return queue_status == PLATFORM_TASK_NOMEM ? JOBS_NOMEM : JOBS_INVALID;
    }
    scheduler->owner = pthread_self();
    scheduler->generation = 1U;
    scheduler->inflight_limit = config->inflight_bytes_limit;
    *out_scheduler = scheduler;
    return JOBS_OK;
}

jobs_status jobs_scheduler_submit(jobs_scheduler *scheduler, uint64_t task_id,
                                  uint64_t generation,
                                  jobs_source_snapshot *snapshot)
{
    if (scheduler == NULL || snapshot == NULL || task_id == 0U) return JOBS_INVALID;
    if (!is_owner(scheduler)) return JOBS_WRONG_THREAD;
    if (generation != scheduler->generation) return JOBS_STALE;
    size_t bytes = jobs_source_snapshot_length(snapshot);
    job_context *context = malloc(sizeof(*context));
    if (context == NULL) return JOBS_NOMEM;
    context->scheduler = scheduler;
    context->snapshot = snapshot;
    context->bytes = bytes;

    (void)pthread_mutex_lock(&scheduler->accounting_mutex);
    if (bytes > scheduler->inflight_limit - scheduler->inflight_bytes) {
        (void)pthread_mutex_unlock(&scheduler->accounting_mutex);
        free(context);
        return JOBS_BACKPRESSURE;
    }
    if (!jobs_source_snapshot_retain(snapshot)) {
        (void)pthread_mutex_unlock(&scheduler->accounting_mutex);
        free(context);
        return JOBS_INVALID;
    }
    scheduler->inflight_bytes += bytes;
    platform_task_status status = platform_task_submit_owned(
        scheduler->queue, task_id, generation, scan_snapshot, context, release_job_context);
    if (status != PLATFORM_TASK_OK) {
        scheduler->inflight_bytes -= bytes;
        jobs_source_snapshot_release(snapshot);
    }
    (void)pthread_mutex_unlock(&scheduler->accounting_mutex);
    free(status == PLATFORM_TASK_OK ? NULL : context);
    switch (status) {
        case PLATFORM_TASK_OK: return JOBS_OK;
        case PLATFORM_TASK_FULL: return JOBS_FULL;
        case PLATFORM_TASK_DUPLICATE: return JOBS_DUPLICATE;
        case PLATFORM_TASK_STALE: return JOBS_STALE;
        case PLATFORM_TASK_STOPPED: return JOBS_STOPPED;
        case PLATFORM_TASK_NOMEM: return JOBS_NOMEM;
        default: return JOBS_INVALID;
    }
}

jobs_status jobs_scheduler_cancel(jobs_scheduler *scheduler, uint64_t task_id)
{
    if (scheduler == NULL || task_id == 0U) return JOBS_INVALID;
    if (!is_owner(scheduler)) return JOBS_WRONG_THREAD;
    platform_task_status status = platform_task_cancel(scheduler->queue, task_id);
    return status == PLATFORM_TASK_OK ? JOBS_OK : JOBS_NOT_FOUND;
}

jobs_status jobs_scheduler_advance_generation(jobs_scheduler *scheduler, uint64_t generation)
{
    if (scheduler == NULL) return JOBS_INVALID;
    if (!is_owner(scheduler)) return JOBS_WRONG_THREAD;
    if (generation <= scheduler->generation) return JOBS_STALE;
    platform_task_status status = platform_task_advance_generation(scheduler->queue, generation);
    if (status == PLATFORM_TASK_OK) scheduler->generation = generation;
    return status == PLATFORM_TASK_OK ? JOBS_OK : JOBS_STALE;
}

bool jobs_scheduler_poll(jobs_scheduler *scheduler, jobs_result *out_result)
{
    if (scheduler == NULL || out_result == NULL || !is_owner(scheduler)) return false;
    platform_task_result result;
    while (platform_task_poll(scheduler->queue, &result)) {
        if (result.generation != scheduler->generation) continue;
        *out_result = (jobs_result){result.task_id, result.generation, result.succeeded,
                                    (uint32_t)result.value};
        return true;
    }
    return false;
}

size_t jobs_scheduler_inflight_bytes(jobs_scheduler *scheduler)
{
    if (scheduler == NULL) return 0U;
    (void)pthread_mutex_lock(&scheduler->accounting_mutex);
    size_t bytes = scheduler->inflight_bytes;
    (void)pthread_mutex_unlock(&scheduler->accounting_mutex);
    return bytes;
}

void jobs_scheduler_destroy(jobs_scheduler *scheduler)
{
    if (scheduler == NULL) return;
    platform_task_shutdown(scheduler->queue);
    platform_task_destroy(scheduler->queue);
    (void)pthread_mutex_destroy(&scheduler->accounting_mutex);
    free(scheduler);
}
