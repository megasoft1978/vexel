#ifndef VEXEL_SOURCE_SNAPSHOT_H
#define VEXEL_SOURCE_SNAPSHOT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define JOBS_SNAPSHOT_MAX_BYTES (256U * 1024U * 1024U)

typedef struct jobs_source_snapshot jobs_source_snapshot;

typedef enum jobs_snapshot_status {
    JOBS_SNAPSHOT_OK = 0,
    JOBS_SNAPSHOT_INVALID,
    JOBS_SNAPSHOT_LIMIT,
    JOBS_SNAPSHOT_NOMEM
} jobs_snapshot_status;

/* Copies source. NULL is valid only for the empty slice. */
jobs_snapshot_status jobs_source_snapshot_create(const uint8_t *source, size_t length,
                                                  jobs_source_snapshot **out_snapshot);
bool jobs_source_snapshot_retain(jobs_source_snapshot *snapshot);
void jobs_source_snapshot_release(jobs_source_snapshot *snapshot);
const uint8_t *jobs_source_snapshot_bytes(const jobs_source_snapshot *snapshot);
size_t jobs_source_snapshot_length(const jobs_source_snapshot *snapshot);

#endif
