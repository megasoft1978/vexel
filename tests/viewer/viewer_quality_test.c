#include "app/frame_compose.h"
#include "app/viewer_quality.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>

static uint32_t random_next(uint32_t *state)
{
    uint32_t value = *state;
    value ^= value << 13U;
    value ^= value >> 17U;
    value ^= value << 5U;
    *state = value;
    return value;
}

static bool check_tile_edges_and_cache(void)
{
    ui_viewer_navigation nav;
    viewer_tile_plan plan;
    viewer_tile_region region;
    viewer_tile_cache cache;
    if (!ui_viewer_navigation_init(&nav, 513.0, 257.0, 513.0, 257.0) ||
        !viewer_tile_cache_init(&cache, 1U) ||
        !viewer_tile_plan_init(&plan, 513U, 257U, 1U, &nav)) return false;
    const uint32_t expected_widths[] = {256U, 256U, 1U, 256U, 256U, 1U, 1U, 1U, 1U};
    const uint32_t expected_heights[] = {256U, 256U, 256U, 1U, 1U, 1U};
    size_t found = 0U;
    while (viewer_tile_plan_next(&plan, &cache, 1U, &region)) {
        if (found >= 6U || region.width != expected_widths[found] ||
            region.height != expected_heights[found]) return false;
        size_t bytes = (size_t)region.width * region.height * 4U;
        uint8_t *pixels = calloc(bytes, 1U);
        viewer_sharp_tile tile = {region, bytes, pixels};
        if (pixels == NULL || !viewer_tile_cache_put(&cache, &tile) || tile.rgba != NULL ||
            viewer_tile_cache_get(&cache, 1U, region.grid_x, region.grid_y) == NULL) {
            free(pixels);
            viewer_tile_cache_destroy(&cache);
            return false;
        }
        found++;
    }
    if (found != 6U || cache.count != 6U ||
        cache.accounted_bytes != (size_t)513U * 257U * 4U) {
        viewer_tile_cache_destroy(&cache);
        return false;
    }
    viewer_tile_cache_clear(&cache, 2U);
    bool cleared = cache.count == 0U && cache.accounted_bytes == 0U &&
        viewer_tile_cache_get(&cache, 1U, 0U, 0U) == NULL;
    viewer_tile_cache_destroy(&cache);
    return cleared;
}

static bool check_clipping_extreme_scale_and_budget(void)
{
    ui_viewer_navigation nav = {
        .viewport_width = 512.0, .viewport_height = 256.0,
        .document_width = 4096.0, .document_height = 2048.0,
        .scale = 1000.0, .pan_x = -4095488.0, .pan_y = -1792.0
    };
    viewer_tile_plan plan;
    viewer_tile_region region;
    if (!viewer_tile_plan_init(&plan, 512U, 256U, 9U, &nav)) return false;
    size_t regions = 0U;
    while (viewer_tile_plan_next(&plan, NULL, 1U, &region)) {
        if (region.x >= 512U || region.y >= 256U || region.width == 0U ||
            region.height == 0U || region.width > VIEWER_SHARP_TILE_AXIS ||
            region.height > VIEWER_SHARP_TILE_AXIS) return false;
        regions++;
    }
    if (regions == 0U || regions > 4U) return false;

    nav.scale = 0.001;
    nav.pan_x = 0.0;
    nav.pan_y = 0.0;
    nav.document_width = 100000.0;
    nav.document_height = 100000.0;
    if (!viewer_tile_plan_init(&plan, 512U, 256U, 10U, &nav) ||
        !viewer_tile_plan_next(&plan, NULL, 1U, &region)) return false;

    nav.document_width = 256.0;
    nav.document_height = 256.0;
    nav.scale = 1.0;
    if (!viewer_tile_plan_init(&plan, 256U, 256U, 11U, &nav) ||
        viewer_tile_plan_next(&plan, NULL, 4000U, &region) || plan.planned_work != 0U) return false;

    nav.pan_x = -1.0e300;
    nav.scale = 1000.0;
    return viewer_tile_plan_init(&plan, 256U, 256U, 12U, &nav) &&
        !viewer_tile_plan_next(&plan, NULL, 1U, &region);
}

static bool check_cache_bounds_and_lru(void)
{
    viewer_tile_cache cache;
    if (!viewer_tile_cache_init(&cache, 3U)) return false;
    for (uint32_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES + 1U; index++) {
        uint8_t *pixels = calloc(4U, 1U);
        viewer_sharp_tile tile = {{3U, index, 0U, index, 0U, 1U, 1U}, 4U, pixels};
        if (pixels == NULL || !viewer_tile_cache_put(&cache, &tile)) {
            free(pixels);
            viewer_tile_cache_destroy(&cache);
            return false;
        }
    }
    bool bounded = cache.count == VIEWER_SHARP_CACHE_ENTRIES &&
        cache.accounted_bytes == VIEWER_SHARP_CACHE_ENTRIES * 4U &&
        viewer_tile_cache_get(&cache, 3U, 0U, 0U) == NULL &&
        viewer_tile_cache_get(&cache, 3U, VIEWER_SHARP_CACHE_ENTRIES, 0U) != NULL;
    viewer_tile_cache_destroy(&cache);
    return bounded;
}

static bool navigation_invariants(const ui_viewer_navigation *nav)
{
    if (!isfinite(nav->scale) || nav->scale < UI_VIEWER_MIN_SCALE ||
        nav->scale > UI_VIEWER_MAX_SCALE || !isfinite(nav->pan_x) ||
        !isfinite(nav->pan_y) || !isfinite(nav->viewport_width) ||
        !isfinite(nav->viewport_height) || nav->viewport_width <= 0.0 ||
        nav->viewport_height <= 0.0) return false;
    double x = 0.0, y = 0.0;
    return ui_viewer_navigation_project(nav, 0.0, 0.0, &x, &y) &&
        isfinite(x) && isfinite(y);
}

typedef enum test_phase { TEST_EMPTY, TEST_LOADING, TEST_READY, TEST_ERROR,
                          TEST_CLOSING, TEST_CLOSED } test_phase;

typedef struct test_viewer_state {
    uint64_t generation;
    uint64_t job_generation;
    test_phase phase;
    bool pending_open;
    bool job_active;
    bool job_is_load;
} test_viewer_state;

static bool test_state_invariants(const test_viewer_state *state)
{
    if (state->generation == 0U ||
        (state->pending_open && state->phase != TEST_LOADING) ||
        (state->phase == TEST_LOADING && !state->pending_open &&
         !(state->job_active && state->job_is_load &&
           state->job_generation == state->generation)) ||
        (state->phase == TEST_READY && state->pending_open) ||
        ((state->phase == TEST_CLOSING || state->phase == TEST_CLOSED) &&
         state->pending_open) ||
        (state->phase == TEST_CLOSED && state->job_active)) return false;
    if (state->job_active && (state->job_generation == 0U ||
        state->job_generation > state->generation)) return false;
    if (state->job_active && state->job_generation == state->generation &&
        ((state->job_is_load && state->phase != TEST_LOADING) ||
         (!state->job_is_load && state->phase != TEST_READY))) return false;
    return true;
}

static bool check_randomized_viewer_state_machine(void)
{
    uint32_t random = UINT32_C(0x574f504e);
    test_viewer_state state = {.generation = 1U, .phase = TEST_EMPTY};
    uint64_t stale_completions = 0U;
    uint64_t seeded_generation = 0U;
    if (!viewer_generation_advance(state.generation, &seeded_generation)) return false;
    state.generation = seeded_generation;
    state.phase = TEST_LOADING;
    state.job_active = true;
    state.job_is_load = true;
    state.job_generation = state.generation;
    if (!viewer_generation_advance(state.generation, &seeded_generation)) return false;
    state.generation = seeded_generation;
    state.pending_open = true;
    if (viewer_generation_is_current(state.generation, state.job_generation)) return false;
    state.job_active = false;
    stale_completions++;
    if (!test_state_invariants(&state)) return false;
    for (uint32_t step = 0U; step < 200000U; step++) {
        uint32_t event = random_next(&random) % 8U;
        if (event == 0U && state.phase != TEST_CLOSING && state.phase != TEST_CLOSED) {
            uint64_t next = 0U;
            if (viewer_generation_advance(state.generation, &next)) {
                state.generation = next;
                state.phase = TEST_LOADING;
                state.pending_open = true;
            }
        } else if (event == 1U && !state.job_active &&
                   state.phase != TEST_CLOSING && state.phase != TEST_CLOSED &&
                   (state.pending_open || state.phase == TEST_READY)) {
            state.job_active = true;
            state.job_is_load = state.pending_open;
            state.job_generation = state.generation;
            if (state.job_is_load) state.pending_open = false;
        } else if (event == 2U && state.job_active) {
            bool current = viewer_generation_is_current(state.generation,
                                                         state.job_generation);
            test_phase before = state.phase;
            if (!current) stale_completions++;
            if (current && state.job_is_load && state.phase == TEST_LOADING)
                state.phase = (random_next(&random) & 1U) != 0U ? TEST_READY : TEST_ERROR;
            else if (!current && state.phase == TEST_CLOSING)
                state.phase = TEST_CLOSED;
            if (!current && before != TEST_CLOSING && state.phase != before) return false;
            state.job_active = false;
        } else if ((event == 3U || event == 4U) && state.phase == TEST_READY) {
            uint64_t next = 0U;
            if (viewer_generation_advance(state.generation, &next)) state.generation = next;
        } else if (event == 5U && state.phase != TEST_CLOSED &&
                   state.phase != TEST_CLOSING) {
            uint64_t next = 0U;
            if (viewer_generation_advance(state.generation, &next)) {
                state.generation = next;
                state.phase = TEST_CLOSING;
                state.pending_open = false;
                if (!state.job_active) state.phase = TEST_CLOSED;
            }
        } else if (event == 6U && state.phase == TEST_ERROR) {
            uint64_t next = 0U;
            if (viewer_generation_advance(state.generation, &next)) {
                state.generation = next;
                state.phase = TEST_LOADING;
                state.pending_open = true;
            }
        }
        if (!test_state_invariants(&state)) {
            (void)fprintf(stderr, "viewer state invariant at step %u (phase=%d, gen=%llu, job=%d/%llu, pending=%d)\n",
                step, (int)state.phase, (unsigned long long)state.generation,
                state.job_active ? 1 : 0, (unsigned long long)state.job_generation,
                state.pending_open ? 1 : 0);
            return false;
        }
    }
    if (stale_completions == 0U)
        (void)fputs("viewer state sequence did not produce a stale completion\n", stderr);
    return stale_completions > 0U && test_state_invariants(&state);
}

static bool check_generation_edges(void)
{
    uint64_t next = 0U;
    return !viewer_generation_advance(0U, &next) &&
        !viewer_generation_advance(UINT64_MAX, &next) &&
        viewer_generation_advance(1U, &next) && next == 2U &&
        viewer_generation_is_current(2U, 2U) &&
        !viewer_generation_is_current(3U, 2U);
}

static bool check_randomized_event_sequence(void)
{
    uint32_t random = UINT32_C(0x57494557);
    ui_viewer_navigation nav;
    viewer_tile_cache cache;
    uint64_t generation = 1U;
    if (!ui_viewer_navigation_init(&nav, 900.0, 700.0, 2048.0, 1536.0) ||
        !ui_viewer_navigation_fit(&nav) || !viewer_tile_cache_init(&cache, generation)) return false;
    for (uint32_t event = 0U; event < 200000U; event++) {
        uint32_t operation = random_next(&random) % 10U;
        uint32_t a = random_next(&random);
        ui_viewer_navigation before = nav;
        if (operation == 0U) {
            (void)ui_viewer_navigation_zoom(&nav, (a & 1U) != 0U ? 1.17 : 0.83,
                (double)(a % 900U), (double)(a % 700U));
        } else if (operation == 1U) {
            nav.pan_x += (double)((int32_t)(a % 200001U) - 100000) / 10.0;
            nav.pan_y += (double)((int32_t)(random_next(&random) % 200001U) - 100000) / 10.0;
        } else if (operation == 2U) {
            (void)ui_viewer_navigation_fit(&nav);
        } else if (operation == 3U) {
            (void)ui_viewer_navigation_fit_width(&nav);
        } else if (operation == 4U) {
            (void)ui_viewer_navigation_actual_size(&nav);
        } else if (operation == 5U) {
            double width = (double)(random_next(&random) % 8192U + 1U);
            double height = (double)(random_next(&random) % 8192U + 1U);
            (void)ui_viewer_navigation_resize(&nav, width, height);
        } else if (operation == 6U) {
            (void)ui_viewer_navigation_drag_begin(&nav, (double)(a % 8192U),
                (double)(random_next(&random) % 8192U));
        } else if (operation == 7U) {
            (void)ui_viewer_navigation_drag(&nav, (double)(a % 8192U),
                (double)(random_next(&random) % 8192U));
        } else if (operation == 8U) {
            (void)ui_viewer_navigation_drag_end(&nav);
        } else {
            generation++;
            viewer_tile_cache_clear(&cache, generation);
        }
        if (!navigation_invariants(&nav)) {
            viewer_tile_cache_destroy(&cache);
            return false;
        }
        if (nav.scale != before.scale || nav.pan_x != before.pan_x ||
            nav.pan_y != before.pan_y || nav.viewport_width != before.viewport_width ||
            nav.viewport_height != before.viewport_height) {
            if (generation == UINT64_MAX) {
                viewer_tile_cache_destroy(&cache);
                return false;
            }
            generation++;
            viewer_tile_cache_clear(&cache, generation);
        }
    }
    bool valid = cache.generation == generation && cache.count == 0U;
    viewer_tile_cache_destroy(&cache);
    return valid;
}

int main(void)
{
    if (!check_tile_edges_and_cache()) return 1;
    if (!check_clipping_extreme_scale_and_budget()) return 2;
    if (!check_cache_bounds_and_lru()) return 3;
    if (!check_randomized_event_sequence()) return 4;
    if (!check_generation_edges()) return 5;
    if (!check_randomized_viewer_state_machine()) return 6;
    (void)puts("viewer quality: tile, cache, cancel epochs, 200000 navigation and state events pass");
    return 0;
}
