// Tile-binned compute rasterizer (ADR-0035). One 16x16 threadgroup per tile,
// one thread per pixel. Mirrors src/reference.rs step for step: integer
// fixed-point crossing tests (i64), 4x4 samples, f32 gradient colours, the
// CPU's integer source-over and resolve. Prepended with <metal_stdlib> and
// shaders/layout.h by src/run.rs. No fast math; FP contraction is off so
// `a * b + c` stays two roundings, as in Rust.
#pragma clang fp contract(off)

#define VX_EDGE_CHUNK 256
#define VX_PRIM_CHUNK 64
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

// vexel-render stroke.rs within(): squared distance against the squared radius.
static inline bool vx_within(float dx, float dy, float r) {
    return dx * dx + dy * dy <= r * r;
}

// vexel-render stroke_join.rs in_triangle (boundary inclusive).
static inline bool vx_in_triangle(float2 p, float2 a, float2 b, float2 c) {
    float d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
    float d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    float d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
    bool neg = d1 < 0.0f || d2 < 0.0f || d3 < 0.0f;
    bool pos = d1 > 0.0f || d2 > 0.0f || d3 > 0.0f;
    return !(neg && pos);
}

// Stroke primitive hit at local offset (px, py) from the tile base; mirrors
// reference.rs prim_hit.
static inline bool vx_prim_hit(threadgroup const GpuStrokePrim& q, float r, float px, float py) {
    uint k = q.kind;
    if ((k & VX_PRIM_JOIN) != 0u) {
        if (!(px >= q.w[8] && px <= q.w[10] && py >= q.w[9] && py <= q.w[11])) {
            return false;
        }
        float2 v = float2(q.w[0], q.w[1]);
        float2 p1 = float2(q.w[2], q.w[3]);
        float2 m = float2(q.w[4], q.w[5]);
        float2 p2 = float2(q.w[6], q.w[7]);
        float2 at = float2(px, py);
        uint jk = k & 255u;
        if (jk == VX_JOIN_ROUND) {
            return vx_within(px - v.x, py - v.y, r);
        }
        if (jk == VX_JOIN_MITER) {
            return vx_in_triangle(at, v, p1, m) || vx_in_triangle(at, v, m, p2);
        }
        return vx_in_triangle(at, v, p1, p2);
    }
    float ax = q.w[6];
    float ay = q.w[7];
    if ((k & VX_SEG_DOT) != 0u) {
        float dx = px - ax;
        float dy = py - ay;
        if ((k & (VX_SEG_ROUND_A | VX_SEG_ROUND_B)) != 0u) {
            return vx_within(dx, dy, r);
        }
        return (k & VX_SEG_DOT_SQUARE) != 0u && fabs(dx) <= r && fabs(dy) <= r;
    }
    float qx = px - q.w[0];
    float qy = py - q.w[1];
    float along = qx * q.w[2] + qy * q.w[3];
    float across = fabs(qx * q.w[3] - qy * q.w[2]);
    return (along >= q.w[4] && along <= q.w[5] && across <= r)
        || ((k & VX_SEG_ROUND_A) != 0u && vx_within(px - ax, py - ay, r))
        || ((k & VX_SEG_ROUND_B) != 0u && vx_within(px - q.w[8], py - q.w[9], r));
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
    device const GpuStrokePrim* prims [[buffer(8)]],
    constant VxMode& mode [[buffer(9)]],
    device uint* out [[buffer(10)]],
    uint2 tg [[threadgroup_position_in_grid]],
    uint2 lid [[thread_position_in_threadgroup]],
    uint lin [[thread_index_in_threadgroup]])
{
    threadgroup GpuEdge chunk[VX_EDGE_CHUNK];
    threadgroup GpuStrokePrim pchunk[VX_PRIM_CHUNK];
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
        // Stroke coverage: one bit per sample, set on the first primitive hit.
        uint smask = 0u;
        if ((shape.flags & VX_SHAPE_STROKE) != 0u && entry.prim_count > 0u) {
            float r = shape.stroke_width;
            float lxs[VX_SAMPLE_COUNT];
            float lys[VX_SAMPLE_COUNT];
            for (uint s = 0; s < VX_SAMPLE_COUNT; s++) {
                float dx = float(lid.x) + VX_SAMPLE_LOCATIONS[s & 3u];
                float dy = float(lid.y) + VX_SAMPLE_LOCATIONS[s >> 2];
                lxs[s] = shape.stroke_m[0] * dx + shape.stroke_m[2] * dy;
                lys[s] = shape.stroke_m[1] * dx + shape.stroke_m[3] * dy;
            }
            for (uint base = 0; base < entry.prim_count; base += VX_PRIM_CHUNK) {
                uint n = min(uint(VX_PRIM_CHUNK), entry.prim_count - base);
                threadgroup_barrier(mem_flags::mem_threadgroup);
                if (lin < n) {
                    pchunk[lin] = prims[entry.prim_start + base + lin];
                }
                threadgroup_barrier(mem_flags::mem_threadgroup);
                for (uint i = 0; i < n; i++) {
                    for (uint s = 0; s < VX_SAMPLE_COUNT; s++) {
                        if (((smask >> s) & 1u) == 0u && vx_prim_hit(pchunk[i], r, lxs[s], lys[s])) {
                            smask |= 1u << s;
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
            if (((inside_mask >> s) & 1u) == 0u) {
                continue;
            }
            bool covered = evenodd ? ((parity >> s) & 1u) != 0u : wind[s] != 0;
            bool stroked = ((smask >> s) & 1u) != 0u;
            uint4 sample = uint4(0u);
            for (uint pass = 0; pass < 2u; pass++) {
                uint kind = pass == 0u ? shape.paint_kind : shape.stroke_paint_kind;
                if (!(pass == 0u ? covered : stroked)) {
                    continue;
                }
                uint4 color;
                if (kind == VX_PAINT_SOLID) {
                    color = vx_unpack(pass == 0u ? shape.color_rgba : shape.stroke_color_rgba);
                } else {
                    float fx = float(x) + VX_SAMPLE_LOCATIONS[s & 3u];
                    float fy = float(y) + VX_SAMPLE_LOCATIONS[s >> 2];
                    uint gi = pass == 0u ? shape.gradient_index : shape.stroke_gradient_index;
                    if (!vx_gradient(gradients[gi], kind, stops, fx, fy, color)) {
                        continue;
                    }
                }
                sample = vx_source_over(sample, color);
            }
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
