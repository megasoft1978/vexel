// Metal compute rasteriser spike. Pixel semantics mirror vexel-render:
// 4x4 samples at {0.125,0.375,0.625,0.875}, ray cast to +x (half-open y),
// even-odd or non-zero, per-shape resolve (alpha-weighted RGB), integer
// straight-alpha source-over. Built with -fno-fast-math.
#include <metal_stdlib>
using namespace metal;

constant float LOC[4] = {0.125f, 0.375f, 0.625f, 0.875f};
constant uint SF = 28;  // floats per shape: bbox4, grad p0/p1 (4), 4 stops x (offset,r,g,b,a)

struct Params {
    uint width;
    uint height;
    uint tilesX;
    uint tilesY;
};

struct ShapeDispatch {
    uint shape;
    uint x0;
    uint y0;
    uint pad;
};

inline uint4 over(uint4 d, uint4 s) {
    if (s.w == 0) return d;
    uint inv = 255 - s.w;
    uint oan = s.w * 255 + d.w * inv;
    uint4 o;
    for (int c = 0; c < 3; c++) {
        uint num = s[c] * s.w * 255 + d[c] * d.w * inv;
        o[c] = (num + oan / 2) / oan;
    }
    o.w = (oan + 127) / 255;
    return o;
}

inline uint4 grad_color(device const float* f, float px, float py) {
    float dx = f[6] - f[4], dy = f[7] - f[5];
    float l2 = dx * dx + dy * dy;
    float t = l2 > 0.0f ? ((px - f[4]) * dx + (py - f[5]) * dy) / l2 : 0.0f;
    t = clamp(t, 0.0f, 1.0f);
    int i = 0;
    while (i < 2 && t > f[8 + (i + 1) * 5]) i++;
    float o0 = f[8 + i * 5], o1 = f[8 + (i + 1) * 5];
    float u = o1 > o0 ? clamp((t - o0) / (o1 - o0), 0.0f, 1.0f) : 0.0f;
    uint4 r;
    for (int c = 0; c < 4; c++) {
        float a = f[8 + i * 5 + 1 + c], b = f[8 + (i + 1) * 5 + 1 + c];
        r[c] = uint(floor(a + (b - a) * u + 0.5f));
    }
    return r;
}

inline uint4 shade(device const float* sf, device const uint* su, device const float4* edges,
                   uint s, uint2 p, uint4 dst) {
    device const float* f = sf + s * SF;
    float x = float(p.x), y = float(p.y);
    if (f[2] <= x || f[0] >= x + 1.0f || f[3] <= y || f[1] >= y + 1.0f) return dst;
    uint start = su[s * 4], cnt = su[s * 4 + 1], flags = su[s * 4 + 2], packed = su[s * 4 + 3];
    int wind[16];
    for (int i = 0; i < 16; i++) wind[i] = 0;
    for (uint i = 0; i < cnt; i++) {
        float4 E = edges[start + i];
        if (max(E.y, E.w) <= y || min(E.y, E.w) >= y + 1.0f) continue;
        if (max(E.x, E.z) <= x) continue;
        for (int sy = 0; sy < 4; sy++) {
            float py = y + LOC[sy];
            bool up = E.y <= py && E.w > py;
            bool dn = E.w <= py && E.y > py;
            if (!(up || dn)) continue;
            float xc = E.x + (py - E.y) * (E.z - E.x) / (E.w - E.y);
            int dir = up ? 1 : -1;
            for (int sx = 0; sx < 4; sx++)
                if (xc > x + LOC[sx]) wind[sy * 4 + sx] += dir;
        }
    }
    bool evenodd = (flags & 1) != 0, grad = (flags & 2) != 0;
    uint4 solid = uint4(packed & 255, (packed >> 8) & 255, (packed >> 16) & 255, packed >> 24);
    uint alphaSum = 0;
    uint3 pm = uint3(0);
    for (int sy = 0; sy < 4; sy++) {
        for (int sx = 0; sx < 4; sx++) {
            int w = wind[sy * 4 + sx];
            bool in = evenodd ? ((w & 1) != 0) : (w != 0);
            if (!in) continue;
            uint4 c = grad ? grad_color(f, x + LOC[sx], y + LOC[sy]) : solid;
            alphaSum += c.w;
            pm += c.xyz * c.w;
        }
    }
    if (alphaSum == 0) return dst;
    uint4 src;
    src.xyz = (pm + alphaSum / 2) / alphaSum;
    src.w = (alphaSum + 8) / 16;
    return over(dst, src);
}

inline uint pack(uint4 c) { return c.x | (c.y << 8) | (c.z << 16) | (c.w << 24); }
inline uint4 unpack(uint v) { return uint4(v & 255, (v >> 8) & 255, (v >> 16) & 255, v >> 24); }

// Strategy A: one 16x16 threadgroup per tile; per-pixel loop over the tile's z-ordered shape list.
kernel void tile_kernel(device const float* sf [[buffer(0)]], device const uint* su [[buffer(1)]],
                        device const float4* edges [[buffer(2)]], device const uint* tileStart [[buffer(3)]],
                        device const uint* tileList [[buffer(4)]], device uint* fb [[buffer(5)]],
                        constant Params& P [[buffer(6)]], uint2 gid [[thread_position_in_grid]],
                        uint2 tg [[threadgroup_position_in_grid]]) {
    if (gid.x >= P.width || gid.y >= P.height) return;
    uint t = tg.y * P.tilesX + tg.x;
    uint4 d = uint4(0);
    for (uint i = tileStart[t]; i < tileStart[t + 1]; i++) d = shade(sf, su, edges, tileList[i], gid, d);
    fb[gid.y * P.width + gid.x] = pack(d);
}

// Strategy B: one dispatch per shape over its pixel bbox, blending straight into the framebuffer.
kernel void shape_kernel(device const float* sf [[buffer(0)]], device const uint* su [[buffer(1)]],
                         device const float4* edges [[buffer(2)]], device uint* fb [[buffer(5)]],
                         constant Params& P [[buffer(6)]], constant ShapeDispatch& D [[buffer(7)]],
                         uint2 gid [[thread_position_in_grid]]) {
    uint2 p = uint2(D.x0 + gid.x, D.y0 + gid.y);
    if (p.x >= P.width || p.y >= P.height) return;
    uint idx = p.y * P.width + p.x;
    fb[idx] = pack(shade(sf, su, edges, D.shape, p, unpack(fb[idx])));
}
