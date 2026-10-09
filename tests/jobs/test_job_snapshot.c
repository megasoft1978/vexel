#define _DARWIN_C_SOURCE
#include "jobs/job_scheduler.h"
#include "platform/platform_task.h"

#include <pthread.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysctl.h>
#include <time.h>

#define CHECK(expr) do { if (!(expr)) { \
    fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #expr); return false; \
} } while (0)

typedef struct ref_thread_arg {
    jobs_source_snapshot *snapshot;
    const uint8_t *expected;
    size_t length;
} ref_thread_arg;

static void *retain_worker(void *opaque)
{
    ref_thread_arg *arg = opaque;
    for (size_t i = 0U; i < 20000U; ++i) {
        if (!jobs_source_snapshot_retain(arg->snapshot)) return (void *)1;
        const uint8_t *bytes = jobs_source_snapshot_bytes(arg->snapshot);
        if (jobs_source_snapshot_length(arg->snapshot) != arg->length ||
            memcmp(bytes, arg->expected, arg->length) != 0) {
            jobs_source_snapshot_release(arg->snapshot);
            return (void *)1;
        }
        jobs_source_snapshot_release(arg->snapshot);
    }
    jobs_source_snapshot_release(arg->snapshot);
    return NULL;
}

static bool snapshot_tests(void)
{
    jobs_source_snapshot *snapshot = NULL;
    CHECK(jobs_source_snapshot_create(NULL, 1U, &snapshot) == JOBS_SNAPSHOT_INVALID);
    CHECK(snapshot == NULL);
    CHECK(jobs_source_snapshot_create((const uint8_t *)"x",
          (size_t)JOBS_SNAPSHOT_MAX_BYTES + 1U, &snapshot) == JOBS_SNAPSHOT_LIMIT);
    CHECK(jobs_source_snapshot_create(NULL, 0U, &snapshot) == JOBS_SNAPSHOT_OK);
    CHECK(jobs_source_snapshot_length(snapshot) == 0U);
    CHECK(jobs_source_snapshot_bytes(snapshot) != NULL);
    jobs_source_snapshot_release(snapshot);

    uint8_t mutable[4096];
    uint8_t expected[sizeof(mutable)];
    for (size_t i = 0U; i < sizeof(mutable); ++i) mutable[i] = (uint8_t)(i * 29U + 3U);
    memcpy(expected, mutable, sizeof(mutable));
    CHECK(jobs_source_snapshot_create(mutable, sizeof(mutable), &snapshot) == JOBS_SNAPSHOT_OK);
    memset(mutable, 0, sizeof(mutable));
    CHECK(memcmp(jobs_source_snapshot_bytes(snapshot), expected, sizeof(expected)) == 0);

    pthread_t workers[6];
    ref_thread_arg args[6];
    for (size_t i = 0U; i < 6U; ++i) {
        CHECK(jobs_source_snapshot_retain(snapshot));
        args[i] = (ref_thread_arg){snapshot, expected, sizeof(expected)};
        CHECK(pthread_create(&workers[i], NULL, retain_worker, &args[i]) == 0);
    }
    for (size_t i = 0U; i < 6U; ++i) {
        void *result = NULL;
        CHECK(pthread_join(workers[i], &result) == 0 && result == NULL);
    }
    jobs_source_snapshot_release(snapshot);
    return true;
}

typedef struct queue_context {
    atomic_bool *gate;
    atomic_size_t *cleanups;
    atomic_bool *started;
    bool waits;
} queue_context;

static bool queue_callback(void *opaque, const platform_task_cancel_token *token,
                           int64_t *out_value)
{
    queue_context *context = opaque;
    if (context->started != NULL)
        atomic_store_explicit(context->started, true, memory_order_release);
    if (context->waits) {
        while (!atomic_load_explicit(context->gate, memory_order_acquire) &&
               !atomic_load_explicit(&token->canceled, memory_order_acquire)) {
            struct timespec delay = {0, 1000000L};
            (void)nanosleep(&delay, NULL);
        }
    }
    *out_value = 17;
    return !atomic_load_explicit(&token->canceled, memory_order_acquire);
}

static void queue_cleanup(void *opaque)
{
    queue_context *context = opaque;
    (void)atomic_fetch_add_explicit(context->cleanups, 1U, memory_order_relaxed);
}

static bool wait_count(atomic_size_t *value, size_t expected)
{
    for (size_t i = 0U; i < 3000U; ++i) {
        if (atomic_load_explicit(value, memory_order_acquire) == expected) return true;
        struct timespec delay = {0, 1000000L};
        (void)nanosleep(&delay, NULL);
    }
    return atomic_load_explicit(value, memory_order_acquire) == expected;
}

static bool wait_started(atomic_bool *value)
{
    for (size_t i = 0U; i < 3000U; ++i) {
        if (atomic_load_explicit(value, memory_order_acquire)) return true;
        struct timespec delay = {0, 1000000L};
        (void)nanosleep(&delay, NULL);
    }
    return atomic_load_explicit(value, memory_order_acquire);
}

static void *shutdown_worker(void *opaque)
{
    platform_task_shutdown(opaque);
    return NULL;
}

static bool queue_cleanup_tests(void)
{
    platform_task_config config = {2U, 1U, NULL, NULL, NULL};
    platform_task_queue *queue = NULL;
    CHECK(platform_task_queue_create(&config, &queue) == PLATFORM_TASK_OK);
    atomic_size_t cleanups;
    atomic_init(&cleanups, 0U);
    atomic_bool gate;
    atomic_init(&gate, false);
    atomic_bool started;
    atomic_init(&started, false);
    queue_context success = {&gate, &cleanups, NULL, false};
    CHECK(platform_task_submit_owned(queue, 1U, 1U, queue_callback, &success,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(wait_count(&cleanups, 1U));
    platform_task_result result;
    CHECK(platform_task_poll(queue, &result) && result.task_id == 1U && result.succeeded);
    puts("owned_cleanup_success=1");

    queue_context blocker = {&gate, &cleanups, &started, true};
    queue_context canceled = {&gate, &cleanups, NULL, false};
    CHECK(platform_task_submit_owned(queue, 2U, 1U, queue_callback, &blocker,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(wait_started(&started));
    CHECK(platform_task_submit_owned(queue, 3U, 1U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(platform_task_submit_owned(queue, 3U, 1U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_DUPLICATE);
    CHECK(platform_task_submit_owned(queue, 9U, 1U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_FULL);
    CHECK(atomic_load_explicit(&cleanups, memory_order_relaxed) == 1U);
    CHECK(platform_task_cancel(queue, 3U) == PLATFORM_TASK_OK);
    atomic_store_explicit(&gate, true, memory_order_release);
    CHECK(wait_count(&cleanups, 3U));
    platform_task_shutdown(queue);
    platform_task_destroy(queue);
    CHECK(atomic_load_explicit(&cleanups, memory_order_relaxed) == 3U);
    puts("owned_cleanup_queued_cancel=1 failed_duplicate_full_cleanup=0");

    config = (platform_task_config){2U, 1U, NULL, NULL, NULL};
    CHECK(platform_task_queue_create(&config, &queue) == PLATFORM_TASK_OK);
    atomic_store_explicit(&gate, false, memory_order_release);
    atomic_store_explicit(&started, false, memory_order_release);
    blocker = (queue_context){&gate, &cleanups, &started, true};
    canceled = (queue_context){&gate, &cleanups, NULL, false};
    CHECK(platform_task_submit_owned(queue, 4U, 1U, queue_callback, &blocker,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(wait_started(&started));
    CHECK(platform_task_submit_owned(queue, 5U, 1U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(platform_task_advance_generation(queue, 2U) == PLATFORM_TASK_OK);
    atomic_store_explicit(&gate, true, memory_order_release);
    CHECK(wait_count(&cleanups, 5U));
    platform_task_destroy(queue);
    CHECK(atomic_load_explicit(&cleanups, memory_order_relaxed) == 5U);
    puts("owned_cleanup_stale_generation=1");

    CHECK(platform_task_queue_create(&config, &queue) == PLATFORM_TASK_OK);
    atomic_store_explicit(&gate, false, memory_order_release);
    atomic_store_explicit(&started, false, memory_order_release);
    blocker = (queue_context){&gate, &cleanups, &started, true};
    canceled = (queue_context){&gate, &cleanups, NULL, false};
    CHECK(platform_task_submit_owned(queue, 6U, 1U, queue_callback, &blocker,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    CHECK(wait_started(&started));
    CHECK(platform_task_submit_owned(queue, 7U, 1U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_OK);
    pthread_t shutdown_thread;
    CHECK(pthread_create(&shutdown_thread, NULL, shutdown_worker, queue) == 0);
    struct timespec delay = {0, 10000000L};
    (void)nanosleep(&delay, NULL);
    atomic_store_explicit(&gate, true, memory_order_release);
    CHECK(pthread_join(shutdown_thread, NULL) == 0);
    platform_task_destroy(queue);
    CHECK(atomic_load_explicit(&cleanups, memory_order_relaxed) == 7U);
    puts("owned_cleanup_shutdown_discard=1");

    CHECK(platform_task_queue_create(&config, &queue) == PLATFORM_TASK_OK);
    canceled = (queue_context){&gate, &cleanups, NULL, false};
    CHECK(platform_task_submit_owned(queue, 8U, 99U, queue_callback, &canceled,
                                     queue_cleanup) == PLATFORM_TASK_STALE);
    CHECK(atomic_load_explicit(&cleanups, memory_order_relaxed) == 7U);
    platform_task_destroy(queue);
    puts("owned_cleanup_failed_submit=0");
    return true;
}

static uint32_t reference_digest(const uint8_t *bytes, size_t length)
{
    uint32_t digest = UINT32_C(2166136261);
    for (size_t i = 0U; i < length; ++i) digest = (digest ^ bytes[i]) * UINT32_C(16777619);
    return digest;
}

static bool scheduler_test(void)
{
    uint8_t source[32768];
    for (size_t i = 0U; i < sizeof(source); ++i) source[i] = (uint8_t)(i * 17U + 11U);
    jobs_source_snapshot *snapshot = NULL;
    CHECK(jobs_source_snapshot_create(source, sizeof(source), &snapshot) == JOBS_SNAPSHOT_OK);
    memset(source, 0, sizeof(source));
    jobs_scheduler_config config = {4U, 1U, sizeof(source)};
    jobs_scheduler *scheduler = NULL;
    CHECK(jobs_scheduler_create(&config, &scheduler) == JOBS_OK);
    jobs_scheduler_config small_budget = {4U, 1U, sizeof(source) - 1U};
    jobs_scheduler *budgeted = NULL;
    CHECK(jobs_scheduler_create(&small_budget, &budgeted) == JOBS_OK);
    CHECK(jobs_scheduler_submit(budgeted, 77U, 1U, snapshot) == JOBS_BACKPRESSURE);
    CHECK(jobs_scheduler_inflight_bytes(budgeted) == 0U);
    CHECK(jobs_source_snapshot_bytes(snapshot) != NULL);
    jobs_scheduler_destroy(budgeted);
    CHECK(jobs_scheduler_submit(scheduler, 1U, 1U, snapshot) == JOBS_OK);
    jobs_source_snapshot_release(snapshot);
    jobs_result result;
    bool found = false;
    for (size_t i = 0U; i < 3000U && !found; ++i) {
        found = jobs_scheduler_poll(scheduler, &result);
        if (!found) { struct timespec delay = {0, 1000000L}; (void)nanosleep(&delay, NULL); }
    }
    CHECK(found && result.task_id == 1U && result.generation == 1U && result.succeeded);
    uint8_t original[32768];
    for (size_t i = 0U; i < sizeof(original); ++i) original[i] = (uint8_t)(i * 17U + 11U);
    CHECK(result.digest == reference_digest(original, sizeof(original)));
    CHECK(jobs_scheduler_inflight_bytes(scheduler) == 0U);
    CHECK(jobs_scheduler_advance_generation(scheduler, 2U) == JOBS_OK);
    CHECK(jobs_scheduler_advance_generation(scheduler, 2U) == JOBS_STALE);
    jobs_scheduler_destroy(scheduler);
    return true;
}

static bool aggregate_budget_test(void)
{
    const size_t bytes = 16U * 1024U * 1024U;
    uint8_t *source = malloc(bytes);
    CHECK(source != NULL);
    memset(source, 0x3c, bytes);
    jobs_source_snapshot *snapshot = NULL;
    CHECK(jobs_source_snapshot_create(source, bytes, &snapshot) == JOBS_SNAPSHOT_OK);
    free(source);
    jobs_scheduler_config config = {8U, 1U, 2U * bytes};
    jobs_scheduler *scheduler = NULL;
    CHECK(jobs_scheduler_create(&config, &scheduler) == JOBS_OK);
    CHECK(jobs_scheduler_submit(scheduler, 20U, 1U, snapshot) == JOBS_OK);
    CHECK(jobs_scheduler_submit(scheduler, 20U, 1U, snapshot) == JOBS_DUPLICATE);
    CHECK(jobs_scheduler_inflight_bytes(scheduler) == bytes);
    CHECK(jobs_scheduler_submit(scheduler, 21U, 1U, snapshot) == JOBS_OK);
    CHECK(jobs_scheduler_inflight_bytes(scheduler) == 2U * bytes);
    CHECK(jobs_scheduler_submit(scheduler, 22U, 1U, snapshot) == JOBS_BACKPRESSURE);
    CHECK(jobs_scheduler_inflight_bytes(scheduler) == 2U * bytes);
    CHECK(jobs_source_snapshot_length(snapshot) == bytes);
    CHECK(jobs_scheduler_cancel(scheduler, 20U) == JOBS_OK);
    CHECK(jobs_scheduler_cancel(scheduler, 21U) == JOBS_OK);
    for (size_t i = 0U; i < 3000U && jobs_scheduler_inflight_bytes(scheduler) != 0U; ++i) {
        struct timespec delay = {0, 1000000L};
        (void)nanosleep(&delay, NULL);
    }
    CHECK(jobs_scheduler_inflight_bytes(scheduler) == 0U);
    jobs_scheduler_destroy(scheduler);
    jobs_source_snapshot_release(snapshot);
    return true;
}

static uint32_t random_next(uint32_t *state)
{
    *state ^= *state << 13U; *state ^= *state >> 17U; *state ^= *state << 5U;
    return *state;
}

static bool randomized_schedules(void)
{
    uint8_t data[4096];
    for (size_t i = 0U; i < sizeof(data); ++i) data[i] = (uint8_t)(i ^ (i >> 3U));
    jobs_source_snapshot *snapshot = NULL;
    CHECK(jobs_source_snapshot_create(data, sizeof(data), &snapshot) == JOBS_SNAPSHOT_OK);
    jobs_scheduler_config config = {32U, 4U, 32U * sizeof(data)};
    jobs_scheduler *scheduler = NULL;
    CHECK(jobs_scheduler_create(&config, &scheduler) == JOBS_OK);
    uint32_t seed = UINT32_C(0x4d334d33);
    uint64_t generation = 1U;
    uint64_t next_id = 1U;
    size_t accepted = 0U;
    size_t canceled = 0U;
    bool delivered[1201] = {false};
    jobs_result result;
    for (size_t step = 0U; step < 1200U; ++step) {
        uint32_t choice = random_next(&seed) % 10U;
        if (choice < 6U) {
            jobs_status status = jobs_scheduler_submit(scheduler, next_id++, generation, snapshot);
            if (status == JOBS_OK) accepted++;
            else CHECK(status == JOBS_FULL || status == JOBS_BACKPRESSURE);
        } else if (choice < 8U) {
            uint64_t id = (next_id > 1U) ? 1U + random_next(&seed) % (next_id - 1U) : 1U;
            jobs_status status = jobs_scheduler_cancel(scheduler, id);
            if (status == JOBS_OK) canceled++;
            else CHECK(status == JOBS_NOT_FOUND);
        } else {
            generation++;
            CHECK(jobs_scheduler_advance_generation(scheduler, generation) == JOBS_OK);
        }
        if ((step & 7U) == 0U) while (jobs_scheduler_poll(scheduler, &result)) {
            CHECK(result.generation == generation);
            CHECK(result.succeeded && result.digest == reference_digest(data, sizeof(data)));
            CHECK(result.task_id < 1201U && !delivered[result.task_id]);
            delivered[result.task_id] = true;
        }
    }
    jobs_scheduler_destroy(scheduler);
    jobs_source_snapshot_release(snapshot);
    printf("random_seed=1295209779 random_submissions=%zu random_cancellations=%zu\n",
           accepted, canceled);
    return true;
}

static double elapsed_seconds(struct timespec start, struct timespec end)
{
    return (double)(end.tv_sec - start.tv_sec) +
           (double)(end.tv_nsec - start.tv_nsec) / 1000000000.0;
}

static bool query_string(const char *key, char *buffer, size_t capacity)
{
    size_t length = capacity;
    return sysctlbyname(key, buffer, &length, NULL, 0U) == 0 && length > 0U;
}

static int run_benchmark(const char *output_path, const char *binary_path)
{
    const size_t cycles = 1000U;
    uint8_t bytes[65536];
    memset(bytes, 0x5a, sizeof(bytes));
    jobs_source_snapshot *snapshot = NULL;
    jobs_scheduler_config config = {1U, 1U, sizeof(bytes)};
    if (jobs_source_snapshot_create(bytes, sizeof(bytes), &snapshot) != JOBS_SNAPSHOT_OK) return 1;
    char model[128] = "unknown";
    char os_version[128] = "unknown";
    (void)query_string("hw.model", model, sizeof(model));
    (void)query_string("kern.osproductversion", os_version, sizeof(os_version));
    struct stat info;
    uint64_t binary_size = stat(binary_path, &info) == 0 ? (uint64_t)info.st_size : 0U;
    struct timespec overall_start, overall_end;
    (void)clock_gettime(CLOCK_MONOTONIC, &overall_start);
    double join_max = 0.0;
    size_t cancellations = 0U;
    for (size_t cycle = 0U; cycle < cycles; ++cycle) {
        jobs_scheduler *scheduler = NULL;
        if (jobs_scheduler_create(&config, &scheduler) != JOBS_OK ||
            jobs_scheduler_submit(scheduler, 1U, 1U, snapshot) != JOBS_OK) {
            jobs_scheduler_destroy(scheduler);
            jobs_source_snapshot_release(snapshot);
            return 1;
        }
        if (jobs_scheduler_cancel(scheduler, 1U) == JOBS_OK) cancellations++;
        struct timespec join_start, join_end;
        (void)clock_gettime(CLOCK_MONOTONIC, &join_start);
        jobs_scheduler_destroy(scheduler);
        (void)clock_gettime(CLOCK_MONOTONIC, &join_end);
        double join_time = elapsed_seconds(join_start, join_end);
        if (join_time > join_max) join_max = join_time;
    }
    (void)clock_gettime(CLOCK_MONOTONIC, &overall_end);
    jobs_source_snapshot_release(snapshot);
    FILE *file = fopen(output_path, "w");
    if (file == NULL) return 1;
    (void)fprintf(file,
        "{\n  \"cycles\": %zu,\n  \"worker_count\": 1,\n  \"elapsed_seconds\": %.9f,\n"
        "  \"join_max_seconds\": %.9f,\n  \"cancellation_count\": %zu,\n"
        "  \"compiler\": \"AppleClang %s\",\n  \"os\": \"macOS %s\",\n"
        "  \"model\": \"%s\",\n  \"architecture\": \"arm64\",\n"
        "  \"build_type\": \"Release\",\n  \"binary_size_bytes\": %llu\n}\n",
        cycles, elapsed_seconds(overall_start, overall_end), join_max, cancellations,
        __clang_version__, os_version, model, (unsigned long long)binary_size);
    int close_status = fclose(file);
    return close_status == 0 && cancellations == cycles && join_max < 2.0 ? 0 : 1;
}

int main(int argc, char **argv)
{
    if (argc == 4 && strcmp(argv[1], "--benchmark") == 0 &&
        strcmp(argv[2], "--output") == 0) return run_benchmark(argv[3], argv[0]);
    if (argc != 1) return 2;
    if (!snapshot_tests() || !queue_cleanup_tests() || !scheduler_test() ||
        !aggregate_budget_test() ||
        !randomized_schedules()) return 1;
    puts("{\"status\":\"pass\",\"snapshot\":\"pass\",\"owned_cleanup_cases\":4,"
         "\"failed_submit_cleanup\":0,\"deterministic_digest\":\"pass\","
         "\"byte_budget\":\"pass\",\"generation_filter\":\"pass\","
         "\"random_seed\":1295209779,\"random_schedule\":\"pass\"}");
    return 0;
}
