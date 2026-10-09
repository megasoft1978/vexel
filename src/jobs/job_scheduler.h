#ifndef VEXEL_JOB_SCHEDULER_H
#define VEXEL_JOB_SCHEDULER_H

#include "jobs/source_snapshot.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define JOBS_SCHEDULER_MAX_CAPACITY 64U
#define JOBS_SCHEDULER_MAX_WORKERS 4U

typedef struct jobs_scheduler jobs_scheduler;

typedef enum jobs_status {
    JOBS_OK = 0,
    JOBS_INVALID,
    JOBS_FULL,
    JOBS_DUPLICATE,
    JOBS_STALE,
    JOBS_NOT_FOUND,
    JOBS_NOMEM,
    JOBS_STOPPED,
    JOBS_BACKPRESSURE,
    JOBS_WRONG_THREAD
} jobs_status;

typedef struct jobs_scheduler_config {
    size_t capacity;
    size_t worker_count;
    size_t inflight_bytes_limit;
} jobs_scheduler_config;

typedef struct jobs_result {
    uint64_t task_id;
    uint64_t generation;
    bool succeeded;
    uint32_t digest;
} jobs_result;

jobs_status jobs_scheduler_create(const jobs_scheduler_config *config,
                                  jobs_scheduler **out_scheduler);
/* Owner-thread-only. Success retains snapshot; failure leaves caller ownership unchanged. */
jobs_status jobs_scheduler_submit(jobs_scheduler *scheduler, uint64_t task_id,
                                  uint64_t generation,
                                  jobs_source_snapshot *snapshot);
jobs_status jobs_scheduler_cancel(jobs_scheduler *scheduler, uint64_t task_id);
jobs_status jobs_scheduler_advance_generation(jobs_scheduler *scheduler, uint64_t generation);
bool jobs_scheduler_poll(jobs_scheduler *scheduler, jobs_result *out_result);
size_t jobs_scheduler_inflight_bytes(jobs_scheduler *scheduler);
/* Owner must quiesce API calls before destroy; destroy cancels and joins workers. */
void jobs_scheduler_destroy(jobs_scheduler *scheduler);

#endif
