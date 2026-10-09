#include "native_task_tests.h"

#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <errno.h>

#include "platform/platform_task.h"

typedef struct gate_context { pthread_mutex_t mutex; pthread_cond_t cond; bool entered; bool release; } gate_context;
typedef struct allocation_context { size_t calls; size_t fail_at; size_t frees; } allocation_context;
static atomic_uint callbacks_done;
static atomic_bool gate_saw_cancel;

static bool immediate_callback(void *context, const platform_task_cancel_token *token, int64_t *out_value)
{
    int64_t *value = context;
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    *out_value = *value;
    (void)atomic_fetch_add_explicit(&callbacks_done, 1U, memory_order_release);
    return true;
}

static bool failed_callback(void *context, const platform_task_cancel_token *token, int64_t *out_value)
{
    (void)context; (void)token;
    *out_value = -1;
    (void)atomic_fetch_add_explicit(&callbacks_done, 1U, memory_order_release);
    return false;
}

static bool gate_callback(void *opaque, const platform_task_cancel_token *token, int64_t *out_value)
{
    gate_context *gate = opaque;
    (void)pthread_mutex_lock(&gate->mutex);
    gate->entered = true;
    (void)pthread_cond_broadcast(&gate->cond);
    while (!gate->release && !atomic_load_explicit(&token->canceled, memory_order_acquire)) {
        struct timespec deadline;
        (void)clock_gettime(CLOCK_REALTIME, &deadline);
        deadline.tv_nsec += 1000000L;
        if (deadline.tv_nsec >= 1000000000L) { deadline.tv_sec++; deadline.tv_nsec -= 1000000000L; }
        int rc = pthread_cond_timedwait(&gate->cond, &gate->mutex, &deadline);
        if (rc != 0 && rc != ETIMEDOUT) break;
    }
    bool canceled = atomic_load_explicit(&token->canceled, memory_order_acquire);
    (void)pthread_mutex_unlock(&gate->mutex);
    atomic_store_explicit(&gate_saw_cancel, canceled, memory_order_release);
    *out_value = 41;
    (void)atomic_fetch_add_explicit(&callbacks_done, 1U, memory_order_release);
    return !canceled;
}

static void *test_allocate(void *opaque, size_t size)
{
    allocation_context *context = opaque;
    size_t call = context->calls++;
    return call == context->fail_at ? NULL : calloc(1U, size);
}

static void test_deallocate(void *opaque, void *memory)
{
    allocation_context *context = opaque;
    context->frees++;
    free(memory);
}

static double monotonic_seconds(void)
{
    struct timespec time_value;
    if (clock_gettime(CLOCK_MONOTONIC, &time_value) != 0) return 0.0;
    return (double)time_value.tv_sec + (double)time_value.tv_nsec / 1000000000.0;
}

static bool create_queue(size_t capacity, size_t workers, platform_task_queue **queue)
{
    platform_task_config config = {capacity, workers, NULL, NULL, NULL};
    return platform_task_queue_create(&config, queue) == PLATFORM_TASK_OK;
}

static bool wait_for_gate(gate_context *gate)
{
    (void)pthread_mutex_lock(&gate->mutex);
    while (!gate->entered) (void)pthread_cond_wait(&gate->cond, &gate->mutex);
    (void)pthread_mutex_unlock(&gate->mutex);
    return true;
}

static void wake_gate(gate_context *gate)
{
    (void)pthread_mutex_lock(&gate->mutex);
    gate->release = true;
    (void)pthread_cond_broadcast(&gate->cond);
    (void)pthread_mutex_unlock(&gate->mutex);
}

static bool wait_for_callbacks(unsigned count)
{
    for (unsigned i = 0U; i < 1000U; ++i) {
        if (atomic_load_explicit(&callbacks_done, memory_order_acquire) >= count) return true;
        struct timespec pause = {0, 1000000L};
        (void)nanosleep(&pause, NULL);
    }
    return atomic_load_explicit(&callbacks_done, memory_order_acquire) >= count;
}

static bool test_config_and_allocations(void)
{
    platform_task_queue *queue = NULL;
    platform_task_config invalid = {65U, 1U, NULL, NULL, NULL};
    if (platform_task_queue_create(&invalid, &queue) != PLATFORM_TASK_INVALID || queue != NULL) return false;
    invalid = (platform_task_config){1U, 5U, NULL, NULL, NULL};
    if (platform_task_queue_create(&invalid, &queue) != PLATFORM_TASK_INVALID || queue != NULL) return false;
    for (size_t fail_at = 0U; fail_at < 2U; ++fail_at) {
        allocation_context allocation = {0U, fail_at, 0U};
        platform_task_config config = {2U, 1U, test_allocate, test_deallocate, &allocation};
        if (platform_task_queue_create(&config, &queue) != PLATFORM_TASK_NOMEM || queue != NULL) return false;
        if (allocation.frees != fail_at) return false;
    }
    return true;
}

static bool test_cancel_and_bounds(void)
{
    platform_task_queue *queue = NULL;
    if (!create_queue(2U, 1U, &queue)) return false;
    int64_t value = 7;
    gate_context gate = {PTHREAD_MUTEX_INITIALIZER, PTHREAD_COND_INITIALIZER, false, false};
    atomic_store_explicit(&callbacks_done, 0U, memory_order_release);
    atomic_store_explicit(&gate_saw_cancel, false, memory_order_release);
    bool valid = platform_task_submit(queue, 1U, 1U, gate_callback, &gate) == PLATFORM_TASK_OK &&
        platform_task_submit(queue, 1U, 1U, gate_callback, &gate) == PLATFORM_TASK_DUPLICATE &&
        wait_for_gate(&gate) && platform_task_submit(queue, 2U, 1U, immediate_callback, &value) == PLATFORM_TASK_OK &&
        platform_task_submit(queue, 3U, 1U, immediate_callback, &value) == PLATFORM_TASK_FULL &&
        platform_task_cancel(queue, 2U) == PLATFORM_TASK_OK &&
        platform_task_cancel(queue, 1U) == PLATFORM_TASK_OK;
    wake_gate(&gate);
    bool finished = wait_for_callbacks(1U);
    valid = valid && atomic_load_explicit(&callbacks_done, memory_order_acquire) == 1U &&
        atomic_load_explicit(&gate_saw_cancel, memory_order_acquire) && finished;
    platform_task_result result;
    if (valid) {
        for (unsigned i = 0U; i < 10000U && platform_task_poll(queue, &result); ++i) { }
        valid = !platform_task_poll(queue, &result);
    }
    platform_task_destroy(queue);
    (void)pthread_cond_destroy(&gate.cond);
    (void)pthread_mutex_destroy(&gate.mutex);
    return valid;
}

static bool init_gate(gate_context *gate)
{
    if (pthread_mutex_init(&gate->mutex, NULL) != 0) return false;
    if (pthread_cond_init(&gate->cond, NULL) != 0) {
        (void)pthread_mutex_destroy(&gate->mutex);
        return false;
    }
    gate->entered = false;
    gate->release = false;
    return true;
}

static void destroy_gate(gate_context *gate)
{
    (void)pthread_cond_destroy(&gate->cond);
    (void)pthread_mutex_destroy(&gate->mutex);
}

static bool test_generation_and_result(void)
{
    platform_task_queue *queue = NULL;
    if (!create_queue(2U, 1U, &queue)) return false;
    gate_context gate;
    if (!init_gate(&gate)) { platform_task_destroy(queue); return false; }
    int64_t value = 99;
    platform_task_result result;
    bool valid;
    atomic_store_explicit(&gate_saw_cancel, false, memory_order_release);
    atomic_store_explicit(&callbacks_done, 0U, memory_order_release);
    valid = platform_task_submit(queue, 10U, 1U, gate_callback, &gate) == PLATFORM_TASK_OK &&
        wait_for_gate(&gate) && platform_task_advance_generation(queue, 2U) == PLATFORM_TASK_OK;
    bool finished = wait_for_callbacks(1U);
    valid = valid && atomic_load_explicit(&callbacks_done, memory_order_acquire) == 1U &&
        atomic_load_explicit(&gate_saw_cancel, memory_order_acquire) && finished;
    struct timespec quiet = {0, 5000000L};
    (void)nanosleep(&quiet, NULL);
    valid = valid && !platform_task_poll(queue, &result);
    atomic_store_explicit(&callbacks_done, 0U, memory_order_release);
    valid = valid && platform_task_submit(queue, 12U, 1U, immediate_callback, &value) == PLATFORM_TASK_STALE &&
        platform_task_submit(queue, 11U, 2U, immediate_callback, &value) == PLATFORM_TASK_OK;
    finished = wait_for_callbacks(1U);
    valid = valid && atomic_load_explicit(&callbacks_done, memory_order_acquire) == 1U;
    bool found = false;
    for (unsigned i = 0U; valid && i < 10000U && !found; ++i) found = platform_task_poll(queue, &result);
    valid = valid && finished && found && result.task_id == 11U && result.generation == 2U &&
        result.succeeded && result.value == 99;
    destroy_gate(&gate);
    platform_task_destroy(queue);
    return valid;
}

static bool test_callback_failure_result(void)
{
    platform_task_queue *queue = NULL;
    if (!create_queue(1U, 1U, &queue)) return false;
    atomic_store_explicit(&callbacks_done, 0U, memory_order_release);
    bool valid = platform_task_submit(queue, 13U, 1U, failed_callback, NULL) == PLATFORM_TASK_OK &&
        wait_for_callbacks(1U);
    platform_task_result result;
    bool found = false;
    for (unsigned i = 0U; valid && i < 1000U && !found; ++i) {
        found = platform_task_poll(queue, &result);
        if (!found) { struct timespec pause = {0, 1000000L}; (void)nanosleep(&pause, NULL); }
    }
    valid = valid && found && result.task_id == 13U && !result.succeeded && result.value == -1;
    platform_task_destroy(queue);
    return valid;
}

static bool validate_queue_contract(void)
{
    return test_config_and_allocations() && test_cancel_and_bounds() &&
        test_generation_and_result() && test_callback_failure_result();
}

static void *paused_api_call(void *opaque)
{
    platform_task_result result;
    (void)platform_task_poll(opaque, &result);
    return NULL;
}

static bool trigger_death_assertion(void)
{
    platform_task_queue *queue = NULL;
    if (!create_queue(1U, 1U, &queue)) return false;
    pthread_t caller;
    platform_task_test_pause_next_api_call();
    if (pthread_create(&caller, NULL, paused_api_call, queue) != 0) return false;
    if (!platform_task_test_wait_until_paused()) return false;
    platform_task_destroy(queue);
    platform_task_test_release_paused_call();
    (void)pthread_join(caller, NULL);
    return false;
}

bool native_task_smoke(unsigned cycles, bool death_probe)
{
    if (death_probe) return trigger_death_assertion();
    if (!validate_queue_contract()) return false;
    int64_t value = 1;
    double maximum_join = 0.0;
    for (unsigned cycle = 0U; cycle < cycles; ++cycle) {
        platform_task_queue *queue = NULL;
        if (!create_queue(2U, 1U, &queue)) return false;
        if (platform_task_submit(queue, (uint64_t)cycle + 1U, 1U, immediate_callback, &value) != PLATFORM_TASK_OK ||
            platform_task_cancel(queue, (uint64_t)cycle + 1U) != PLATFORM_TASK_OK) {
            platform_task_destroy(queue); return false;
        }
        double start = monotonic_seconds();
        platform_task_destroy(queue);
        double elapsed = monotonic_seconds() - start;
        if (elapsed > maximum_join) maximum_join = elapsed;
    }
    (void)printf("completed lifecycle cycles: %u\ncanceled queued work: pass\ncanceled running work: pass\n"
                 "stale publications: 0\nmaximum join seconds: %.6f\n",
                 cycles, maximum_join);
    return maximum_join < 2.0;
}
