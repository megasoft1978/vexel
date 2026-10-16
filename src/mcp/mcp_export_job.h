#ifndef VEXEL_MCP_MCP_EXPORT_JOB_H
#define VEXEL_MCP_MCP_EXPORT_JOB_H

#include "mcp_export_snapshot.h"
#include "platform/platform_task.h"
#include "platform/media/platform_animation.h"

#include <stdbool.h>
#include <stdint.h>

typedef struct mcp_export_job mcp_export_job;

typedef enum mcp_export_job_state {
    MCP_EXPORT_JOB_READY = 0,
    MCP_EXPORT_JOB_RUNNING,
    MCP_EXPORT_JOB_ENCODED,
    MCP_EXPORT_JOB_CANCELLED,
    MCP_EXPORT_JOB_FAILED
} mcp_export_job_state;

typedef enum mcp_export_job_error {
    MCP_EXPORT_JOB_ERROR_NONE = 0,
    MCP_EXPORT_JOB_ERROR_INVALID_ARGUMENT,
    MCP_EXPORT_JOB_ERROR_DECODE,
    MCP_EXPORT_JOB_ERROR_TIMELINE,
    MCP_EXPORT_JOB_ERROR_DURATION,
    MCP_EXPORT_JOB_ERROR_LIMIT,
    MCP_EXPORT_JOB_ERROR_RENDER,
    MCP_EXPORT_JOB_ERROR_ENCODE,
    MCP_EXPORT_JOB_ERROR_TIMEOUT,
    MCP_EXPORT_JOB_ERROR_CANCELLED,
    MCP_EXPORT_JOB_ERROR_OUT_OF_MEMORY
} mcp_export_job_error;

typedef struct mcp_export_job_status {
    mcp_export_job_state state;
    mcp_export_job_error error;
    uint16_t progress_permille;
    uint32_t width;
    uint32_t height;
    uint32_t frame_count;
} mcp_export_job_status;

/*
 * fps must be an integral value from 1 through 30 because the native GIF
 * encoder and current MCP export tool use uint32_t FPS. On success this
 * consumes snapshot; on failure snapshot stays caller-owned.
 */
bool mcp_export_job_create(mcp_export_snapshot *snapshot, double fps,
                           double duration_seconds, mcp_export_job **out_job);
/*
 * platform_task_callback; invoke once. The callback borrows the queue token
 * only during execution and does not retain it. An owner using a task queue
 * must gate result use on a successful, non-cancelled queue result, then make
 * its final job cancellation check before take_encoded/publication.
 */
bool mcp_export_job_execute(void *job,
    const platform_task_cancel_token *token, int64_t *out_value);
void mcp_export_job_request_cancel(mcp_export_job *job);
/* Thread-safe snapshot of state and bounded status fields. */
bool mcp_export_job_get_status(mcp_export_job *job,
                               mcp_export_job_status *out_status);
/* Moves bytes only in ENCODED state and if cancellation was not requested. */
bool mcp_export_job_take_encoded(mcp_export_job *job,
                                 platform_animation_bytes *out_bytes);
/* Frees untaken bytes and owned snapshot. Caller must join before destroy. */
void mcp_export_job_destroy(mcp_export_job *job);

#ifdef MCP_EXPORT_JOB_TESTING
/* Test executable only: pause after a rendered zero-based frame index. */
void mcp_export_job_test_pause_after_frame(uint32_t frame_index);
bool mcp_export_job_test_wait_until_paused(void);
void mcp_export_job_test_release_frame(void);
#endif

#endif
