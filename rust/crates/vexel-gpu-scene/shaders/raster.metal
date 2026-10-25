// Tile-binned compute rasterizer (ADR-0035). One 16x16 threadgroup per tile,
// one thread per pixel. Mirrors src/reference.rs step for step: integer
// fixed-point crossing tests (i64), 4x4 samples, f32 gradient colours, the
// CPU's integer source-over and resolve. Prepended with <metal_stdlib> and
// shaders/layout.h by src/run.rs. No fast math; FP contraction is off so
// `a * b + c` stays two roundings, as in Rust.
#pragma clang fp contract(off)

#define VX_EDGE_CHUNK 256
#define VX_SAMPLE_UNIT (1 << (VX_EDGE_FIXED_SHIFT - 2))

struct VxMode {
    uint premultiply;
};

static inline uint4 vx_unpack(uint v) {
    return uint4(v & 255u, (v >> 8) & 255u, (v >> 16) & 255u, v >> 24);
}

static inline uint vx_pack(uint4 c) {
    return c.x | (c.y << 8) | (c.z << 16) | (c.w << 24);
}

// vexel-render composite::source_over.
static inline uint4 vx_source_over(uint4 d, uint4 s) {
    uint sa = s.w;
    if (sa == 0u) {
        return d;
    }
    uint da = d.w;
    uint inv = 255u - sa;
    uint alpha_num = sa * 255u + da * inv;
    uint3 num = s.xyz * (sa * 255u) + d.xyz * (da * inv);
    uint3 rgb = (num + alpha_num / 2u) / alpha_num;
    return uint4(rgb, (alpha_num + 127u) / 255u);
}

// Closes frames above `keep`: each layer pixel is composited into the frame
// below (or the canvas pixel) with the group's per-alpha opacity table
// (run frames: opacity 1), as vexel-render group.rs composite_layer.
static inline void vx_close(
    thread uint4* layer, thread uint* ids, thread uint& open, uint keep,
    thread uint4& pixel, device const GpuGroup* groups)
{
    while (open > keep) {
        open--;
        uint4 s = layer[open];
        uint a = s.w;
        if ((ids[open] & VX_FRAME_RUN) == 0u) {
            a = (groups[ids[open]].lut[a >> 2] >> ((a & 3u) * 8u)) & 255u;
        }
        if (a == 0u) {
            continue;
        }
        uint4 src = uint4(s.xyz, a);
        if (open == 0u) {
            pixel = vx_source_over(pixel, src);
        } else {
            layer[open - 1u] = vx_source_over(layer[open - 1u], src);
        }
    }
}

static inline float vx_spread(float t, uint mode) {
    if (mode == VX_SPREAD_PAD) {
        return t;
    }
    if (mode == VX_SPREAD_REFLECT) {
        float v = fmod(t, 2.0f);
        if (v < 0.0f) {
            v += 2.0f;
        }
        return v > 1.0f ? 2.0f - v : v;
    }
    return t - floor(t);
}

static inline float vx_radial_t(float fr, float gx, float gy) {
    float a = fr * fr - 1.0f;
    float pdx = gx - fr;
    float pdy = gy;
    float b = 2.0f * pdx * fr;
    float c = pdx * pdx + pdy * pdy;
    if (fabs(a) <= 1e-15f) {
        if (fabs(b) <= 1e-15f) {
            return NAN;
        }
        return -c / b;
    }
    float disc = b * b - 4.0f * a * c;
    if (disc < 0.0f || !isfinite(disc)) {
        return NAN;
    }
    float root = sqrt(disc);
    float t0 = (-b - root) / (2.0f * a);
    float t1 = (-b + root) / (2.0f * a);
    float t = max(t0, t1);
    if (t < 0.0f) {
        return min(t0, t1);
    }
    return t;
}

static inline uint vx_stop_byte(float v) {
    return uint(uchar(v));
}

// Gradient colour; returns false when the point is not placed.
static inline bool vx_gradient(
    device const GpuGradient& g, uint kind, device const GpuStop* stops, float x, float y,
    thread uint4& out)
{
    float gx = g.inverse[0] * x + g.inverse[2] * y + g.inverse[4];
    float gy = g.inverse[1] * x + g.inverse[3] * y + g.inverse[5];
    if (!(isfinite(gx) && isfinite(gy))) {
        return false;
    }
    float t = kind == VX_PAINT_RADIAL ? vx_radial_t(g.focal_ratio, gx, gy) : gx;
    if (!isfinite(t)) {
        return false;
    }
    t = vx_spread(t, g.spread);
    device const GpuStop* s = stops + g.stop_start;
    uint last = g.stop_count - 1u;
    if (t <= s[0].offset) {
        out = uint4(vx_stop_byte(s[0].rgba[0]), vx_stop_byte(s[0].rgba[1]),
                    vx_stop_byte(s[0].rgba[2]), vx_stop_byte(s[0].rgba[3]));
        return true;
    }
    if (t >= s[last].offset) {
        out = uint4(vx_stop_byte(s[last].rgba[0]), vx_stop_byte(s[last].rgba[1]),
                    vx_stop_byte(s[last].rgba[2]), vx_stop_byte(s[last].rgba[3]));
        return true;
    }
    uint right = 1u;
    while (right < last && t > s[right].offset) {
        right++;
    }
    float lo = s[right - 1u].offset;
    float range = s[right].offset - lo;
    float amount = range <= 0.0f ? 1.0f : (t - lo) / range;
    uint c[4];
    for (uint k = 0; k < 4; k++) {
        float v = (1.0f - amount) * s[right - 1u].rgba[k] + amount * s[right].rgba[k];
        c[k] = uint(uchar(clamp(round(v), 0.0f, 255.0f)));
    }
    out = uint4(c[0], c[1], c[2], c[3]);
    return true;
}

kernel void vx_raster(
    constant GpuSceneHeader& h [[buffer(0)]],
    device const GpuShape* shapes [[buffer(1)]],
    device const GpuEdge* edges [[buffer(2)]],
    device const GpuTile* tiles [[buffer(3)]],
    device const GpuTileEntry* list [[buffer(4)]],
    device const GpuGradient* gradients [[buffer(5)]],
    device const GpuStop* stops [[buffer(6)]],
    device const GpuGroup* groups [[buffer(7)]],
    constant VxMode& mode [[buffer(8)]],
    device uint* out [[buffer(9)]],
    uint2 tg [[threadgroup_position_in_grid]],
    uint2 lid [[thread_position_in_threadgroup]],
    uint lin [[thread_index_in_threadgroup]])
{
    threadgroup GpuEdge chunk[VX_EDGE_CHUNK];
    uint x = tg.x * VX_TILE_SIZE + lid.x;
    uint y = tg.y * VX_TILE_SIZE + lid.y;
    bool inside = x < h.width && y < h.height;
    GpuTile tile = tiles[tg.y * h.tiles_x + tg.x];

    uint4 pixel = vx_unpack(h.background_rgba);

    // Per-sample content-rectangle flags (sample column/row inside the range).
    uint inside_mask = 0u;
    for (uint s = 0; s < VX_SAMPLE_COUNT; s++) {
        uint col = x * 4u + (s & 3u);
        uint row = y * 4u + (s >> 2);
        bool ok = col >= h.content_x0 && col < h.content_x1
            && row >= h.content_y0 && row < h.content_y1;
        inside_mask |= ok ? (1u << s) : 0u;
    }

    // Open frames (layered groups and shape runs): ids and their layer pixels.
    uint4 layer[VX_MAX_FRAMES];
    uint ids[VX_MAX_FRAMES];
    uint open = 0u;

    for (uint e = 0; e < tile.list_count; e++) {
        GpuTileEntry entry = list[tile.list_start + e];
        GpuShape shape = shapes[entry.shape];
        uint shared = 0u;
        while (shared < open && shared < shape.frame_count && ids[shared] == shape.frames[shared]) {
            shared++;
        }
        vx_close(layer, ids, open, shared, pixel, groups);
        for (uint i = shared; i < shape.frame_count; i++) {
            ids[i] = shape.frames[i];
            layer[i] = uint4(0u);
        }
        open = shape.frame_count;
        int wind[VX_SAMPLE_COUNT];
        uint parity = 0u;
        for (uint s = 0; s < VX_SAMPLE_COUNT; s++) {
            wind[s] = 0;
        }
        for (uint base = 0; base < entry.edge_count; base += VX_EDGE_CHUNK) {
            uint n = min(uint(VX_EDGE_CHUNK), entry.edge_count - base);
            threadgroup_barrier(mem_flags::mem_threadgroup);
            if (lin < n) {
                chunk[lin] = edges[entry.edge_start + base + lin];
            }
            threadgroup_barrier(mem_flags::mem_threadgroup);
            for (uint i = 0; i < n; i++) {
                GpuEdge g = chunk[i];
                long dy = long(g.y1) - long(g.y0);
                for (uint sy = 0; sy < 4u; sy++) {
                    int py = int((lid.y * 4u + sy) * VX_SAMPLE_UNIT + VX_SAMPLE_UNIT / 2);
                    bool spans = (g.y0 <= py && g.y1 > py) || (g.y1 <= py && g.y0 > py);
                    if (!spans) {
                        continue;
                    }
                    long dyn = long(py) - long(g.y0);
                    long dx = long(g.x1) - long(g.x0);
                    for (uint sx = 0; sx < 4u; sx++) {
                        int px = int((lid.x * 4u + sx) * VX_SAMPLE_UNIT + VX_SAMPLE_UNIT / 2);
                        long num = (long(g.x0) - long(px)) * dy + dyn * dx;
                        if ((dy > 0 && num > 0) || (dy < 0 && num < 0)) {
                            uint s = sy * 4u + sx;
                            wind[s] += dy > 0 ? 1 : -1;
                            parity ^= 1u << s;
                        }
                    }
                }
            }
        }
        if (!inside) {
            continue;
        }
        bool evenodd = shape.fill_rule == VX_FILL_EVENODD;
        uint alpha_sum = 0u;
        uint3 acc = uint3(0u);
        for (uint s = 0; s < VX_SAMPLE_COUNT; s++) {
            bool covered = evenodd ? ((parity >> s) & 1u) != 0u : wind[s] != 0;
            if (!covered || ((inside_mask >> s) & 1u) == 0u) {
                continue;
            }
            uint4 color;
            if (shape.paint_kind == VX_PAINT_SOLID) {
                color = vx_unpack(shape.color_rgba);
            } else {
                float fx = float(x) + VX_SAMPLE_LOCATIONS[s & 3u];
                float fy = float(y) + VX_SAMPLE_LOCATIONS[s >> 2];
                if (!vx_gradient(gradients[shape.gradient_index], shape.paint_kind, stops, fx, fy, color)) {
                    continue;
                }
            }
            uint4 sample = vx_source_over(uint4(0u), color);
            alpha_sum += sample.w;
            acc += sample.xyz * sample.w;
        }
        if (alpha_sum == 0u) {
            continue;
        }
        uint4 src = uint4((acc + alpha_sum / 2u) / alpha_sum, (alpha_sum + VX_SAMPLE_COUNT / 2u) / VX_SAMPLE_COUNT);
        if (open == 0u) {
            pixel = vx_source_over(pixel, src);
        } else {
            layer[open - 1u] = vx_source_over(layer[open - 1u], src);
        }
    }
    vx_close(layer, ids, open, 0u, pixel, groups);
    if (inside) {
        if (mode.premultiply != 0u) {
            pixel.xyz = (pixel.xyz * pixel.w + 127u) / 255u;
        }
        out[y * h.width + x] = vx_pack(pixel);
    }
}
