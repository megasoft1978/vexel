#include "index.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct packed_entry {
    spatial_bounds bounds;
    uint64_t id;
    uint64_t morton;
    uint32_t source_index;
} packed_entry;

typedef struct packed_node {
    spatial_bounds bounds;
    uint32_t left;
    uint32_t right;
    uint32_t entry;
    bool leaf;
} packed_node;

struct spatial_index {
    size_t count;
    size_t node_count;
    packed_entry *entries;
    packed_node *nodes;
    uint32_t root;
};

typedef struct build_context { packed_node *nodes; size_t next; } build_context;
typedef struct hit_list { uint32_t *items; size_t count; size_t capacity; bool oom; } hit_list;

static bool bounds_valid(spatial_bounds b) {
    return isfinite(b.x) && isfinite(b.y) && isfinite(b.width) && isfinite(b.height) &&
           b.width >= 0.0 && b.height >= 0.0 && isfinite(b.x + b.width) && isfinite(b.y + b.height);
}

static bool intersects(spatial_bounds a, spatial_bounds b) {
    return a.x <= b.x + b.width && a.x + a.width >= b.x &&
           a.y <= b.y + b.height && a.y + a.height >= b.y;
}

static spatial_bounds combine(spatial_bounds a, spatial_bounds b) {
    double x = fmin(a.x, b.x), y = fmin(a.y, b.y);
    double right = fmax(a.x + a.width, b.x + b.width);
    double bottom = fmax(a.y + a.height, b.y + b.height);
    return (spatial_bounds){x, y, right - x, bottom - y};
}

static uint64_t spread_bits(uint32_t x) {
    uint64_t v = x;
    v = (v | (v << 16)) & UINT64_C(0x0000ffff0000ffff);
    v = (v | (v << 8)) & UINT64_C(0x00ff00ff00ff00ff);
    v = (v | (v << 4)) & UINT64_C(0x0f0f0f0f0f0f0f0f);
    v = (v | (v << 2)) & UINT64_C(0x3333333333333333);
    v = (v | (v << 1)) & UINT64_C(0x5555555555555555);
    return v;
}

static double center(double origin, double extent) { return origin * 0.5 + (origin + extent) * 0.5; }

static uint32_t quantize(double value, double minimum, double maximum) {
    double half_min = minimum * 0.5, half_max = maximum * 0.5;
    double range = half_max - half_min;
    if (range <= 0.0) return 0;
    double normalized = (value * 0.5 - half_min) / range;
    if (normalized <= 0.0) return 0;
    if (normalized >= 1.0) return UINT32_MAX;
    return (uint32_t)(normalized * (double)UINT32_MAX);
}

static int compare_entries(const void *left, const void *right) {
    const packed_entry *a = left, *b = right;
    if (a->morton < b->morton) return -1;
    if (a->morton > b->morton) return 1;
    if (a->source_index < b->source_index) return -1;
    if (a->source_index > b->source_index) return 1;
    return 0;
}

static int compare_ids(const void *left, const void *right) {
    uint64_t a = *(const uint64_t *)left, b = *(const uint64_t *)right;
    return a < b ? -1 : a > b ? 1 : 0;
}

static uint32_t build_nodes(build_context *ctx, const packed_entry *entries, size_t first, size_t end) {
    uint32_t node_index = (uint32_t)ctx->next++;
    packed_node *node = &ctx->nodes[node_index];
    if (end - first == 1) {
        node->bounds = entries[first].bounds;
        node->entry = (uint32_t)first;
        node->leaf = true;
        return node_index;
    }
    size_t middle = first + (end - first) / 2;
    node->left = build_nodes(ctx, entries, first, middle);
    node->right = build_nodes(ctx, entries, middle, end);
    node->bounds = combine(ctx->nodes[node->left].bounds, ctx->nodes[node->right].bounds);
    node->leaf = false;
    return node_index;
}

spatial_status spatial_index_create(const spatial_record *records, size_t count, spatial_index **out_index) {
    if (out_index == NULL) return SPATIAL_STATUS_INVALID;
    *out_index = NULL;
    if (count > SPATIAL_INDEX_MAX_RECORDS) return SPATIAL_STATUS_LIMIT;
    if (records == NULL && count != 0) return SPATIAL_STATUS_INVALID;
    if (count == 0) {
        spatial_index *empty = calloc(1, sizeof(*empty));
        if (empty == NULL) return SPATIAL_STATUS_OOM;
        *out_index = empty;
        return SPATIAL_STATUS_OK;
    }
    size_t entry_bytes, node_count, node_bytes, all_bytes;
    node_count = count * 2 - 1;
    if (count > SIZE_MAX / sizeof(packed_entry) || node_count > SIZE_MAX / sizeof(packed_node)) return SPATIAL_STATUS_OVERFLOW;
    entry_bytes = count * sizeof(packed_entry);
    node_bytes = node_count * sizeof(packed_node);
    if (entry_bytes > SIZE_MAX - node_bytes || entry_bytes + node_bytes > SIZE_MAX - count * sizeof(uint64_t)) return SPATIAL_STATUS_OVERFLOW;
    all_bytes = entry_bytes + node_bytes + count * sizeof(uint64_t);
    if (all_bytes > SPATIAL_INDEX_MAX_BYTES) return SPATIAL_STATUS_LIMIT;
    uint64_t *ids = malloc(count * sizeof(*ids));
    spatial_index *index = calloc(1, sizeof(*index));
    if (ids == NULL || index == NULL) { free(ids); free(index); return SPATIAL_STATUS_OOM; }
    index->entries = malloc(entry_bytes);
    index->nodes = malloc(node_bytes);
    if (index->entries == NULL || index->nodes == NULL) { free(ids); spatial_index_destroy(index); return SPATIAL_STATUS_OOM; }
    double min_x = INFINITY, min_y = INFINITY, max_x = -INFINITY, max_y = -INFINITY;
    double min_left = INFINITY, min_top = INFINITY, max_right = -INFINITY, max_bottom = -INFINITY;
    for (size_t i = 0; i < count; ++i) {
        spatial_bounds b = records[i].bounds;
        if (!bounds_valid(b)) { free(ids); spatial_index_destroy(index); return SPATIAL_STATUS_INVALID; }
        ids[i] = records[i].id;
        min_left = fmin(min_left, b.x); min_top = fmin(min_top, b.y);
        max_right = fmax(max_right, b.x + b.width); max_bottom = fmax(max_bottom, b.y + b.height);
        min_x = fmin(min_x, center(b.x, b.width)); min_y = fmin(min_y, center(b.y, b.height));
        max_x = fmax(max_x, center(b.x, b.width)); max_y = fmax(max_y, center(b.y, b.height));
        index->entries[i] = (packed_entry){b, records[i].id, 0, (uint32_t)i};
    }
    if (!isfinite(max_right - min_left) || !isfinite(max_bottom - min_top)) {
        free(ids); spatial_index_destroy(index); return SPATIAL_STATUS_OVERFLOW;
    }
    qsort(ids, count, sizeof(*ids), compare_ids);
    for (size_t i = 1; i < count; ++i) if (ids[i] == ids[i - 1]) { free(ids); spatial_index_destroy(index); return SPATIAL_STATUS_INVALID; }
    free(ids);
    for (size_t i = 0; i < count; ++i) {
        uint32_t qx = quantize(center(index->entries[i].bounds.x, index->entries[i].bounds.width), min_x, max_x);
        uint32_t qy = quantize(center(index->entries[i].bounds.y, index->entries[i].bounds.height), min_y, max_y);
        index->entries[i].morton = spread_bits(qx) | (spread_bits(qy) << 1);
    }
    qsort(index->entries, count, sizeof(*index->entries), compare_entries);
    index->count = count;
    index->node_count = node_count;
    build_context context = {index->nodes, 0};
    index->root = build_nodes(&context, index->entries, 0, count);
    *out_index = index;
    return SPATIAL_STATUS_OK;
}

void spatial_index_destroy(spatial_index *index) {
    if (index == NULL) return;
    free(index->entries); free(index->nodes); free(index);
}

size_t spatial_index_count(const spatial_index *index) { return index == NULL ? 0 : index->count; }

static void collect_hits(const spatial_index *index, uint32_t node_index, spatial_bounds query, hit_list *hits) {
    if (hits->oom || !intersects(index->nodes[node_index].bounds, query)) return;
    const packed_node *node = &index->nodes[node_index];
    if (node->leaf) {
        if (hits->count == hits->capacity) {
            size_t next = hits->capacity == 0 ? 32 : hits->capacity * 2;
            if (next < hits->capacity || next > SIZE_MAX / sizeof(*hits->items)) { hits->oom = true; return; }
            uint32_t *items = realloc(hits->items, next * sizeof(*items));
            if (items == NULL) { hits->oom = true; return; }
            hits->items = items; hits->capacity = next;
        }
        hits->items[hits->count++] = index->entries[node->entry].source_index;
        return;
    }
    collect_hits(index, node->left, query, hits);
    collect_hits(index, node->right, query, hits);
}

static int compare_indices(const void *left, const void *right) {
    uint32_t a = *(const uint32_t *)left, b = *(const uint32_t *)right;
    return a < b ? -1 : a > b ? 1 : 0;
}

spatial_status spatial_index_query(const spatial_index *index, spatial_bounds viewport,
                                   size_t *out_record_indices, size_t capacity, size_t *out_count) {
    if (out_count == NULL) return SPATIAL_STATUS_INVALID;
    *out_count = 0;
    if (index == NULL || !bounds_valid(viewport) || (capacity != 0 && out_record_indices == NULL)) return SPATIAL_STATUS_INVALID;
    if (index->count == 0) return SPATIAL_STATUS_OK;
    hit_list hits = {0};
    collect_hits(index, index->root, viewport, &hits);
    if (hits.oom) { free(hits.items); return SPATIAL_STATUS_OOM; }
    if (hits.count > 1) qsort(hits.items, hits.count, sizeof(*hits.items), compare_indices);
    *out_count = hits.count;
    if (capacity < hits.count) { free(hits.items); return SPATIAL_STATUS_LIMIT; }
    for (size_t i = 0; i < hits.count; ++i) out_record_indices[i] = hits.items[i];
    free(hits.items);
    return SPATIAL_STATUS_OK;
}
