#ifndef VEXEL_SPATIAL_INDEX_H
#define VEXEL_SPATIAL_INDEX_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SPATIAL_INDEX_MAX_RECORDS 1000000U
#define SPATIAL_INDEX_MAX_BYTES (192U * 1024U * 1024U)

typedef struct spatial_bounds { double x, y, width, height; } spatial_bounds;
typedef struct spatial_record { uint64_t id; spatial_bounds bounds; } spatial_record;
typedef struct spatial_index spatial_index;
typedef enum spatial_status { SPATIAL_STATUS_OK = 0, SPATIAL_STATUS_INVALID, SPATIAL_STATUS_LIMIT, SPATIAL_STATUS_OVERFLOW, SPATIAL_STATUS_OOM } spatial_status;

/* Copies caller records. The immutable result is single-owner and safe for concurrent queries. */
spatial_status spatial_index_create(const spatial_record *records, size_t count, spatial_index **out_index);
void spatial_index_destroy(spatial_index *index);
size_t spatial_index_count(const spatial_index *index);
/* Returns required count. Insufficient capacity writes no results. Query edges are inclusive. */
spatial_status spatial_index_query(const spatial_index *index, spatial_bounds viewport,
                                   size_t *out_record_indices, size_t capacity, size_t *out_count);

#endif
