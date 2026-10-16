#include "document_edit.h"

#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return false; } } while (0)

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' data-a='old' data-b='zero'>"
    "<rect id='shape' width='4' height='4'/></svg>";

typedef struct snapshot_worker_context {
    const document_edit_snapshot *snapshot;
    const uint8_t *expected;
    size_t length;
    bool okay;
    atomic_size_t completed;
    atomic_size_t released;
} snapshot_worker_context;

static bool set_root_attribute(document_edit *edit, const char *name,
    const char *value)
{
    document_handle root = {0};
    return document_edit_root(edit, &root) && document_edit_set_attribute(edit,
        root, (base_bytes){(const uint8_t *)name, strlen(name)},
        (base_bytes){(const uint8_t *)value, strlen(value)}) == DOCUMENT_EDIT_OK;
}

static void *materialize_worker(void *opaque)
{
    snapshot_worker_context *context = opaque;
    uint8_t *buffer = malloc(context->length == 0U ? 1U : context->length);
    if (buffer == NULL) { context->okay = false; return NULL; }
    context->okay = true;
    for (size_t iteration = 0U; iteration < 1000U; ++iteration) {
        if (iteration < 60U) {
            while (atomic_load_explicit(&context->released, memory_order_acquire) !=
                iteration) { }
        }
        size_t length = 0U;
        if (!document_edit_snapshot_materialize(context->snapshot, buffer,
                context->length, &length) || length != context->length ||
            memcmp(buffer, context->expected, length) != 0) {
            context->okay = false;
            break;
        }
        if (iteration < 60U) {
            atomic_store_explicit(&context->completed, iteration + 1U,
                memory_order_release);
            while (atomic_load_explicit(&context->released, memory_order_acquire) !=
                iteration + 1U) { }
        }
    }
    free(buffer);
    return NULL;
}

static bool test_snapshot_stability_and_worker_read(void)
{
    document_edit_result created = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    CHECK(created.error == DOCUMENT_EDIT_OK && created.edit != NULL);
    document_edit *edit = created.edit;
    CHECK(set_root_attribute(edit, "data-a", "captured"));
    document_edit_snapshot *snapshot = NULL;
    CHECK(document_edit_snapshot_create(edit, &snapshot) == DOCUMENT_EDIT_OK &&
        snapshot != NULL);
    size_t length = document_edit_snapshot_byte_length(snapshot);
    CHECK(length > 0U);
    uint8_t *expected = malloc(length);
    uint8_t *too_small = malloc(length);
    CHECK(expected != NULL && too_small != NULL);
    size_t materialized = 0U;
    CHECK(document_edit_snapshot_materialize(snapshot, expected, length,
        &materialized) && materialized == length);
    CHECK(!document_edit_snapshot_materialize(snapshot, too_small, length - 1U,
        &materialized) && materialized == 0U);

    snapshot_worker_context context = {.snapshot = snapshot, .expected = expected,
        .length = length, .okay = false};
    atomic_init(&context.completed, 0U);
    atomic_init(&context.released, 0U);
    pthread_t thread;
    CHECK(pthread_create(&thread, NULL, materialize_worker, &context) == 0);
    for (size_t iteration = 0U; iteration < 60U; ++iteration) {
        while (atomic_load_explicit(&context.completed, memory_order_acquire) !=
            iteration + 1U) { }
        char value[32];
        int count = snprintf(value, sizeof(value), "edit-%zu", iteration);
        CHECK(count > 0 && (size_t)count < sizeof(value) &&
            set_root_attribute(edit, "data-b", value));
        atomic_store_explicit(&context.released, iteration + 1U,
            memory_order_release);
    }
    CHECK(pthread_join(thread, NULL) == 0 && context.okay);
    CHECK(document_edit_undo(edit) == DOCUMENT_EDIT_OK);
    CHECK(document_edit_redo(edit) == DOCUMENT_EDIT_OK);
    uint8_t *after_history = malloc(length);
    CHECK(after_history != NULL && document_edit_snapshot_materialize(snapshot,
        after_history, length, &materialized) && materialized == length &&
        memcmp(expected, after_history, length) == 0);

    free(after_history);
    free(too_small);
    free(expected);
    document_edit_snapshot_destroy(snapshot);
    document_edit_destroy(edit);
    return true;
}

static bool test_unedited_snapshot_and_invalid_arguments(void)
{
    document_edit_result created = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    CHECK(created.error == DOCUMENT_EDIT_OK);
    document_edit_snapshot *snapshot = NULL;
    CHECK(document_edit_snapshot_create(created.edit, &snapshot) == DOCUMENT_EDIT_OK);
    uint8_t output[sizeof(source)];
    size_t length = 0U;
    CHECK(document_edit_snapshot_materialize(snapshot, output, sizeof(output), &length) &&
        length == sizeof(source) - 1U && memcmp(output, source, length) == 0);
    CHECK(document_edit_snapshot_create(NULL, &snapshot) ==
        DOCUMENT_EDIT_INVALID_ARGUMENT && snapshot == NULL);
    CHECK(document_edit_snapshot_byte_length(NULL) == 0U);
    document_edit_snapshot_destroy(snapshot);
    document_edit_destroy(created.edit);
    return true;
}

int main(void)
{
    if (!test_snapshot_stability_and_worker_read() ||
        !test_unedited_snapshot_and_invalid_arguments()) return 1;
    (void)puts("document edit snapshot: immutable worker materialization passed");
    return 0;
}
