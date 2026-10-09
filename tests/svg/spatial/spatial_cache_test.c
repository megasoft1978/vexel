#include "index.h"
#include "tile_cache.h"

#include <math.h>
#include <float.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { fprintf(stderr, "check failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); return 1; } } while (0)
#define RANDOM_SEED UINT64_C(20261009)

static uint64_t rng_state = RANDOM_SEED;
static uint32_t random_u32(void) {
    rng_state ^= rng_state << 13; rng_state ^= rng_state >> 7; rng_state ^= rng_state << 17;
    return (uint32_t)(rng_state >> 16);
}
static bool overlap(spatial_bounds a, spatial_bounds b) {
    return a.x <= b.x + b.width && a.x + a.width >= b.x && a.y <= b.y + b.height && a.y + a.height >= b.y;
}
static int spatial_tests(void) {
    spatial_index *index = (spatial_index *)(uintptr_t)1;
    CHECK(spatial_index_create(NULL, 1, &index) == SPATIAL_STATUS_INVALID && index == NULL);
    CHECK(spatial_index_create(NULL, 0, &index) == SPATIAL_STATUS_OK && spatial_index_count(index) == 0);
    size_t count = 9, found = 777;
    size_t output[16];
    memset(output, 0x5a, sizeof(output));
    CHECK(spatial_index_query(index, (spatial_bounds){0,0,1,1}, output, 16, &found) == SPATIAL_STATUS_OK && found == 0);
    spatial_index_destroy(index);
    spatial_record invalid[] = {{1,{0,0,-1,1}}, {2,{NAN,0,1,1}}, {3,{DBL_MAX,0,DBL_MAX,1}}, {4,{0,0,1,1}}, {4,{1,1,1,1}}};
    CHECK(spatial_index_create(invalid, 1, &index) == SPATIAL_STATUS_INVALID);
    CHECK(spatial_index_create(invalid + 1, 1, &index) == SPATIAL_STATUS_INVALID);
    CHECK(spatial_index_create(invalid + 2, 1, &index) == SPATIAL_STATUS_INVALID);
    spatial_record spanning[] = {{5,{-DBL_MAX,0,0,1}}, {6,{DBL_MAX,0,0,1}}};
    CHECK(spatial_index_create(spanning, 2, &index) == SPATIAL_STATUS_OVERFLOW);
    CHECK(spatial_index_create(invalid + 3, 2, &index) == SPATIAL_STATUS_INVALID);
    CHECK(spatial_index_create((const spatial_record *)(uintptr_t)1, SPATIAL_INDEX_MAX_RECORDS + 1U, &index) == SPATIAL_STATUS_LIMIT);
    CHECK(spatial_index_query(NULL, (spatial_bounds){0,0,1,1}, output, 16, &found) == SPATIAL_STATUS_INVALID);
    spatial_record edge[] = {{41,{0,0,2,2}}, {42,{2,2,0,0}}, {43,{4,4,1,1}}};
    CHECK(spatial_index_create(edge, 3, &index) == SPATIAL_STATUS_OK);
    found = 0;
    CHECK(spatial_index_query(index, (spatial_bounds){2,2,0,0}, output, 16, &found) == SPATIAL_STATUS_OK && found == 2);
    CHECK(output[0] == 0 && output[1] == 1);
    output[0] = 0x5a5a;
    CHECK(spatial_index_query(index, (spatial_bounds){-1,-1,9,9}, output, 1, &found) == SPATIAL_STATUS_LIMIT && found == 3 && output[0] == 0x5a5a);
    CHECK(spatial_index_query(index, (spatial_bounds){0,0,-1,1}, output, 16, &found) == SPATIAL_STATUS_INVALID);
    spatial_index_destroy(index);
    (void)count;

    enum { RECORDS = 1500, QUERIES = 2000 };
    spatial_record *records = malloc(sizeof(*records) * RECORDS);
    size_t *actual = malloc(sizeof(*actual) * RECORDS);
    size_t *expected = malloc(sizeof(*expected) * RECORDS);
    CHECK(records != NULL && actual != NULL && expected != NULL);
    rng_state = RANDOM_SEED;
    for (size_t i = 0; i < RECORDS; ++i) {
        double x = (double)(random_u32() % 20000U) / 10.0 - 1000.0;
        double y = (double)(random_u32() % 20000U) / 10.0 - 1000.0;
        records[i] = (spatial_record){(uint64_t)i + 1, {x, y, (double)(random_u32() % 200U) / 10.0, (double)(random_u32() % 200U) / 10.0}};
    }
    CHECK(spatial_index_create(records, RECORDS, &index) == SPATIAL_STATUS_OK);
    for (size_t q = 0; q < QUERIES; ++q) {
        spatial_bounds viewport = {(double)(random_u32() % 20000U) / 10.0 - 1000.0,
                                   (double)(random_u32() % 20000U) / 10.0 - 1000.0,
                                   (double)(random_u32() % 300U) / 10.0,
                                   (double)(random_u32() % 300U) / 10.0};
        size_t expected_count = 0, actual_count = 0;
        for (size_t i = 0; i < RECORDS; ++i) if (overlap(records[i].bounds, viewport)) expected[expected_count++] = i;
        CHECK(spatial_index_query(index, viewport, actual, RECORDS, &actual_count) == SPATIAL_STATUS_OK);
        CHECK(actual_count == expected_count);
        CHECK(memcmp(actual, expected, actual_count * sizeof(size_t)) == 0);
    }
    spatial_index_destroy(index); free(records); free(actual); free(expected);
    return 0;
}


int spatial_cache_contract_tests(void);

int main(void) {
    if (spatial_tests() != 0 || spatial_cache_contract_tests() != 0) return 1;
    puts("spatial and tile-cache deterministic checks passed");
    return 0;
}
