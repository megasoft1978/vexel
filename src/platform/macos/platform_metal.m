#import <Cocoa/Cocoa.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>

#include "platform/platform_metal.h"
#include "platform/macos/platform_metal_internal.h"

#include <stdio.h>
#include <string.h>
#include <dispatch/dispatch.h>
#include <CommonCrypto/CommonDigest.h>

static const uint64_t PLATFORM_METAL_TIMEOUT_SECONDS = 5U;

static id<MTLDevice> platform_present_device;
static id<MTLCommandQueue> platform_present_queue;
static id<MTLRenderPipelineState> platform_present_pipeline;
static id<MTLTexture> platform_present_texture;
static uint32_t platform_present_texture_width;
static uint32_t platform_present_texture_height;

static bool platform_metal_fail(char *diagnostic, size_t capacity, const char *message)
{
    if (diagnostic != NULL && capacity > 0U) {
        (void)snprintf(diagnostic, capacity, "%s", message);
    }
    return false;
}

static bool platform_metal_dimensions_valid(uint32_t width, uint32_t height,
                                            size_t byte_count)
{
    if (width == 0U || height == 0U || width > 4096U || height > 4096U) {
        return false;
    }
    size_t width_size = (size_t)width;
    size_t height_size = (size_t)height;
    if (width_size > SIZE_MAX / height_size) {
        return false;
    }
    size_t pixels = width_size * height_size;
    return pixels <= SIZE_MAX / 4U && pixels * 4U == byte_count;
}

static bool platform_metal_wait(id<MTLCommandBuffer> command, char *diagnostic,
                                size_t diagnostic_capacity)
{
    dispatch_semaphore_t completed = dispatch_semaphore_create(0);
    [command addCompletedHandler:^(__unused id<MTLCommandBuffer> buffer) {
        dispatch_semaphore_signal(completed);
    }];
    [command commit];
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW,
        (int64_t)(PLATFORM_METAL_TIMEOUT_SECONDS * NSEC_PER_SEC));
    if (dispatch_semaphore_wait(completed, deadline) != 0) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "Metal completion timed out after 5 seconds");
    }
    return command.status == MTLCommandBufferStatusCompleted ||
        platform_metal_fail(diagnostic, diagnostic_capacity, "Metal command buffer failed");
}

static bool platform_metal_verify_readback(id<MTLTexture> texture, const uint8_t *rgba,
                                           size_t byte_count, uint32_t width,
                                           uint32_t height, uint8_t out_sha256[32],
                                           char *diagnostic, size_t diagnostic_capacity)
{
    uint8_t *readback = malloc(byte_count);
    if (readback == NULL) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "offscreen Metal readback allocation failed");
    }
    [texture getBytes:readback bytesPerRow:(size_t)width * 4U
        fromRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0];
    bool matches = memcmp(readback, rgba, byte_count) == 0;
    if (matches) {
        (void)CC_SHA256(readback, (CC_LONG)byte_count, out_sha256);
    }
    free(readback);
    return matches || platform_metal_fail(diagnostic, diagnostic_capacity,
                                           "offscreen Metal readback differs from input");
}

bool platform_metal_copy_and_hash(const uint8_t *rgba, size_t byte_count,
                                  uint32_t width, uint32_t height,
                                  uint8_t out_sha256[32], char *diagnostic,
                                  size_t diagnostic_capacity)
{
    if (rgba == NULL || out_sha256 == NULL ||
        !platform_metal_dimensions_valid(width, height, byte_count)) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "invalid offscreen Metal arguments");
    }
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        id<MTLCommandQueue> queue = [device newCommandQueue];
        if (device == nil || queue == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "offscreen Metal device or queue unavailable");
        }
        MTLTextureDescriptor *descriptor = [MTLTextureDescriptor
            texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
            width:width height:height mipmapped:NO];
        descriptor.storageMode = MTLStorageModeShared;
        descriptor.usage = MTLTextureUsageShaderRead | MTLTextureUsageShaderWrite;
        id<MTLTexture> source = [device newTextureWithDescriptor:descriptor];
        id<MTLTexture> destination = [device newTextureWithDescriptor:descriptor];
        if (source == nil || destination == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "offscreen Metal texture allocation failed");
        }
        [source replaceRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0
                    withBytes:rgba bytesPerRow:(size_t)width * 4U];
        id<MTLCommandBuffer> command = [queue commandBuffer];
        id<MTLBlitCommandEncoder> blit = [command blitCommandEncoder];
        if (command == nil || blit == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "offscreen Metal blit encoder unavailable");
        }
        [blit copyFromTexture:source sourceSlice:0 sourceLevel:0
                 sourceOrigin:MTLOriginMake(0, 0, 0)
                       sourceSize:MTLSizeMake(width, height, 1)
                        toTexture:destination destinationSlice:0 destinationLevel:0
                       destinationOrigin:MTLOriginMake(0, 0, 0)];
        [blit endEncoding];
        if (!platform_metal_wait(command, diagnostic, diagnostic_capacity)) {
            return false;
        }
        return platform_metal_verify_readback(destination, rgba, byte_count,
            width, height, out_sha256, diagnostic, diagnostic_capacity);
    }
}

bool platform_metal_present_and_verify(platform_window *window, const uint8_t *rgba,
                                       size_t byte_count, uint32_t width,
                                       uint32_t height, char *diagnostic,
                                       size_t diagnostic_capacity)
{
    if (![NSThread isMainThread] || window == NULL || rgba == NULL ||
        !platform_metal_dimensions_valid(width, height, byte_count)) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "invalid Metal arguments or wrong thread");
    }
    @autoreleasepool {
        NSView *view = (__bridge NSView *)platform_window_native_view(window);
        if (view == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "window view unavailable");
        }
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        if (device == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal device unavailable");
        }
        CAMetalLayer *layer = (CAMetalLayer *)view.layer;
        if (![layer isKindOfClass:[CAMetalLayer class]]) {
            layer = [CAMetalLayer layer];
            layer.device = device;
            layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
            layer.framebufferOnly = NO;
            layer.contentsScale = view.window.backingScaleFactor;
            view.wantsLayer = YES;
            view.layer = layer;
        }
        layer.device = device;
        layer.framebufferOnly = NO;
        layer.drawableSize = CGSizeMake((CGFloat)width, (CGFloat)height);

        NSError *error = nil;
        static NSString *shader = @"#include <metal_stdlib>\n"
            "using namespace metal;\n"
            "struct V { float4 position [[position]]; float2 uv; };\n"
            "vertex V vmain(uint id [[vertex_id]]) {\n"
            "  float2 p[3] = {float2(-1,-1), float2(3,-1), float2(-1,3)};\n"
            "  float2 t[3] = {float2(0,1), float2(2,1), float2(0,-1)};\n"
            "  V o; o.position=float4(p[id],0,1); o.uv=t[id]; return o; }\n"
            "fragment float4 fmain(V in [[stage_in]], texture2d<float> tex [[texture(0)]]) {\n"
            "  constexpr sampler s(coord::normalized, address::clamp_to_edge, filter::nearest);\n"
            "  return tex.sample(s,in.uv); }\n";
        id<MTLLibrary> library = [device newLibraryWithSource:shader options:nil error:&error];
        if (library == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal shader compilation failed");
        }
        MTLRenderPipelineDescriptor *descriptor = [[MTLRenderPipelineDescriptor alloc] init];
        descriptor.vertexFunction = [library newFunctionWithName:@"vmain"];
        descriptor.fragmentFunction = [library newFunctionWithName:@"fmain"];
        descriptor.colorAttachments[0].pixelFormat = layer.pixelFormat;
        id<MTLRenderPipelineState> pipeline = [device newRenderPipelineStateWithDescriptor:descriptor error:&error];
        if (pipeline == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal pipeline creation failed");
        }
        MTLTextureDescriptor *texture_descriptor = [MTLTextureDescriptor
            texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
            width:width height:height mipmapped:NO];
        texture_descriptor.storageMode = MTLStorageModeShared;
        texture_descriptor.usage = MTLTextureUsageShaderRead;
        id<MTLTexture> texture = [device newTextureWithDescriptor:texture_descriptor];
        if (texture == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal tile texture allocation failed");
        }
        [texture replaceRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0
                    withBytes:rgba bytesPerRow:(size_t)width * 4U];
        id<CAMetalDrawable> drawable = [layer nextDrawable];
        if (drawable == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "CAMetalLayer returned no drawable");
        }
        id<MTLCommandQueue> queue = [device newCommandQueue];
        id<MTLCommandBuffer> command = [queue commandBuffer];
        if (command == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal command buffer unavailable");
        }
        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = drawable.texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        id<MTLRenderCommandEncoder> encoder = [command renderCommandEncoderWithDescriptor:pass];
        [encoder setRenderPipelineState:pipeline];
        [encoder setFragmentTexture:texture atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
        [encoder endEncoding];
        [command presentDrawable:drawable];
        dispatch_semaphore_t completed = dispatch_semaphore_create(0);
        [command addCompletedHandler:^(__unused id<MTLCommandBuffer> buffer) {
            dispatch_semaphore_signal(completed);
        }];
        [command commit];
        dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW,
            (int64_t)(PLATFORM_METAL_TIMEOUT_SECONDS * NSEC_PER_SEC));
        if (dispatch_semaphore_wait(completed, deadline) != 0) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal completion timed out after 5 seconds");
        }
        if (command.status != MTLCommandBufferStatusCompleted) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal command buffer failed");
        }
        uint8_t *readback = malloc(byte_count);
        if (readback == NULL) {
            return platform_metal_fail(diagnostic, diagnostic_capacity, "Metal readback allocation failed");
        }
        [texture getBytes:readback bytesPerRow:(size_t)width * 4U
            fromRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0];
        size_t mismatch = byte_count;
        for (size_t i = 0; i < byte_count; i++) {
            if (readback[i] != rgba[i]) {
                mismatch = i;
                break;
            }
        }
        if (mismatch != byte_count) {
            if (diagnostic != NULL && diagnostic_capacity > 0U) {
                (void)snprintf(diagnostic, diagnostic_capacity,
                    "Metal texture mismatch at byte %zu: expected %02x got %02x",
                    mismatch, rgba[mismatch], readback[mismatch]);
            }
            free(readback);
            return false;
        }
        free(readback);
        if (diagnostic != NULL && diagnostic_capacity > 0U) {
            (void)snprintf(diagnostic, diagnostic_capacity,
                           "byte-exact readback %ux%u (%zu bytes)", width, height, byte_count);
        }
        return true;
    }
}

static bool platform_metal_prepare_present(char *diagnostic, size_t diagnostic_capacity)
{
    if (platform_present_device != nil && platform_present_queue != nil &&
        platform_present_pipeline != nil) {
        return true;
    }
    id<MTLDevice> device = MTLCreateSystemDefaultDevice();
    id<MTLCommandQueue> queue = [device newCommandQueue];
    if (device == nil || queue == nil) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "Metal device or command queue unavailable");
    }
    static NSString *shader = @"#include <metal_stdlib>\n"
        "using namespace metal;\n"
        "struct V { float4 position [[position]]; float2 uv; };\n"
        "vertex V vmain(uint id [[vertex_id]]) {\n"
        "  float2 p[3] = {float2(-1,-1), float2(3,-1), float2(-1,3)};\n"
        "  float2 t[3] = {float2(0,1), float2(2,1), float2(0,-1)};\n"
        "  V o; o.position=float4(p[id],0,1); o.uv=t[id]; return o; }\n"
        "fragment float4 fmain(V in [[stage_in]], texture2d<float> tex [[texture(0)]]) {\n"
        "  constexpr sampler s(coord::normalized, address::clamp_to_edge, filter::nearest);\n"
        "  return tex.sample(s,in.uv); }\n";
    NSError *error = nil;
    id<MTLLibrary> library = [device newLibraryWithSource:shader options:nil error:&error];
    if (library == nil) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "Metal presentation shader compilation failed");
    }
    MTLRenderPipelineDescriptor *descriptor = [[MTLRenderPipelineDescriptor alloc] init];
    descriptor.vertexFunction = [library newFunctionWithName:@"vmain"];
    descriptor.fragmentFunction = [library newFunctionWithName:@"fmain"];
    descriptor.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
    id<MTLRenderPipelineState> pipeline =
        [device newRenderPipelineStateWithDescriptor:descriptor error:&error];
    if (pipeline == nil) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "Metal presentation pipeline creation failed");
    }
    platform_present_device = device;
    platform_present_queue = queue;
    platform_present_pipeline = pipeline;
    return true;
}

static bool platform_metal_present_dimensions_valid(uint32_t width, uint32_t height,
                                                     size_t byte_count)
{
    if (width == 0U || height == 0U || width > 8192U || height > 8192U) return false;
    size_t w = (size_t)width;
    size_t h = (size_t)height;
    if (w > SIZE_MAX / h) return false;
    size_t pixels = w * h;
    return pixels <= SIZE_MAX / 4U && pixels * 4U == byte_count;
}

bool platform_metal_present(platform_window *window, const uint8_t *rgba,
                            size_t byte_count, uint32_t width, uint32_t height,
                            char *diagnostic, size_t diagnostic_capacity)
{
    if (![NSThread isMainThread] || window == NULL || rgba == NULL ||
        !platform_metal_present_dimensions_valid(width, height, byte_count)) {
        return platform_metal_fail(diagnostic, diagnostic_capacity,
                                   "invalid Metal presentation arguments or wrong thread");
    }
    @autoreleasepool {
        if (!platform_metal_prepare_present(diagnostic, diagnostic_capacity)) {
            return false;
        }
        NSView *view = (__bridge NSView *)platform_window_native_view(window);
        if (view == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "window view unavailable");
        }
        CAMetalLayer *layer = (CAMetalLayer *)view.layer;
        if (![layer isKindOfClass:[CAMetalLayer class]]) {
            layer = [CAMetalLayer layer];
            layer.device = platform_present_device;
            layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
            layer.framebufferOnly = NO;
            layer.contentsScale = view.window.backingScaleFactor;
            view.wantsLayer = YES;
            view.layer = layer;
        }
        layer.device = platform_present_device;
        layer.frame = view.bounds;
        layer.contentsScale = view.window.backingScaleFactor;
        layer.drawableSize = CGSizeMake((CGFloat)width, (CGFloat)height);
        if (platform_present_texture == nil || platform_present_texture_width != width ||
            platform_present_texture_height != height) {
            MTLTextureDescriptor *texture_descriptor = [MTLTextureDescriptor
                texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                width:width height:height mipmapped:NO];
            texture_descriptor.storageMode = MTLStorageModeShared;
            texture_descriptor.usage = MTLTextureUsageShaderRead;
            id<MTLTexture> texture =
                [platform_present_device newTextureWithDescriptor:texture_descriptor];
            if (texture == nil) {
                return platform_metal_fail(diagnostic, diagnostic_capacity,
                                           "Metal presentation texture allocation failed");
            }
            platform_present_texture = texture;
            platform_present_texture_width = width;
            platform_present_texture_height = height;
        }
        [platform_present_texture replaceRegion:MTLRegionMake2D(0, 0, width, height)
            mipmapLevel:0 withBytes:rgba bytesPerRow:(size_t)width * 4U];
        id<CAMetalDrawable> drawable = [layer nextDrawable];
        if (drawable == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "CAMetalLayer returned no drawable");
        }
        id<MTLCommandBuffer> command = [platform_present_queue commandBuffer];
        if (command == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "Metal presentation command buffer unavailable");
        }
        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = drawable.texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        id<MTLRenderCommandEncoder> encoder =
            [command renderCommandEncoderWithDescriptor:pass];
        if (encoder == nil) {
            return platform_metal_fail(diagnostic, diagnostic_capacity,
                                       "Metal presentation encoder unavailable");
        }
        [encoder setRenderPipelineState:platform_present_pipeline];
        [encoder setFragmentTexture:platform_present_texture atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
        [encoder endEncoding];
        [command presentDrawable:drawable];
        [command commit];
        if (diagnostic != NULL && diagnostic_capacity > 0U) {
            (void)snprintf(diagnostic, diagnostic_capacity,
                           "presented %ux%u RGBA frame", width, height);
        }
        return true;
    }
}

void platform_metal_reset(platform_window *window)
{
    if (![NSThread isMainThread] || window == NULL) {
        return;
    }
    @autoreleasepool {
        NSView *view = (__bridge NSView *)platform_window_native_view(window);
        view.layer = nil;
        view.wantsLayer = NO;
        platform_present_texture = nil;
        platform_present_texture_width = 0U;
        platform_present_texture_height = 0U;
        platform_present_pipeline = nil;
        platform_present_queue = nil;
        platform_present_device = nil;
    }
}
