#include "platform/platform_task.h"

#include <assert.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>

typedef enum task_state {
    TASK_FREE,
    TASK_QUEUED,
    TASK_CANCELED_QUEUED,
    TASK_RUNNING,
    TASK_COMPLETE,
    TASK_DISCARDED
} task_state;
typedef struct task_slot {
    task_state state;
    uint64_t id;
    uint64_t generation;
    platform_task_callback callback;
    void *context;
    platform_task_cancel_token token;
    platform_task_result result;
} task_slot;

struct platform_task_queue {
    pthread_mutex_t mutex;
    pthread_mutex_t join_mutex;
    pthread_cond_t changed;
    pthread_t workers[PLATFORM_TASK_MAX_WORKERS];
    size_t workers_started;
    size_t capacity;
    size_t queued[PLATFORM_TASK_MAX_CAPACITY];
    size_t queue_head;
    size_t queue_count;
    uint64_t generation;
    bool stopping;
    task_slot *slots;
    platform_task_free_fn deallocate;
    void *allocator_context;
    atomic_size_t active_calls;
};

static pthread_mutex_t test_gate_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t test_gate_cond = PTHREAD_COND_INITIALIZER;
static bool test_gate_armed;
static bool test_gate_paused;
static bool test_gate_released;

static void api_enter(platform_task_queue *queue)
{
    (void)atomic_fetch_add_explicit(&queue->active_calls, 1U, memory_order_acq_rel);
    (void)pthread_mutex_lock(&test_gate_mutex);
    if (test_gate_armed) {
        test_gate_armed = false;
        test_gate_paused = true;
        (void)pthread_cond_broadcast(&test_gate_cond);
        while (!test_gate_released) (void)pthread_cond_wait(&test_gate_cond, &test_gate_mutex);
        test_gate_paused = false;
        test_gate_released = false;
    }
    (void)pthread_mutex_unlock(&test_gate_mutex);
}

static void api_leave(platform_task_queue *queue)
{
    (void)atomic_fetch_sub_explicit(&queue->active_calls, 1U, memory_order_acq_rel);
}

void platform_task_test_pause_next_api_call(void)
{
    (void)pthread_mutex_lock(&test_gate_mutex);
    test_gate_armed = true;
    test_gate_released = false;
    (void)pthread_mutex_unlock(&test_gate_mutex);
}

bool platform_task_test_wait_until_paused(void)
{
    (void)pthread_mutex_lock(&test_gate_mutex);
    while (!test_gate_paused) (void)pthread_cond_wait(&test_gate_cond, &test_gate_mutex);
    (void)pthread_mutex_unlock(&test_gate_mutex);
    return true;
}

void platform_task_test_release_paused_call(void)
{
    (void)pthread_mutex_lock(&test_gate_mutex);
    test_gate_released = true;
    (void)pthread_cond_broadcast(&test_gate_cond);
    (void)pthread_mutex_unlock(&test_gate_mutex);
}

static void *default_allocate(void *context, size_t size)
{
    (void)context;
    return calloc(1U, size);
}

static void default_deallocate(void *context, void *memory)
{
    (void)context;
    free(memory);
}

static void discard_slot(task_slot *slot)
{
    slot->state = TASK_DISCARDED;
    (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
}

static void *worker_main(void *opaque)
{
    platform_task_queue *queue = opaque;
    for (;;) {
        size_t index = 0U;
        task_slot *slot = NULL;
        (void)pthread_mutex_lock(&queue->mutex);
        while (queue->queue_count == 0U && !queue->stopping)
            (void)pthread_cond_wait(&queue->changed, &queue->mutex);
        if (queue->queue_count == 0U && queue->stopping) {
            (void)pthread_mutex_unlock(&queue->mutex);
            return NULL;
        }
        index = queue->queued[queue->queue_head];
        queue->queue_head = (queue->queue_head + 1U) % queue->capacity;
        queue->queue_count--;
        slot = &queue->slots[index];
        if (slot->state == TASK_CANCELED_QUEUED ||
            (slot->state != TASK_QUEUED || slot->generation != queue->generation || queue->stopping)) {
            slot->state = TASK_FREE;
            (void)pthread_mutex_unlock(&queue->mutex);
            continue;
        }
        slot->state = TASK_RUNNING;
        (void)pthread_mutex_unlock(&queue->mutex);

        int64_t value = 0;
        bool succeeded = slot->callback(slot->context, &slot->token, &value);

        (void)pthread_mutex_lock(&queue->mutex);
        if (slot->generation == queue->generation &&
            !atomic_load_explicit(&slot->token.canceled, memory_order_acquire) && !queue->stopping) {
            slot->result = (platform_task_result){slot->id, slot->generation, succeeded, value};
            slot->state = TASK_COMPLETE;
        } else {
            slot->state = TASK_FREE;
        }
        (void)pthread_cond_broadcast(&queue->changed);
        (void)pthread_mutex_unlock(&queue->mutex);
    }
}

platform_task_status platform_task_queue_create(const platform_task_config *config,
                                                 platform_task_queue **out_queue)
{
    if (config == NULL || out_queue == NULL || config->capacity == 0U ||
        config->capacity > PLATFORM_TASK_MAX_CAPACITY || config->worker_count == 0U ||
        config->worker_count > PLATFORM_TASK_MAX_WORKERS ||
        ((config->allocate == NULL) != (config->deallocate == NULL))) return PLATFORM_TASK_INVALID;
    *out_queue = NULL;
    platform_task_alloc_fn allocate = config->allocate != NULL ? config->allocate : default_allocate;
    platform_task_free_fn deallocate = config->deallocate != NULL ? config->deallocate : default_deallocate;
    platform_task_queue *queue = allocate(config->allocator_context, sizeof(*queue));
    if (queue == NULL) return PLATFORM_TASK_NOMEM;
    memset(queue, 0, sizeof(*queue));
    queue->capacity = config->capacity;
    queue->generation = 1U;
    queue->deallocate = deallocate;
    queue->allocator_context = config->allocator_context;
    atomic_init(&queue->active_calls, 0U);
    if (pthread_mutex_init(&queue->mutex, NULL) != 0) { deallocate(config->allocator_context, queue); return PLATFORM_TASK_NOMEM; }
    if (pthread_mutex_init(&queue->join_mutex, NULL) != 0) {
        (void)pthread_mutex_destroy(&queue->mutex); deallocate(config->allocator_context, queue); return PLATFORM_TASK_NOMEM;
    }
    if (pthread_cond_init(&queue->changed, NULL) != 0) {
        (void)pthread_mutex_destroy(&queue->join_mutex); (void)pthread_mutex_destroy(&queue->mutex);
        deallocate(config->allocator_context, queue); return PLATFORM_TASK_NOMEM;
    }
    queue->slots = allocate(config->allocator_context, config->capacity * sizeof(*queue->slots));
    if (queue->slots == NULL) {
        (void)pthread_cond_destroy(&queue->changed); (void)pthread_mutex_destroy(&queue->join_mutex);
        (void)pthread_mutex_destroy(&queue->mutex);
        deallocate(config->allocator_context, queue); return PLATFORM_TASK_NOMEM;
    }
    memset(queue->slots, 0, config->capacity * sizeof(*queue->slots));
    for (size_t i = 0U; i < config->capacity; ++i) atomic_init(&queue->slots[i].token.canceled, false);
    for (size_t i = 0U; i < config->worker_count; ++i) {
        if (pthread_create(&queue->workers[i], NULL, worker_main, queue) != 0) {
            (void)pthread_mutex_lock(&queue->mutex); queue->stopping = true;
            (void)pthread_cond_broadcast(&queue->changed); (void)pthread_mutex_unlock(&queue->mutex);
            for (size_t j = 0U; j < queue->workers_started; ++j) (void)pthread_join(queue->workers[j], NULL);
            deallocate(config->allocator_context, queue->slots);
            (void)pthread_cond_destroy(&queue->changed); (void)pthread_mutex_destroy(&queue->join_mutex);
            (void)pthread_mutex_destroy(&queue->mutex);
            deallocate(config->allocator_context, queue); return PLATFORM_TASK_NOMEM;
        }
        queue->workers_started++;
    }
    *out_queue = queue;
    return PLATFORM_TASK_OK;
}

platform_task_status platform_task_submit(platform_task_queue *queue, uint64_t task_id,
                                           uint64_t generation, platform_task_callback callback,
                                           void *context)
{
    if (queue == NULL || callback == NULL || task_id == 0U) return PLATFORM_TASK_INVALID;
    api_enter(queue);
    (void)pthread_mutex_lock(&queue->mutex);
    platform_task_status status = PLATFORM_TASK_OK;
    task_slot *free_slot = NULL;
    for (size_t i = 0U; i < queue->capacity; ++i) {
        task_slot *slot = &queue->slots[i];
        if (slot->state != TASK_FREE && slot->state != TASK_DISCARDED && slot->id == task_id) {
            status = PLATFORM_TASK_DUPLICATE; break;
        }
        if ((slot->state == TASK_FREE || slot->state == TASK_DISCARDED) && free_slot == NULL) free_slot = slot;
    }
    if (status == PLATFORM_TASK_OK && generation != queue->generation) status = PLATFORM_TASK_STALE;
    if (status == PLATFORM_TASK_OK && queue->stopping) status = PLATFORM_TASK_STOPPED;
    if (status == PLATFORM_TASK_OK && (free_slot == NULL || queue->queue_count == queue->capacity)) {
        status = PLATFORM_TASK_FULL;
    }
    if (status == PLATFORM_TASK_OK) {
        size_t index = (size_t)(free_slot - queue->slots);
        free_slot->id = task_id; free_slot->generation = generation; free_slot->callback = callback;
        free_slot->context = context; free_slot->result = (platform_task_result){0};
        (void)atomic_store_explicit(&free_slot->token.canceled, false, memory_order_release);
        free_slot->state = TASK_QUEUED;
        size_t tail = (queue->queue_head + queue->queue_count) % queue->capacity;
        queue->queued[tail] = index; queue->queue_count++;
        (void)pthread_cond_signal(&queue->changed);
    }
    (void)pthread_mutex_unlock(&queue->mutex); api_leave(queue); return status;
}

platform_task_status platform_task_cancel(platform_task_queue *queue, uint64_t task_id)
{
    if (queue == NULL || task_id == 0U) return PLATFORM_TASK_INVALID;
    api_enter(queue); (void)pthread_mutex_lock(&queue->mutex);
    platform_task_status status = PLATFORM_TASK_NOT_FOUND;
    for (size_t i = 0U; i < queue->capacity; ++i) {
        task_slot *slot = &queue->slots[i];
        if (slot->state != TASK_FREE && slot->state != TASK_DISCARDED && slot->id == task_id) {
            status = PLATFORM_TASK_OK;
            (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
            if (slot->state == TASK_QUEUED) {
                slot->state = TASK_CANCELED_QUEUED;
            }
            else if (slot->state == TASK_COMPLETE) discard_slot(slot);
            break;
        }
    }
    (void)pthread_mutex_unlock(&queue->mutex); api_leave(queue); return status;
}

platform_task_status platform_task_advance_generation(platform_task_queue *queue, uint64_t generation)
{
    if (queue == NULL) return PLATFORM_TASK_INVALID;
    api_enter(queue); (void)pthread_mutex_lock(&queue->mutex);
    platform_task_status status = generation <= queue->generation ? PLATFORM_TASK_STALE : PLATFORM_TASK_OK;
    if (status == PLATFORM_TASK_OK) {
        queue->generation = generation;
        for (size_t i = 0U; i < queue->capacity; ++i) {
            task_slot *slot = &queue->slots[i];
            if (slot->state != TASK_FREE && slot->generation < generation) {
                if (slot->state == TASK_QUEUED) {
                    (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
                    slot->state = TASK_CANCELED_QUEUED;
                }
                else if (slot->state == TASK_COMPLETE) discard_slot(slot);
                else if (slot->state == TASK_RUNNING)
                    (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
            }
        }
        (void)pthread_cond_broadcast(&queue->changed);
    }
    (void)pthread_mutex_unlock(&queue->mutex); api_leave(queue); return status;
}

bool platform_task_poll(platform_task_queue *queue, platform_task_result *out_result)
{
    if (queue == NULL || out_result == NULL) return false;
    api_enter(queue); (void)pthread_mutex_lock(&queue->mutex);
    bool found = false;
    for (size_t i = 0U; i < queue->capacity; ++i) {
        task_slot *slot = &queue->slots[i];
        if (slot->state == TASK_COMPLETE && slot->generation == queue->generation) {
            *out_result = slot->result; slot->state = TASK_FREE; found = true; break;
        }
    }
    (void)pthread_mutex_unlock(&queue->mutex); api_leave(queue); return found;
}

void platform_task_shutdown(platform_task_queue *queue)
{
    if (queue == NULL) return;
    api_enter(queue); (void)pthread_mutex_lock(&queue->mutex);
    queue->stopping = true;
    for (size_t i = 0U; i < queue->capacity; ++i) {
        task_slot *slot = &queue->slots[i];
        if (slot->state == TASK_QUEUED) {
            (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
            slot->state = TASK_CANCELED_QUEUED;
        }
        else if (slot->state == TASK_COMPLETE) discard_slot(slot);
        else if (slot->state == TASK_RUNNING)
            (void)atomic_store_explicit(&slot->token.canceled, true, memory_order_release);
    }
    (void)pthread_cond_broadcast(&queue->changed); (void)pthread_mutex_unlock(&queue->mutex);
    api_leave(queue);
    (void)pthread_mutex_lock(&queue->join_mutex);
    for (size_t i = 0U; i < queue->workers_started; ++i) (void)pthread_join(queue->workers[i], NULL);
    queue->workers_started = 0U;
    (void)pthread_mutex_unlock(&queue->join_mutex);
}

void platform_task_destroy(platform_task_queue *queue)
{
    if (queue == NULL) return;
    assert(atomic_load_explicit(&queue->active_calls, memory_order_acquire) == 0U);
    platform_task_shutdown(queue);
    queue->deallocate(queue->allocator_context, queue->slots);
    (void)pthread_cond_destroy(&queue->changed); (void)pthread_mutex_destroy(&queue->join_mutex);
    (void)pthread_mutex_destroy(&queue->mutex);
    platform_task_free_fn deallocate = queue->deallocate;
    void *allocator_context = queue->allocator_context;
    deallocate(allocator_context, queue);
}
