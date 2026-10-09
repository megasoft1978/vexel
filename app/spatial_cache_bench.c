#include "index.h"
#include "tile_cache.h"

#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysctl.h>
#include <sys/utsname.h>
#include <time.h>
#include <mach/mach_time.h>

static uint64_t state = UINT64_C(20261009);
static uint32_t random_u32(void) { state ^= state << 13; state ^= state >> 7; state ^= state << 17; return (uint32_t)(state >> 16); }
static double tick_seconds(uint64_t ticks, mach_timebase_info_data_t scale) {
    return ((double)ticks * (double)scale.numer / (double)scale.denom) / 1000000000.0;
}
static int compare_double(const void *a, const void *b) { double x = *(const double *)a, y = *(const double *)b; return x < y ? -1 : x > y ? 1 : 0; }
static bool parse_size(const char *s, size_t *out) { char *end = NULL; errno = 0; unsigned long long v = strtoull(s, &end, 10); if (errno != 0 || end == s || *end != '\0' || v > SIZE_MAX) return false; *out = (size_t)v; return true; }
static bool write_result(const char *path, const spatial_record *records, size_t entries,
                        const size_t *counts, size_t queries, const double *times,
                        double build_seconds, uint64_t seed, size_t binary_bytes,
                        size_t cap_accounted, bool cap_rejected) {
    FILE *f = fopen(path, "w"); if (f == NULL) return false;
    double *sorted = malloc(queries * sizeof(*sorted)); if (sorted == NULL) { fclose(f); return false; }
    memcpy(sorted, times, queries * sizeof(*sorted)); qsort(sorted, queries, sizeof(*sorted), compare_double);
    double p50 = sorted[(queries + 1) / 2 - 1], p95 = sorted[(queries * 95 + 99) / 100 - 1], maximum = sorted[queries - 1];
    struct utsname host; memset(&host, 0, sizeof(host)); (void)uname(&host);
    char os_version[64] = "unknown", model[128] = "unknown", chip[128] = "unknown";
    size_t text_size = sizeof(os_version);
    (void)sysctlbyname("kern.osproductversion", os_version, &text_size, NULL, 0);
    text_size = sizeof(model); (void)sysctlbyname("hw.model", model, &text_size, NULL, 0);
    text_size = sizeof(chip); (void)sysctlbyname("machdep.cpu.brand_string", chip, &text_size, NULL, 0);
#if defined(NDEBUG)
    const char *build_type = "Release";
#else
    const char *build_type = "Debug";
#endif
    int ok = fprintf(f, "{\n  \"work_order\":\"WO-M3-02\",\n  \"build_type\":\"%s\",\n  \"machine\":{\"sysname\":\"%s\",\"os_version\":\"%s\",\"model\":\"%s\",\"chip\":\"%s\",\"kernel\":\"%s\",\"architecture\":\"%s\"},\n  \"compiler\":\"%s\",\n  \"seed\":%" PRIu64 ",\n  \"entries\":%zu,\n  \"queries\":%zu,\n  \"index_build_seconds\":%.9f,\n  \"query_p50_ms\":%.9f,\n  \"query_p95_ms\":%.9f,\n  \"query_max_ms\":%.9f,\n  \"p95_target_ms\":5.0,\n  \"p95_target_met\":%s,\n  \"binary_bytes\":%zu,\n  \"record_count\":%zu,\n  \"result_counts\":[",
        build_type, host.sysname, os_version, model, chip, host.release, host.machine, __clang_version__, seed, entries, queries, build_seconds,
        p50 * 1000.0, p95 * 1000.0, maximum * 1000.0, p95 < 0.005 ? "true" : "false", binary_bytes, entries) < 0 ? 0 : 1;
    for (size_t i = 0; ok && i < queries; ++i) if (fprintf(f, "%s%zu", i == 0 ? "" : ",", counts[i]) < 0) ok = 0;
    if (ok && fprintf(f, "],\n  \"cache_cap_behavior\":{\"cap_bytes\":4096,\"accounted_bytes\":%zu,\"entry_rejected_when_accounted_size_exceeds_cap_without_ownership_transfer\":%s}\n}\n", cap_accounted, cap_rejected ? "true" : "false") < 0) ok = 0;
    free(sorted);
    if (fclose(f) != 0) ok = 0;
    (void)records;
    return ok != 0;
}

int main(int argc, char **argv) {
    size_t entries = 100000, queries = 1000; uint64_t seed = UINT64_C(20261009); const char *output = NULL;
    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "--entries") == 0 && i + 1 < argc) { if (!parse_size(argv[++i], &entries)) return 2; }
        else if (strcmp(argv[i], "--queries") == 0 && i + 1 < argc) { if (!parse_size(argv[++i], &queries)) return 2; }
        else if (strcmp(argv[i], "--seed") == 0 && i + 1 < argc) { size_t value; if (!parse_size(argv[++i], &value)) return 2; seed = (uint64_t)value; }
        else if (strcmp(argv[i], "--output") == 0 && i + 1 < argc) output = argv[++i];
        else { fprintf(stderr, "usage: spatial_cache_bench --entries N --queries N --seed N --output PATH\n"); return 2; }
    }
    if (entries == 0 || entries > SPATIAL_INDEX_MAX_RECORDS || queries == 0 || output == NULL) return 2;
    mach_timebase_info_data_t timebase;
    if (mach_timebase_info(&timebase) != KERN_SUCCESS || timebase.denom == 0) return 1;
    spatial_record *records = malloc(entries * sizeof(*records)); size_t *result_counts = calloc(queries, sizeof(*result_counts));
    size_t *results = malloc(entries * sizeof(*results)); double *times = malloc(queries * sizeof(*times));
    if (records == NULL || result_counts == NULL || results == NULL || times == NULL) return 1;
    state = seed;
    for (size_t i = 0; i < entries; ++i) {
        double x = (double)(random_u32() % 1000000U) / 10.0, y = (double)(random_u32() % 1000000U) / 10.0;
        records[i] = (spatial_record){(uint64_t)i + 1, {x, y, 8.0 + (double)(random_u32() % 400U) / 10.0, 8.0 + (double)(random_u32() % 400U) / 10.0}};
    }
    spatial_index *index = NULL;
    uint64_t start_ticks = mach_absolute_time(); spatial_status status = spatial_index_create(records, entries, &index);
    double build_seconds = tick_seconds(mach_absolute_time() - start_ticks, timebase);
    if (status != SPATIAL_STATUS_OK) { fprintf(stderr, "index build failed: %d\n", status); return 1; }
    for (size_t q = 0; q < queries; ++q) {
        spatial_bounds viewport = {(double)(q % 100U) * 1000.0, (double)((q * 37U) % 100U) * 1000.0, 1000.0, 1000.0};
        size_t count = 0; start_ticks = mach_absolute_time();
        status = spatial_index_query(index, viewport, results, entries, &count);
        times[q] = tick_seconds(mach_absolute_time() - start_ticks, timebase);
        if (status != SPATIAL_STATUS_OK) { fprintf(stderr, "query failed: %d\n", status); return 1; }
        result_counts[q] = count;
    }
    raster_tile_cache *cache = NULL; raster_tile_cache_stats cache_stats = {0}; bool cap_rejected = false;
    if (!raster_tile_cache_create(4096, 32, 1, 1, &cache)) return 1;
    raster_tile too_large = {32, 32, 4096, malloc(4096)};
    if (too_large.rgba == NULL) return 1;
    cap_rejected = !raster_tile_cache_put(cache, UINT64_MAX, &too_large) && too_large.rgba != NULL;
    free(too_large.rgba); raster_tile_cache_get_stats(cache, &cache_stats);
    struct stat binary_stat; size_t binary_bytes = stat(argv[0], &binary_stat) == 0 ? (size_t)binary_stat.st_size : 0;
    bool written = write_result(output, records, entries, result_counts, queries, times, build_seconds, seed, binary_bytes, cache_stats.accounted_bytes, cap_rejected);
    qsort(times, queries, sizeof(*times), compare_double);
    printf("entries=%zu queries=%zu build_seconds=%.6f p50_ms=%.3f p95_ms=%.3f max_ms=%.3f\n", entries, queries, build_seconds, times[(queries + 1) / 2 - 1] * 1000.0, times[(queries * 95 + 99) / 100 - 1] * 1000.0, times[queries - 1] * 1000.0);
    raster_tile_cache_destroy(cache); spatial_index_destroy(index); free(records); free(result_counts); free(results); free(times);
    return written ? 0 : 1;
}
