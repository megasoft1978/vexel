#include "mcp_task_store.h"

#include <stdio.h>
#include <string.h>

static unsigned failures;
#define CHECK(value) do { if (!(value)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #value); \
    failures++; return false; } } while (0)

static bool test_lifecycle_and_ownership(void)
{
    mcp_task_store *store = NULL;
    CHECK(mcp_task_store_create(&store));
    char first[MCP_TASK_STORE_ID_CAP], second[MCP_TASK_STORE_ID_CAP], extra[33U];
    CHECK(mcp_task_store_start(store, 7U, 100U, first));
    CHECK(strlen(first) == 32U);
    CHECK(mcp_task_store_start(store, 7U, 101U, second));
    CHECK(strcmp(first, second) != 0);
    CHECK(!mcp_task_store_start(store, 7U, 102U, extra));
    mcp_task_snapshot snapshot;
    uint8_t result[32U];
    CHECK(!mcp_task_store_get(store, 8U, first, 103U, &snapshot,
        result, sizeof(result)));
    CHECK(mcp_task_store_update(store, 7U, first, 104U, 400U, "Rendering 1/2"));
    CHECK(!mcp_task_store_update(store, 7U, first, 105U, 300U, "Regressed"));
    CHECK(!mcp_task_store_update(store, 7U, first, 105U, 1001U, "Out of range"));
    CHECK(mcp_task_store_request_cancel(store, 7U, first, 106U));
    CHECK(mcp_task_store_cancel_requested(store, 7U, first));
    CHECK(mcp_task_store_finish(store, 7U, first, 107U, MCP_TASK_CANCELLED,
        "Export cancelled", NULL, 0U, false));
    CHECK(mcp_task_store_cancel_requested(store, 7U, first));
    CHECK(mcp_task_store_get(store, 7U, first, 108U, &snapshot,
        result, sizeof(result)));
    CHECK(snapshot.status == MCP_TASK_CANCELLED &&
        snapshot.progress_permille == 400U &&
        strcmp(snapshot.message, "Export cancelled") == 0);
    CHECK(!mcp_task_store_finish(store, 7U, first, 109U, MCP_TASK_FAILED,
        "duplicate finish", NULL, 0U, true));
    mcp_task_store_destroy(store);
    return true;
}

static bool test_result_expiry_and_retention(void)
{
    mcp_task_store *store = NULL;
    CHECK(mcp_task_store_create(&store));
    char ids[MCP_TASK_STORE_RETAINED_LIMIT + 1U][MCP_TASK_STORE_ID_CAP];
    const uint8_t json[] = "{\"saved\":true}";
    for (size_t index = 0U; index < MCP_TASK_STORE_RETAINED_LIMIT; ++index) {
        CHECK(mcp_task_store_start(store, 3U, 100U + index, ids[index]));
        CHECK(mcp_task_store_finish(store, 3U, ids[index], 200U + index,
            MCP_TASK_COMPLETED, "Export complete", json, sizeof(json) - 1U,
            false));
    }
    CHECK(mcp_task_store_start(store, 3U, 300U, ids[MCP_TASK_STORE_RETAINED_LIMIT]));
    CHECK(!mcp_task_store_get(store, 3U, ids[0], 301U,
        &(mcp_task_snapshot){0}, NULL, 0U));
    CHECK(mcp_task_store_finish(store, 3U,
        ids[MCP_TASK_STORE_RETAINED_LIMIT], 302U, MCP_TASK_COMPLETED,
        "Export complete", json, sizeof(json) - 1U, false));
    mcp_task_snapshot snapshot;
    uint8_t result[32U];
    CHECK(mcp_task_store_get(store, 3U, ids[1], 303U, &snapshot,
        result, sizeof(result)));
    CHECK(snapshot.status == MCP_TASK_COMPLETED && snapshot.result_length ==
        sizeof(json) - 1U && memcmp(result, json, snapshot.result_length) == 0);
    CHECK(!mcp_task_store_get(store, 3U, ids[1],
        303U + MCP_TASK_STORE_TTL_MS, &snapshot, result, sizeof(result)));
    CHECK(strcmp(mcp_task_status_name(MCP_TASK_WORKING), "working") == 0 &&
        strcmp(mcp_task_status_name(MCP_TASK_COMPLETED), "completed") == 0 &&
        strcmp(mcp_task_status_name(MCP_TASK_CANCELLED), "cancelled") == 0 &&
        strcmp(mcp_task_status_name(MCP_TASK_FAILED), "failed") == 0);
    mcp_task_store_destroy(store);
    return true;
}

int main(void)
{
    bool okay = test_lifecycle_and_ownership() && test_result_expiry_and_retention();
    if (!okay || failures != 0U) return 1;
    (void)puts("mcp async task store: lifecycle, ownership, progress, cancellation, retention passed");
    return 0;
}
