#ifndef VEXEL_PLATFORM_TASK_H
#define VEXEL_PLATFORM_TASK_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdatomic.h>

#define PLATFORM_TASK_MAX_CAPACITY 64U
#define PLATFORM_TASK_MAX_WORKERS 4U

typedef struct platform_task_queue platform_task_queue;
typedef struct platform_task_cancel_token { atomic_bool canceled; } platform_task_cancel_token;
typedef bool (*platform_task_callback)(void *context, const platform_task_cancel_token *token,
                                      int64_t *out_value);
typedef void (*platform_task_context_cleanup)(void *context);
typedef void *(*platform_task_alloc_fn)(void *context, size_t size);
typedef void (*platform_task_free_fn)(void *context, void *memory);

typedef struct platform_task_config {
    size_t capacity;
    size_t worker_count;
    platform_task_alloc_fn allocate;
    platform_task_free_fn deallocate;
    void *allocator_context;
} platform_task_config;

typedef enum platform_task_status {
    PLATFORM_TASK_OK = 0,
    PLATFORM_TASK_INVALID,
    PLATFORM_TASK_FULL,
    PLATFORM_TASK_DUPLICATE,
    PLATFORM_TASK_STALE,
    PLATFORM_TASK_NOT_FOUND,
    PLATFORM_TASK_NOMEM,
    PLATFORM_TASK_STOPPED
} platform_task_status;

typedef struct platform_task_result {
    uint64_t task_id;
    uint64_t generation;
    bool succeeded;
    int64_t value;
} platform_task_result;

/* The queue borrows context until queued cancellation or callback return. */
platform_task_status platform_task_queue_create(const platform_task_config *config,
                                                 platform_task_queue **out_queue);
platform_task_status platform_task_submit(platform_task_queue *queue, uint64_t task_id,
                                           uint64_t generation, platform_task_callback callback,
                                           void *context);
/* On success queue owns context and calls cleanup exactly once outside its mutex. */
platform_task_status platform_task_submit_owned(platform_task_queue *queue, uint64_t task_id,
                                                 uint64_t generation,
                                                 platform_task_callback callback,
                                                 void *context,
                                                 platform_task_context_cleanup cleanup);
platform_task_status platform_task_cancel(platform_task_queue *queue, uint64_t task_id);
platform_task_status platform_task_advance_generation(platform_task_queue *queue,
                                                       uint64_t generation);
bool platform_task_poll(platform_task_queue *queue, platform_task_result *out_result);
/* Requests cooperative cancellation and joins every worker before returning. */
void platform_task_shutdown(platform_task_queue *queue);
/* The owner must quiesce callers before destroy; concurrent entry is undefined. */
void platform_task_destroy(platform_task_queue *queue);

/* Deterministic misuse probe used only by the native test driver. */
void platform_task_test_pause_next_api_call(void);
bool platform_task_test_wait_until_paused(void);
void platform_task_test_release_paused_call(void);

#endif
