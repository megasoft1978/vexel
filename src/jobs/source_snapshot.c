#include "jobs/source_snapshot.h"

#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>

struct jobs_source_snapshot {
    atomic_size_t references;
    size_t length;
    uint8_t bytes[];
};

jobs_snapshot_status jobs_source_snapshot_create(const uint8_t *source, size_t length,
                                                  jobs_source_snapshot **out_snapshot)
{
    if (out_snapshot == NULL) return JOBS_SNAPSHOT_INVALID;
    *out_snapshot = NULL;
    if (source == NULL && length != 0U) return JOBS_SNAPSHOT_INVALID;
    if (length > JOBS_SNAPSHOT_MAX_BYTES) return JOBS_SNAPSHOT_LIMIT;
    if (length > SIZE_MAX - sizeof(jobs_source_snapshot)) return JOBS_SNAPSHOT_LIMIT;
    jobs_source_snapshot *snapshot = malloc(sizeof(*snapshot) + length);
    if (snapshot == NULL) return JOBS_SNAPSHOT_NOMEM;
    atomic_init(&snapshot->references, 1U);
    snapshot->length = length;
    if (length != 0U) memcpy(snapshot->bytes, source, length);
    *out_snapshot = snapshot;
    return JOBS_SNAPSHOT_OK;
}

bool jobs_source_snapshot_retain(jobs_source_snapshot *snapshot)
{
    if (snapshot == NULL) return false;
    size_t current = atomic_load_explicit(&snapshot->references, memory_order_relaxed);
    for (;;) {
        if (current == 0U || current == SIZE_MAX) return false;
        if (atomic_compare_exchange_weak_explicit(&snapshot->references, &current, current + 1U,
                                                  memory_order_relaxed, memory_order_relaxed))
            return true;
    }
}

void jobs_source_snapshot_release(jobs_source_snapshot *snapshot)
{
    if (snapshot != NULL &&
        atomic_fetch_sub_explicit(&snapshot->references, 1U, memory_order_acq_rel) == 1U)
        free(snapshot);
}

const uint8_t *jobs_source_snapshot_bytes(const jobs_source_snapshot *snapshot)
{
    return snapshot != NULL ? snapshot->bytes : NULL;
}

size_t jobs_source_snapshot_length(const jobs_source_snapshot *snapshot)
{
    return snapshot != NULL ? snapshot->length : 0U;
}
