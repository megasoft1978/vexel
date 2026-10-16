#include "mcp_export_job.h"
#include "project/project_animation_render.h"
#include "project/project_timeline_integration.h"
#include "project/project_timeline_session.h"
#include "platform/platform_task.h"

#include <CoreGraphics/CoreGraphics.h>
#include <CoreFoundation/CoreFoundation.h>
#include <ImageIO/ImageIO.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static unsigned failures;
#define CHECK(value) do { if (!(value)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #value); \
    failures++; return false; } } while (0)

static base_bytes bytes(const char *text)
{
    return (base_bytes){(const uint8_t *)text, strlen(text)};
}

static bool make_snapshot_with(const char *scene, const char *timeline,
                               mcp_export_snapshot **out)
{
    static const char red[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><rect width='2' height='2' fill='#ff0000'/></svg>";
    static const char green[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><rect width='2' height='2' fill='#00ff00'/></svg>";
    static const char manifest[] = "{}";
    project_bundle_asset assets[] = {
        {bytes("red"), bytes("image/svg+xml"), bytes(red)},
        {bytes("green"), bytes("image/svg+xml"), bytes(green)}
    };
    project_bundle_input input = {bytes(manifest), bytes(scene), bytes(timeline),
                                  assets, 2U};
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) return false;
    uint8_t *encoded = malloc(measured.encoded_size);
    if (encoded == NULL) return false;
    project_bundle_result written = project_bundle_encode(&input,
        (base_mut_bytes){encoded, measured.encoded_size});
    project_bundle_view view = {0};
    if (written.error != PROJECT_BUNDLE_OK || project_bundle_decode(
        (base_bytes){encoded, written.encoded_size}, &view).error !=
        PROJECT_BUNDLE_OK) { free(encoded); return false; }
    project_bundle_result snapshot_result = mcp_export_snapshot_create(
        &view, view.scene_svg, view.timeline_json, out);
    memset(encoded, 0, written.encoded_size);
    free(encoded);
    return snapshot_result.error == PROJECT_BUNDLE_OK;
}

static bool make_snapshot(mcp_export_snapshot **out)
{
    static const char timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":4,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"red\"},"
        "{\"tick\":1,\"asset\":\"green\"},"
        "{\"tick\":2,\"asset\":\"red\"},"
        "{\"tick\":3,\"asset\":\"green\"}]}]}";
    static const char scene[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><g id='layer'/></svg>";
    return make_snapshot_with(scene, timeline, out);
}

static void destroy_expected_frames(raster_tile frames[4])
{
    for (size_t index = 0U; index < 4U; ++index)
        raster_tile_destroy(&frames[index]);
}

static bool sample_native_frames(const mcp_export_snapshot *snapshot,
                                 raster_tile out_frames[4])
{
    const project_bundle_view *bundle = mcp_export_snapshot_bundle(snapshot);
    if (bundle == NULL) return false;
    svg_shape_document scene = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (svg_shape_decode(bundle->scene_svg, &scene, &diagnostic) !=
        SVG_SHAPE_ERROR_NONE) return false;
    project_timeline_view timeline = {0};
    project_timeline_context_owner owner = {0};
    project_timeline_integration_result checked =
        project_timeline_validate_bundle_owned(bundle, bundle->scene_svg,
            NULL, NULL, &timeline, &owner);
    if (checked.validation.error != PROJECT_TIMELINE_OK) {
        svg_shape_document_destroy(&scene);
        project_timeline_context_owner_destroy(&owner);
        return false;
    }
    project_timeline_session *session = NULL;
    project_timeline_edit_result created = project_timeline_session_create(
        bundle->timeline_json, &owner, project_timeline_edit_default_limits(),
        &session);
    if (created.error != PROJECT_TIMELINE_OK || session == NULL) {
        svg_shape_document_destroy(&scene);
        project_timeline_context_owner_destroy(&owner);
        return false;
    }
    bool okay = true;
    for (uint32_t index = 0U; index < 4U && okay; ++index) {
        uint32_t tick = index * timeline.ticks_per_second / 24U;
        okay = project_timeline_session_render_frame(session, &scene, bundle,
            tick, &out_frames[index]).error == PROJECT_ANIMATION_RENDER_OK;
    }
    project_timeline_session_destroy(session);
    project_timeline_context_owner_destroy(&owner);
    svg_shape_document_destroy(&scene);
    if (!okay) destroy_expected_frames(out_frames);
    return okay;
}

static bool compare_decoded_gif(const platform_animation_bytes *gif,
                                const raster_tile expected[4])
{
    if (gif == NULL || gif->bytes == NULL || gif->length == 0U) return false;
    CFDataRef data = CFDataCreate(kCFAllocatorDefault, gif->bytes,
                                  (CFIndex)gif->length);
    if (data == NULL) return false;
    CGImageSourceRef source = CGImageSourceCreateWithData(data, NULL);
    CFRelease(data);
    size_t image_count = source == NULL ? 0U : CGImageSourceGetCount(source);
    if (source == NULL || image_count != 4U) {
        if (source != NULL) CFRelease(source);
        return false;
    }
    bool okay = true;
    for (size_t index = 0U; index < image_count && okay; ++index) {
        CGImageRef image = CGImageSourceCreateImageAtIndex(source, index, NULL);
        CFDictionaryRef properties = CGImageSourceCopyPropertiesAtIndex(
            source, index, NULL);
        if (image == NULL || properties == NULL ||
            CGImageGetWidth(image) != expected[index].width ||
            CGImageGetHeight(image) != expected[index].height) {
            okay = false;
        }
        CFDictionaryRef gif_properties = properties == NULL ? NULL :
            CFDictionaryGetValue(properties, kCGImagePropertyGIFDictionary);
        CFNumberRef delay = gif_properties == NULL ? NULL :
            CFDictionaryGetValue(gif_properties,
                                 kCGImagePropertyGIFUnclampedDelayTime);
        double seconds = 0.0;
        if (delay == NULL || !CFNumberGetValue(delay, kCFNumberDoubleType,
                                               &seconds) ||
            seconds < 1.0 / 24.0 - 0.01 ||
            seconds > 1.0 / 24.0 + 0.01) {
            (void)fprintf(stderr, "bad delay frame=%zu delay=%p seconds=%f\n",
                          index, (void *)delay, seconds);
            okay = false;
        }
        const raster_tile *sample = &expected[index];
        size_t byte_count = sample->byte_count;
        uint8_t *pixels = okay ? calloc(byte_count, 1U) : NULL;
        CGColorSpaceRef color_space = pixels == NULL ? NULL :
            CGColorSpaceCreateDeviceRGB();
        CGContextRef context = color_space == NULL ? NULL :
            CGBitmapContextCreate(pixels, sample->width,
                sample->height, 8U, (size_t)sample->width * 4U,
                color_space, (CGBitmapInfo)kCGBitmapByteOrder32Big |
                             (CGBitmapInfo)kCGImageAlphaPremultipliedLast);
        if (context == NULL) okay = false;
        if (okay) {
            CGContextDrawImage(context, CGRectMake(0.0, 0.0,
                sample->width, sample->height), image);
            okay = memcmp(pixels, sample->rgba, byte_count) == 0;
            if (!okay) (void)fprintf(stderr,
                "bad pixels frame=%zu decoded=%u,%u,%u,%u native=%u,%u,%u,%u\n",
                index, pixels[0], pixels[1], pixels[2], pixels[3],
                sample->rgba[0], sample->rgba[1], sample->rgba[2], sample->rgba[3]);
        }
        if (context != NULL) CGContextRelease(context);
        if (color_space != NULL) CGColorSpaceRelease(color_space);
        free(pixels);
        if (image != NULL) CGImageRelease(image);
        if (properties != NULL) CFRelease(properties);
    }
    CFRelease(source);
    return okay;
}

static bool test_snapshot_job_and_queue(void)
{
    mcp_export_snapshot *snapshot = NULL;
    CHECK(make_snapshot(&snapshot));
    raster_tile expected_frames[4] = {{0}};
    CHECK(sample_native_frames(snapshot, expected_frames));
    mcp_export_job *invalid = NULL;
    CHECK(!mcp_export_job_create(snapshot, 0.0, 0.1, &invalid));
    CHECK(snapshot != NULL && invalid == NULL);
    CHECK(!mcp_export_job_create(snapshot, 31.0, 0.1, &invalid));
    CHECK(!mcp_export_job_create(snapshot, 24.0, 6.0, &invalid));
    mcp_export_job *job = NULL;
    CHECK(mcp_export_job_create(snapshot, 24.0, 4.0 / 24.0, &job));
    snapshot = NULL;
    mcp_export_job_status status = {0};
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_READY && status.progress_permille == 0U);
    platform_task_queue *queue = NULL;
    platform_task_config task_config = {.capacity = 4U, .worker_count = 1U};
    CHECK(platform_task_queue_create(&task_config, &queue) == PLATFORM_TASK_OK);
    CHECK(platform_task_submit(queue, 41U, 1U, mcp_export_job_execute,
                              job) == PLATFORM_TASK_OK);
    platform_task_result result = {0};
    bool found = false;
    for (unsigned attempt = 0U; attempt < 10000U && !found; ++attempt) {
        found = platform_task_poll(queue, &result);
        if (!found) {
            struct timespec pause = {0, 1000000L};
            (void)nanosleep(&pause, NULL);
            CHECK(mcp_export_job_get_status(job, &status));
            CHECK(status.progress_permille <= 1000U);
        }
    }
    CHECK(found && result.task_id == 41U && result.succeeded);
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_ENCODED && status.frame_count == 4U &&
          status.width == 2U && status.height == 2U &&
          status.progress_permille == 1000U);
    CHECK(!mcp_export_job_execute(job, NULL, NULL));
    platform_animation_bytes gif = {0};
    CHECK(mcp_export_job_take_encoded(job, &gif));
    CHECK(gif.length > 20U && memcmp(gif.bytes, "GIF", 3U) == 0);
    CHECK(compare_decoded_gif(&gif, expected_frames));
    destroy_expected_frames(expected_frames);
    platform_animation_bytes_destroy(&gif);
    CHECK(!mcp_export_job_take_encoded(job, &gif));
    platform_task_shutdown(queue);
    platform_task_destroy(queue);
    mcp_export_job_destroy(job);
    for (unsigned repeat = 0U; repeat < 2U; ++repeat) {
        CHECK(make_snapshot(&snapshot));
        CHECK(mcp_export_job_create(snapshot, 24.0, 4.0 / 24.0, &job));
        CHECK(mcp_export_job_execute(job, NULL, NULL));
        mcp_export_job_destroy(job);
    }
    return true;
}

static bool test_cancel_before_run_and_cancel_after_encode(void)
{
    mcp_export_snapshot *snapshot = NULL;
    CHECK(make_snapshot(&snapshot));
    mcp_export_job *job = NULL;
    CHECK(mcp_export_job_create(snapshot, 24.0, 4.0 / 24.0, &job));
    mcp_export_job_request_cancel(job);
    int64_t value = 0;
    CHECK(!mcp_export_job_execute(job, NULL, &value));
    mcp_export_job_status status = {0};
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_CANCELLED);
    platform_animation_bytes gif = {0};
    CHECK(!mcp_export_job_take_encoded(job, &gif));
    mcp_export_job_destroy(job);

    CHECK(make_snapshot(&snapshot));
    CHECK(mcp_export_job_create(snapshot, 24.0, 4.0 / 24.0, &job));
    CHECK(mcp_export_job_execute(job, NULL, &value));
    mcp_export_job_request_cancel(job);
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_CANCELLED &&
          status.error == MCP_EXPORT_JOB_ERROR_CANCELLED);
    CHECK(!mcp_export_job_take_encoded(job, &gif));
    mcp_export_job_destroy(job);
    return true;
}

static bool test_mid_render_queue_cancellation(void)
{
    mcp_export_snapshot *snapshot = NULL;
    CHECK(make_snapshot(&snapshot));
    mcp_export_job *job = NULL;
    CHECK(mcp_export_job_create(snapshot, 24.0, 4.0 / 24.0, &job));
    platform_task_queue *queue = NULL;
    platform_task_config task_config = {.capacity = 4U, .worker_count = 1U};
    CHECK(platform_task_queue_create(&task_config, &queue) == PLATFORM_TASK_OK);
    mcp_export_job_test_pause_after_frame(0U);
    CHECK(platform_task_submit(queue, 42U, 1U, mcp_export_job_execute,
                              job) == PLATFORM_TASK_OK);
    CHECK(mcp_export_job_test_wait_until_paused());
    mcp_export_job_status status = {0};
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_RUNNING &&
          status.progress_permille > 0U && status.progress_permille < 1000U);
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(platform_task_cancel(queue, 42U) == PLATFORM_TASK_OK);
    mcp_export_job_test_release_frame();
    bool canceled = false;
    for (unsigned attempt = 0U; attempt < 10000U && !canceled; ++attempt) {
        CHECK(mcp_export_job_get_status(job, &status));
        canceled = status.state == MCP_EXPORT_JOB_CANCELLED;
        if (!canceled) {
            struct timespec pause = {0, 1000000L};
            (void)nanosleep(&pause, NULL);
        }
    }
    CHECK(canceled);
    platform_task_shutdown(queue);
    platform_task_result result = {0};
    CHECK(!platform_task_poll(queue, &result));
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_CANCELLED &&
          status.error == MCP_EXPORT_JOB_ERROR_CANCELLED);
    platform_animation_bytes gif = {0};
    CHECK(!mcp_export_job_take_encoded(job, &gif));
    CHECK(!mcp_export_job_execute(job, NULL, NULL));
    platform_task_destroy(queue);
    mcp_export_job_destroy(job);
    return true;
}

static bool test_native_duration_decode_and_work_limits(void)
{
    mcp_export_snapshot *snapshot = NULL;
    mcp_export_job *job = NULL;
    mcp_export_job_status status = {0};
    int64_t value = 0;
    CHECK(make_snapshot(&snapshot));
    CHECK(mcp_export_job_create(snapshot, 24.0, 5.0 / 24.0, &job));
    CHECK(!mcp_export_job_execute(job, NULL, &value));
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_FAILED &&
          status.error == MCP_EXPORT_JOB_ERROR_DURATION);
    mcp_export_job_destroy(job);

    static const char broken_scene[] = "<svg width='2' height='2'><g>";
    static const char short_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":4,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"red\"}]}]}";
    CHECK(make_snapshot_with(broken_scene, short_timeline, &snapshot));
    CHECK(mcp_export_job_create(snapshot, 24.0, 1.0 / 24.0, &job));
    CHECK(!mcp_export_job_execute(job, NULL, &value));
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_FAILED &&
          status.error == MCP_EXPORT_JOB_ERROR_DECODE);
    mcp_export_job_destroy(job);

    static const char large_scene[] =
        "<svg width='512' height='512' viewBox='0 0 512 512'><g id='layer'/></svg>";
    static const char long_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":120,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"red\"},"
        "{\"tick\":60,\"asset\":\"green\"}]}]}";
    CHECK(make_snapshot_with(large_scene, long_timeline, &snapshot));
    CHECK(mcp_export_job_create(snapshot, 24.0, 5.0, &job));
    CHECK(!mcp_export_job_execute(job, NULL, &value));
    CHECK(mcp_export_job_get_status(job, &status));
    CHECK(status.state == MCP_EXPORT_JOB_FAILED &&
          status.error == MCP_EXPORT_JOB_ERROR_LIMIT);
    mcp_export_job_destroy(job);
    return true;
}

int main(void)
{
    if (!test_snapshot_job_and_queue() ||
        !test_cancel_before_run_and_cancel_after_encode() ||
        !test_mid_render_queue_cancellation() ||
        !test_native_duration_decode_and_work_limits()) return EXIT_FAILURE;
    if (failures != 0U) return EXIT_FAILURE;
    (void)puts("mcp_export_job_test: ok");
    return EXIT_SUCCESS;
}
