#!/usr/bin/env python3
"""LEGACY per-call builder for the "Walk Home" scene (kept behind `scene_walk_home.py --legacy`).

Deterministic MCP driver: builds the animated "Walk Home" 512x512 scene.

Speaks MCP JSON-RPC over stdio to `vexel --mcp-preview`, builds the scene,
saves /private/tmp/vexel-demo/walk_home_legacy.vexel, exports walk_home_legacy.gif and
renders walk_home_legacy_a.png (0 s) and walk_home_legacy_b.png (2 s).

Run: python3 -I scripts/scene_walk_home.py --legacy  (or this file directly)
Stdlib only. Throttled below the 600 requests/minute per-session budget.
All motion is periodic over the 96-tick loop: every track samples a function
of period 96 ticks (cycles are whole numbers), so tick 96 == tick 0 and the
loop wrap is continuous. Scrolling layers translate by an exact multiple of
their pattern period over the loop.
"""
import atexit
import json
import math
import os
import random
import subprocess
import sys
import time

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
APP = os.path.join(REPO, "build/rust/aarch64-apple-darwin/release/vexel")
if not os.path.exists(APP):  # worktrees carry no release build
    APP = "/Users/megasoft78/Desktop/Freelance/vexel/build/rust/aarch64-apple-darwin/release/vexel"
OUT = "/private/tmp/vexel-demo"
LOOP = 96
TAU = 2 * math.pi
MIN_INTERVAL = 0.108  # < 555 requests/minute
PACED = "--paced" in sys.argv
if PACED:
    MIN_INTERVAL = 0.35

FAST = "--static" in sys.argv  # skip tracks (static art check)


# --------------------------------------------------------------------------
# MCP client
# --------------------------------------------------------------------------
class Mcp:
    def __init__(self):
        self.p = subprocess.Popen(
            [APP, "--mcp-preview", "--grant-dir", OUT],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.n = 0
        self.last = 0.0
        self.sent = 0
        self.rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                                "clientInfo": {"name": "scene_walk_home", "version": "1"}})
        self.note("notifications/initialized")
        tools = self.rpc("tools/list")["result"]["tools"]
        self.tool_names = {t["name"] for t in tools}

    def note(self, method):
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method}) + "\n")
        self.p.stdin.flush()

    def rpc(self, method, params=None):
        wait = MIN_INTERVAL - (time.monotonic() - self.last)
        if wait > 0:
            time.sleep(wait)
        self.n += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method,
                                       "params": params or {}}) + "\n")
        self.p.stdin.flush()
        self.last = time.monotonic()
        self.sent += 1
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("MCP server closed")
        return json.loads(line)

    def call(self, name, **args):
        for attempt in range(4):
            r = self.rpc("tools/call", {"name": name, "arguments": args})
            res = r.get("result", {})
            sc = res.get("structuredContent", {})
            if res.get("isError"):
                if "budget" in json.dumps(sc).lower() or "rate" in json.dumps(sc).lower():
                    time.sleep(5)
                    continue
                raise RuntimeError("%s failed: %s args=%s" % (name, sc, json.dumps(args)[:300]))
            return sc
        raise RuntimeError("%s: rate limited" % name)

    def close(self):
        try:
            self.p.stdin.close()
        except Exception:
            pass
        try:
            self.p.wait(timeout=10)
        except Exception:
            self.p.terminate()
            try:
                self.p.wait(timeout=5)
            except Exception:
                self.p.kill()
                self.p.wait()


m = None
DOC = "preview-document"


def f(v):
    return ("%.2f" % v).rstrip("0").rstrip(".")


def add(tpl, parent="root", **a):
    attrs = {k.replace("_", "-"): (v if isinstance(v, str) else f(v)) for k, v in a.items()}
    return m.call("element_add", documentId=DOC, parentId=parent, template=tpl,
                  attributes=attrs)["elementId"]


def path(d, fill="none", parent="root", stroke="none", sw=1):
    return add("path", parent, d=d, fill=fill, stroke=stroke, stroke_width=sw)


def ell(cx, cy, rx, ry, fill, parent="root"):
    if cx - rx >= 0 and cy - ry >= 0 and cx + rx <= 512 and cy + ry <= 512:
        return add("ellipse", parent, cx=cx, cy=cy, rx=rx, ry=ry, fill=fill)
    # off-canvas parts of scrolling layers: element_add ellipse must lie inside the canvas
    k = 0.5523
    d = ("M%s %s C%s %s %s %s %s %s C%s %s %s %s %s %s C%s %s %s %s %s %s C%s %s %s %s %s %s Z" % tuple(
        f(v) for v in (
            cx + rx, cy,
            cx + rx, cy + k * ry, cx + k * rx, cy + ry, cx, cy + ry,
            cx - k * rx, cy + ry, cx - rx, cy + k * ry, cx - rx, cy,
            cx - rx, cy - k * ry, cx - k * rx, cy - ry, cx, cy - ry,
            cx + k * rx, cy - ry, cx + rx, cy - k * ry, cx + rx, cy)))
    return path(d, fill, parent)


def rect(x, y, w, h, fill, parent="root"):
    if x >= 0 and y >= 0 and x + w <= 512 and y + h <= 512:
        return add("rect", parent, x=x, y=y, width=w, height=h, fill=fill)
    return path("M%s %s L%s %s L%s %s L%s %s Z" % (
        f(x), f(y), f(x + w), f(y), f(x + w), f(y + h), f(x), f(y + h)), fill, parent)


def group(el):
    return m.call("element_group", documentId=DOC, elementId=el)["elementId"]


def gradient(kind, stops, **kw):
    return m.call("gradient_add", documentId=DOC, kind=kind, stops=stops, **kw)["gradientId"]


def paint(el, gid, prop="fill"):
    m.call("element_set_paint", documentId=DOC, elementId=el, property=prop, gradientId=gid)


_tid = [0]


def track(target, prop, vtype, pts, interp="cubic"):
    """pts: [(tick, value)] ascending, first tick 0."""
    if FAST:
        return
    _tid[0] += 1
    tid = "t%d" % _tid[0]
    assert pts[0][0] == 0
    if "--per-key" in sys.argv:  # original behaviour: one request per key (~11 min at 555 req/min)
        m.call("native_timeline_add_property_track", documentId=DOC, trackId=tid,
               targetId=target, property=prop, valueType=vtype, interpolation=interp,
               value=pts[0][1])
        for tick, v in pts[1:]:
            m.call("native_timeline_set_property_key", documentId=DOC, trackId=tid,
                   tick=tick, value=v)
    else:  # bulk keys[]: one request per track (keeps the run under ~6 minutes)
        m.call("native_timeline_add_property_track", documentId=DOC, trackId=tid,
               targetId=target, property=prop, valueType=vtype, interpolation=interp,
               value=pts[0][1], keys=[{"tick": t, "value": v} for t, v in pts[1:]])


def ticks(step):
    t = list(range(0, LOOP, step))
    if t[-1] != LOOP - 1:
        t.append(LOOP - 1)
    return t


def sample(fn, step):
    return [(t, fn(t)) for t in ticks(step)]


def trs(tx=0.0, ty=0.0, rot=0.0, sx=1.0, sy=1.0, pivot=(0, 0)):
    v = {"translate": [round(tx, 3), round(ty, 3)], "rotate": round(rot, 3),
         "pivot": [pivot[0], pivot[1]]}
    if sx != 1.0 or sy != 1.0:
        v["scale"] = [round(sx, 4), round(sy, 4)]
    return v


def hex_rgba(h, a=1.0):
    h = h.lstrip("#")
    return [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)] + [a]


def mix(c1, c2, t):
    return [c1[i] + (c2[i] - c1[i]) * t for i in range(4)]


def rot_track(el, pivot, fn, step=4):
    track(el, "transform", "trs", sample(lambda t: trs(rot=fn(t), pivot=pivot), step))


def theta(t, cycles=4.0, ph=0.0):
    return TAU * cycles * t / LOOP + ph


# --------------------------------------------------------------------------
# geometry helpers
# --------------------------------------------------------------------------
def wave_path(fn, x0, x1, bottom=530, step=6):
    pts = []
    x = x0
    while x <= x1:
        pts.append((x, fn(x)))
        x += step
    d = "M%s %s " % (f(pts[0][0]), f(pts[0][1]))
    d += " ".join("L%s %s" % (f(x), f(y)) for x, y in pts[1:])
    d += " L%s %s L%s %s Z" % (f(pts[-1][0]), bottom, f(pts[0][0]), bottom)
    return d


def ribbon(fn, x0, x1, step=6):
    pts = []
    x = x0
    while x <= x1:
        pts.append((x, fn(x)))
        x += step
    return "M%s %s " % (f(pts[0][0]), f(pts[0][1])) + " ".join(
        "L%s %s" % (f(x), f(y)) for x, y in pts[1:])


def pine_d(cx, by, h, w, tiers=4):
    tw = w * 0.07
    left = [(cx - tw, by), (cx - tw, by - h * 0.16)]
    for i in range(tiers):
        yb = by - h * (0.12 + 0.88 * i / tiers)
        yt = by - h * (0.12 + 0.88 * (i + 1) / tiers) - h * 0.03
        W = w * (1 - 0.6 * i / tiers) / 2
        left.append((cx - W, yb))
        left.append((cx - W * 0.4, yt + (yb - yt) * 0.35))
    left.append((cx, by - h))
    right = [(2 * cx - x, y) for x, y in reversed(left[:-1])]
    pts = left + right
    return "M" + " L".join("%s %s" % (f(x), f(y)) for x, y in pts) + " Z"


def cloud_d(x, y, s):
    pts = [(0, 0), (-2, -10), (16, -12), (22, -26), (40, -22), (52, -36), (70, -24),
           (90, -28), (92, -10), (100, -4), (96, 0)]
    q = lambda i: "%s %s" % (f(x + pts[i][0] * s), f(y + pts[i][1] * s))
    return ("M%s Q%s %s Q%s %s Q%s %s Q%s %s Q%s %s L%s Z" %
            (q(0), q(1), q(2), q(3), q(4), q(5), q(6), q(7), q(8), q(9), q(10), q(0)))


def star4_d(cx, cy, r):
    k = r * 0.28
    return ("M%s %s Q%s %s %s %s Q%s %s %s %s Q%s %s %s %s Q%s %s %s %s Z" % (
        f(cx), f(cy - r), f(cx + k), f(cy - k), f(cx + r), f(cy),
        f(cx + k), f(cy + k), f(cx), f(cy + r),
        f(cx - k), f(cy + k), f(cx - r), f(cy),
        f(cx - k), f(cy - k), f(cx), f(cy - r)))


# scene function constants
PATH_P = 192
PATH_SHIFT = 384
XT = 200  # traveler screen x


def path_y(x):
    return 444 + 6 * math.sin(TAU * x / PATH_P + 0.5)


def scroll(el, shift, step=95):
    """Linear scroll to exactly -shift at tick 96 (keys 0 and 95)."""
    track(el, "transform", "trs",
          [(0, trs(0, 0, pivot=(0, 0))),
           (LOOP - 1, trs(-shift * (LOOP - 1) / LOOP, 0, pivot=(0, 0)))],
          interp="linear")


# --------------------------------------------------------------------------
# build
# --------------------------------------------------------------------------
def build():
    rnd = random.Random(7)

    # ---- sky ----
    sky = rect(0, 0, 512, 512, "#000000")
    paint(sky, gradient("linear", "0:#121c38:1,0.34:#4f3470:1,0.58:#a4507a:1,0.74:#e8796a:1,1:#ffc47a:1",
                        units="userSpace", x1=0, y1=0, x2=0, y2=512))

    # ---- stars ----
    star_ids = []
    for i in range(38):
        x = rnd.uniform(6, 506)
        y = rnd.uniform(6, 250) if i % 3 else rnd.uniform(6, 120)
        if (x - 256) ** 2 + (y - 278) ** 2 < 125 ** 2:
            y = rnd.uniform(6, 150)
        r = rnd.choice([0.8, 1.0, 1.2, 1.5, 1.8])
        color = rnd.choice(["#fff4e0", "#ffe8c8", "#e6ecff", "#ffffff"])
        e = ell(x, y, r, r, color)
        star_ids.append((e, rnd.randrange(LOOP), rnd.choice([1, 1, 2])))
    sparkles = []
    for (x, y, r) in [(60, 58, 5.5), (440, 70, 6.5), (330, 40, 4.5), (150, 120, 4.5),
                      (478, 168, 5), (28, 168, 4), (390, 150, 4), (210, 30, 4.5)]:
        e = path(star4_d(x, y, r), "#fff2d8")
        sparkles.append((e, x, y, rnd.randrange(LOOP)))
    for e, ph, k in star_ids:
        lo = rnd.uniform(0.18, 0.4)
        track(e, "opacity", "scalar",
              sample(lambda t: lo + (1 - lo) * (0.5 + 0.5 * math.sin(theta(t, k, TAU * ph / LOOP))), 12))
    for e, x, y, ph in sparkles:
        def sp(t, x=x, y=y, ph=ph):
            s = 0.5 + 0.5 * math.sin(theta(t, 1, TAU * ph / LOOP))
            return trs(rot=0, sx=0.55 + 0.6 * s, sy=0.55 + 0.6 * s, pivot=(x, y))
        track(e, "transform", "trs", sample(sp, 12))
        track(e, "opacity", "scalar",
              sample(lambda t, ph=ph: 0.35 + 0.65 * (0.5 + 0.5 * math.sin(theta(t, 1, TAU * ph / LOOP))), 12))

    # ---- sun ----
    SX, SY = 256, 278
    halo = ell(SX, SY, 232, 232, "#000000")
    paint(halo, gradient("radial", "0:#ffcf8c:0.5,0.45:#ff9e7a:0.2,1:#ff8a70:0", cx=0.5, cy=0.5, r=0.5))
    sun = group(halo)
    ring2 = ell(SX, SY, 168, 168, "#000000", sun)
    paint(ring2, gradient("radial", "0:#ffd9a0:0.11,1:#ffd9a0:0.11", cx=0.5, cy=0.5, r=0.5))
    ring1 = ell(SX, SY, 134, 134, "#000000", sun)
    paint(ring1, gradient("radial", "0:#ffe0a8:0.16,1:#ffe0a8:0.16", cx=0.5, cy=0.5, r=0.5))
    disc = ell(SX, SY, 100, 100, "#FFC76F", sun)
    inner = ell(SX, SY, 43, 43, "#FFE9A2", sun)
    track(halo, "opacity", "scalar", sample(lambda t: 0.8 + 0.2 * math.sin(theta(t, 1, 0.6)), 12))
    track(ring2, "opacity", "scalar", sample(lambda t: 0.75 + 0.25 * math.sin(theta(t, 1, 1.6)), 12))
    track(ring1, "opacity", "scalar", sample(lambda t: 0.8 + 0.2 * math.sin(theta(t, 1, 2.6)), 12))
    c1, c2 = hex_rgba("#FFC06A"), hex_rgba("#FFD890")
    track(disc, "fill", "rgba", sample(lambda t: mix(c1, c2, 0.5 + 0.5 * math.sin(theta(t, 1, 0.3))), 12))
    i1, i2 = hex_rgba("#FFE29A"), hex_rgba("#FFF6CE")
    track(inner, "fill", "rgba", sample(lambda t: mix(i1, i2, 0.5 + 0.5 * math.sin(theta(t, 1, 1.3))), 12))
    track(sun, "transform", "trs", sample(
        lambda t: trs(0, 5 * math.sin(theta(t, 1, 0)), 0,
                      1 + 0.02 * math.sin(theta(t, 1, 1.0)), 1 + 0.02 * math.sin(theta(t, 1, 1.0)),
                      pivot=(SX, SY)), 8))

    # ---- cloud banks ----
    def bank(shift, per, color, specs, rim):
        # specs: [(local_x, y, scale)], copies every `per` px to cover 0..512+shift
        first = None
        gid = None
        ox = 0
        while ox < 512 + shift + 10:
            for (lx, y, s) in specs:
                d = cloud_d(ox + lx, y, s)
                if gid is None:
                    first = path(d, color)
                    gid = group(first)
                else:
                    path(d, color, gid)
                if rim:
                    path("M%s %s L%s %s L%s %s Z" % (
                        f(ox + lx + 8 * s), f(y - 0.2 * s), f(ox + lx + 94 * s), f(y - 0.2 * s),
                        f(ox + lx + 92 * s), f(y - 3.2 * s)), rim, gid)
            ox += per
        scroll(gid, shift)

    bank(128, 128, "#4a3872", [(10, 78, 0.55)], None)
    bank(256, 256, "#9a5a86", [(20, 128, 0.9), (150, 168, 0.65)], "#e49480")
    bank(384, 384, "#d27f84", [(40, 214, 1.15)], "#ffb585")

    # ---- birds ----
    def bird(cx, cy, size, phase, color="#2a2442", amp=(34, 9), flaps=4):
        d = "M%s %s Q%s %s %s %s Q%s %s %s %s" % (
            f(cx - size), f(cy), f(cx - size / 2), f(cy - size * 0.75), f(cx), f(cy),
            f(cx + size / 2), f(cy - size * 0.75), f(cx + size), f(cy))
        e = path(d, "none", stroke=color, sw=1.8)

        def fn(t):
            u = TAU * t / LOOP
            flap = math.cos(TAU * flaps * t / LOOP + phase)
            return trs(amp[0] * math.sin(u + phase), amp[1] * math.sin(2 * u + phase), 0,
                       1, 0.15 + 0.85 * flap, pivot=(cx, cy))
        track(e, "transform", "trs", sample(fn, 3))

    bird(112, 150, 9, 0.0)
    bird(150, 172, 7, 1.7)
    bird(334, 112, 8, 3.1, amp=(30, 7))
    bird(405, 206, 6, 4.4, amp=(22, 6))

    # ---- hills ----
    far_y = lambda x: 318 + 11 * math.sin(TAU * x / 128 + 0.4) + 5 * math.sin(TAU * 3 * x / 128 + 1.2)
    far = path(wave_path(far_y, 0, 660), "#000000")
    paint(far, gradient("linear", "0:#b4638a:1,1:#6a4a78:1", units="userSpace", x1=0, y1=290, x2=0, y2=420))
    scroll(far, 128)

    haze = rect(0, 280, 512, 120, "#000000")
    paint(haze, gradient("linear", "0:#ffb070:0,1:#ffb070:0.3", units="userSpace", x1=0, y1=280, x2=0, y2=360))

    mid_y = lambda x: 356 + 17 * math.sin(TAU * x / 192 + 0.3) + 6 * math.sin(TAU * 2 * x / 192 + 1.7)
    mid = path(wave_path(mid_y, 0, 720), "#000000")
    paint(mid, gradient("linear", "0:#6a5488:1,1:#3b3562:1", units="userSpace", x1=0, y1=335, x2=0, y2=430))
    scroll(mid, 192)

    # ---- village layer (hump + cottage + pines), P=256 shift 256 ----
    hump_y = lambda x: 396 + 6 * math.sin(TAU * x / 256 + 1.0)
    hump = path(wave_path(hump_y, 0, 780), "#2a2e56")
    vil = group(hump)
    window_glows = []
    ox = 0
    while ox < 512 + 256 + 10:
        cx = ox + 150
        b = hump_y(cx) + 4
        path("M%s %s L%s %s L%s %s L%s %s Z" % (f(cx - 33), f(b), f(cx - 33), f(b - 30), f(cx + 33), f(b - 30), f(cx + 33), f(b)),
             "#2f3454", vil)
        path("M%s %s L%s %s L%s %s Z" % (f(cx - 41), f(b - 28), f(cx), f(b - 62), f(cx + 41), f(b - 28)), "#45355a", vil)
        path("M%s %s L%s %s L%s %s L%s %s L%s %s Z" % (f(cx - 41), f(b - 28), f(cx), f(b - 62), f(cx + 41), f(b - 28), f(cx + 36), f(b - 28), f(cx), f(b - 55)),
             "#5a4670", vil)
        rect(cx + 14, b - 66, 7, 14, "#2f3454", vil)  # chimney
        w = rect(cx - 22, b - 24, 12, 14, "#FFD982", vil)
        rect(cx - 22, b - 18, 12, 1, "#8a5a3a", vil)
        rect(cx + 8, b - 26, 11, 26, "#1a2038", vil)  # door
        g = ell(cx - 16, b - 17, 28, 24, "#000000", vil)
        paint(g, gradient("radial", "0:#ffd982:0.5,1:#ffd982:0", cx=0.5, cy=0.5, r=0.5))
        window_glows.append((w, g, ox))
        # pines
        for (lx, h, ww) in [(40, 62, 30), (228, 48, 24)]:
            px = ox + lx
            path(pine_d(px, hump_y(px) + 6, h, ww), "#1a2848", vil)
        ox += 256
    scroll(vil, 256)
    wa, wb = hex_rgba("#FFC45E"), hex_rgba("#FFEAA8")
    for k, (w, g, o) in enumerate(window_glows):
        ph = 0.9 * k
        track(w, "fill", "rgba", sample(lambda t, ph=ph: mix(wa, wb, 0.5 + 0.5 * math.sin(theta(t, 2, ph))), 6))
        track(g, "opacity", "scalar", sample(lambda t, ph=ph: 0.65 + 0.35 * math.sin(theta(t, 2, ph)), 6))

    # ---- near ground P=384 shift 384 ----
    gnd_y = lambda x: 411 + 7 * math.sin(TAU * x / 384 + 0.8) + 3.5 * math.sin(TAU * 2 * x / 384 + 2.0)
    gnd = path(wave_path(gnd_y, 0, 900), "#000000")
    paint(gnd, gradient("linear", "0:#235560:1,1:#10303f:1", units="userSpace", x1=0, y1=395, x2=0, y2=512))
    scroll(gnd, 384)

    # near pines, same speed as ground
    npines = None
    ox = 0
    while ox < 512 + 384 + 10:
        for (lx, h, w) in [(70, 98, 44), (255, 72, 34)]:
            d = pine_d(ox + lx, 438, h, w)
            if npines is None:
                npines = group(path(d, "#0d2837"))
            else:
                path(d, "#0d2837", npines)
        ox += 384
    scroll(npines, 384)

    # ---- path ribbon with pebbles ----
    rib = ribbon(path_y, 0, 900)
    pr = path(rib, "none", stroke="#d8a574", sw=26)
    pg = group(pr)
    path(rib, "none", pg, stroke="#f0c48e", sw=14)
    path(ribbon(lambda x: path_y(x) - 10, 0, 900), "none", pg, stroke="#f6d4a3", sw=1.5)
    for k in range(0, 9):
        x = 40 + k * 100 + rnd.uniform(-20, 20)
        y = path_y(x) + rnd.uniform(-5, 6)
        ell(x, y, rnd.uniform(1.8, 3.4), rnd.uniform(1, 1.8), rnd.choice(["#b98557", "#c79a68", "#9d6f4a"]), pg)
    scroll(pg, PATH_SHIFT)

    # ---- traveler ----
    build_traveler()

    # ---- fireflies ----
    halo_g = gradient("radial", "0:#ffe9a0:0.6,1:#ffe9a0:0", cx=0.5, cy=0.5, r=0.5)
    spots = [(68, 402), (122, 458), (156, 376), (262, 470), (310, 420), (352, 466),
             (395, 392), (446, 436), (474, 482), (230, 356)]
    for i, (x, y) in enumerate(spots[:9]):
        h = ell(x, y, 7, 7, "#000000")
        paint(h, halo_g)
        fg = group(h)
        ell(x, y, 1.9, 1.9, "#fff2b0", fg)
        a, b = rnd.uniform(10, 26), rnd.uniform(8, 18)
        k1, k2 = rnd.choice([1, 2]), rnd.choice([1, 2])
        p1, p2 = rnd.uniform(0, TAU), rnd.uniform(0, TAU)
        track(fg, "transform", "trs", sample(
            lambda t: trs(a * math.sin(theta(t, k1, p1)), b * math.sin(theta(t, k2, p2)), pivot=(x, y)), 8))
        k3, p3 = rnd.choice([2, 3]), rnd.uniform(0, TAU)
        track(fg, "opacity", "scalar",
              sample(lambda t: 0.12 + 0.88 * (0.5 + 0.5 * math.sin(theta(t, k3, p3))) ** 2, 8))

    # ---- foreground grass ----
    fg_y = lambda x: 496 + 5 * math.sin(TAU * x / 256 + 2.0)
    fgl = group(path(wave_path(fg_y, 0, 1040), "#0b202e"))
    tufts = []
    ox = 0
    while ox < 512 + 512 + 10:
        for lx, hh in [(34, 20), (118, 26), (200, 17)]:
            x = ox + lx
            by = fg_y(x) + 4
            d = ("M%s %s Q%s %s %s %s M%s %s Q%s %s %s %s M%s %s Q%s %s %s %s" % (
                f(x), f(by), f(x - 3), f(by - hh * 0.55), f(x - 8), f(by - hh * 0.9),
                f(x), f(by), f(x), f(by - hh * 0.6), f(x + 1), f(by - hh),
                f(x), f(by), f(x + 3), f(by - hh * 0.5), f(x + 9), f(by - hh * 0.8)))
            tufts.append((path(d, "none", fgl, stroke="#79a98a", sw=1.8), x, by))
        ox += 256
    scroll(fgl, 512)
    for i, (e, x, by) in enumerate(tufts):
        ph = rnd.uniform(0, TAU)
        rot_track(e, (x, by), lambda t, ph=ph: 6 * math.sin(theta(t, 2, ph)), 8)


def build_traveler():
    T = None
    # lantern light pool on path (behind legs)
    pool = ell(XT + 24, 448, 44, 8, "#000000")
    paint(pool, gradient("radial", "0:#ffd070:0.5,1:#ffd070:0", cx=0.5, cy=0.5, r=0.5))
    T = group(pool)
    ell(XT + 2, 445.5, 24, 3.6, "#0d2330", T)  # shadow

    def leg(color, boot, ph_sign, label):
        thigh = path("M193.5 393 Q200 390 207 393 L205 421 L195 421 Z", color, T)
        lg = group(thigh)
        shin = path("M195.5 420 L205 420 L203.5 438 L196.5 438 Z", color, lg)
        sg = group(shin)
        path("M195 436 L204 436 L205.5 440.5 Q211 441.5 211 445 L194 445 Q194 440 195 436 Z", boot, sg)
        hip = (200, 394)
        knee = (200, 421)
        rot_track(lg, hip, lambda t: ph_sign * 25 * math.sin(theta(t)), 4)
        rot_track(sg, knee, lambda t: 6 + 15 * (1 - math.cos(theta(t, 4, 0 if ph_sign > 0 else math.pi))), 4)
        return lg

    leg("#232b47", "#1c1626", -1, "back")
    leg("#34406a", "#2b2030", 1, "front")

    body_ref = path("M187 361 Q200 353 213 361 L215 396 Q200 401 185 396 Z", "#c0603f", T)
    body = group(body_ref)
    # move torso above? backpack must be behind torso; reorder below
    bp = path("M177 360 Q171 361 171 368 L171 389 Q171 393 177 393 L190 393 L190 360 Z", "#8a5a3a", body)
    path("M171 376 L190 376 L190 379 L171 379 Z", "#6e452d", body)
    roll = ell(180, 358.5, 11, 4.5, "#5d8a78", body)
    bpg = bp
    # group backpack pieces: wrap first piece, add the rest
    bpgroup = group(bp)
    m.call("element_reorder", documentId=DOC, elementId=bpgroup, targetId=body_ref, before=True)
    ell(180, 358.5, 11, 4.5, "#5d8a78", bpgroup)
    m.call("element_delete", documentId=DOC, elementId=roll)
    path("M171 376 L190 376 L190 379 L171 379 Z", "#6e452d", bpgroup)
    rot_track(bpgroup, (190, 364), lambda t: 2.4 * math.sin(theta(t, 4, 1.3)), 4)

    # belt + jacket highlight
    rect(185, 390, 30, 3.4, "#6b3a2a", body)
    path("M205 360 L213 361 L215 396 L209 397 Z", "#d4704d", body)
    # far (back) arm, drawn before torso visually: reorder behind
    bua = path("M195.5 363 L203 363 L203.5 380 L197 380 Z", "#8f4632", body)
    bag = group(bua)
    m.call("element_reorder", documentId=DOC, elementId=bag, targetId=body_ref, before=True)
    bfa = path("M196.5 379 L203 379 L203.5 394 L198 394 Z", "#8f4632", bag)
    bfg = group(bfa)
    ell(200.5, 396, 3.4, 3.6, "#d9a889", bfg)
    rot_track(bag, (199, 364), lambda t: 27 * math.sin(theta(t)), 4)
    rot_track(bfg, (200, 380), lambda t: -(16 + 12 * math.sin(theta(t, 4, 0.9))), 4)

    # head group
    hair = ell(197, 334, 6, 8.5, "#5a3a2a", body)
    hg = group(hair)
    path("M200 349 L208 349 L209 357 L201 357 Z", "#e3b090", hg)
    ell(204, 334, 12.5, 13, "#f1c3a0", hg)
    ell(197, 336, 3, 3.8, "#e3b090", hg)  # ear
    eye1 = ell(209.5, 334, 1.7, 2.2, "#2a1d2b", hg)
    eyes = group(eye1)
    ell(215, 334, 1.6, 2.1, "#2a1d2b", eyes)
    ell(212, 340, 3.2, 2.1, "#f08e86", hg)
    path("M209 343 Q212.5 345.5 216 342", "none", hg, stroke="#8a4a3a", sw=1.2)
    ell(218, 337, 1.7, 2, "#e7a98a", hg)
    dome = path("M190 332 Q190 311 205 310.5 Q219 311.5 219.5 331 Q205 326.5 190 332 Z", "#2f4666", hg)
    hat = group(dome)
    path("M188.5 330.5 Q205 325.5 224 330 L225 334.5 Q205 331 187.5 335 Z", "#243550", hat)
    path("M190 322 Q205 318 219 322 L219 325 Q205 321 190 325 Z", "#e8b04a", hat)
    pom = ell(205, 309.5, 4.6, 4.2, "#e8b04a", hat)
    pg = group(pom)
    # pom was already in hat; group nests it
    rot_track(hg, (203, 350), lambda t: 2.2 * math.sin(theta(t, 4, 0.5)), 4)
    rot_track(hat, (205, 330), lambda t: 3.0 * math.sin(theta(t, 4, 1.4)), 4)
    rot_track(pg, (205, 313), lambda t: 14 * math.sin(theta(t, 8, 0.2)), 4)
    # blink
    def blink(t):
        v = 1.0
        for c in (38, 82):
            d = abs(t - c)
            if d < 2.5:
                v = min(v, 0.12 + 0.88 * d / 2.5)
        return v
    track(eyes, "transform", "trs",
          [(t, trs(0, 0, 0, 1, blink(t), pivot=(212, 334))) for t in (0, 34, 36, 38, 40, 42, 78, 80, 82, 84, 86, 95)],
          interp="linear")

    # scarf wrap + tails (drawn over head neck, before head front? after body)
    tail1 = path("M193 354 Q182 352 172 357 L173 366 Q184 362 194 362 Z", "#e04656", body)
    t1 = group(tail1)
    tail2 = path("M173 357 Q165 355 155 361 L156 369 Q166 365 173 366 Z", "#c93447", t1)
    t2 = group(tail2)
    path("M191 351.5 Q205 360 219 351 L218 359 Q205 367 192 359 Z", "#e04656", body)
    rot_track(t1, (193, 358), lambda t: 6 * math.sin(theta(t, 8, 1.0)), 4)
    rot_track(t2, (173, 361), lambda t: 15 * math.sin(theta(t, 8, 2.1)), 4)

    # near (lantern) arm
    nua = path("M204.5 363 L211.5 363 L214 379 L207.5 380 Z", "#d06a48", body)
    nag = group(nua)
    nfa = path("M208.5 377.5 L224 381.5 L223 388 L208 384 Z", "#d06a48", nag)
    nfg = group(nfa)
    ell(225.5, 385.5, 3.4, 3.4, "#f1c3a0", nfg)
    # lantern
    lstr = path("M225.5 388 L225.5 393", "none", nfg, stroke="#3a2f2a", sw=1.1)
    lg = group(lstr)
    lh = ell(225.5, 402, 30, 30, "#000000", lg)
    paint(lh, gradient("radial", "0:#ffd070:0.55,0.4:#ffb85a:0.2,1:#ffb85a:0", cx=0.5, cy=0.5, r=0.5))
    rect(220.5, 393, 10, 2.2, "#3a2f2a", lg)
    rect(221.5, 395, 8, 11, "#FFD36E", lg)
    ell(225.5, 400.5, 2.6, 3.6, "#fff6c4", lg)
    rect(220.5, 406, 10, 2.2, "#3a2f2a", lg)
    rect(224.8, 395, 1.4, 11, "#8a6a3a", lg)
    rot_track(nag, (208, 364), lambda t: -11 * math.sin(theta(t)), 4)
    rot_track(nfg, (209, 380), lambda t: 4 * math.sin(theta(t, 4, 1.0)), 4)
    rot_track(lg, (225.5, 388), lambda t: 13 * math.sin(theta(t, 4, 2.2)) + 3 * math.sin(theta(t, 8, 0.7)), 4)
    track(lh, "opacity", "scalar",
          sample(lambda t: 0.8 + 0.12 * math.sin(theta(t, 5, 0.4)) + 0.08 * math.sin(theta(t, 9, 1.1)), 4))

    # body secondary motion + whole traveler motion
    track(body, "transform", "trs", sample(
        lambda t: trs(0, -1.2 * math.cos(theta(t, 8, 0.6)), 1.6 * math.sin(theta(t, 4, 0.4)), pivot=(200, 396)), 4))

    def tfn(t):
        s = PATH_SHIFT * t / LOOP
        dy = path_y(XT + s) - path_y(XT)
        bob = -1.8 * (1 + math.cos(2 * theta(t)))
        return trs(0, dy + bob, pivot=(200, 440))
    track(T, "transform", "trs", sample(tfn, 2))

    # ---- companion ball ----
    BX = 292
    ballshadow = ell(BX, 447, 9, 2.4, "#0d2330")
    bs = ballshadow
    ball = ell(BX, 431, 8.5, 8.5, "#e8564a")
    bg = group(ball)
    path("M%s 431 Q%s 425 %s 431" % (f(BX - 8.4), f(BX), f(BX + 8.4)), "#f6a24a", bg)
    ell(BX - 3, 427.5, 2.4, 1.8, "#ffd0b8", bg)

    def ball_fn(t):
        s = PATH_SHIFT * t / LOOP
        ground = path_y(BX + s) - path_y(BX)
        ph = (t % 24) / 24.0
        h = 30 * 4 * ph * (1 - ph)
        d = min(ph, 1 - ph) * 24  # ticks from contact
        sq = max(0.0, 1 - d / 2.0)
        sy = 1 - 0.22 * sq + 0.08 * (h / 30)
        sx = 1 + 0.2 * sq - 0.04 * (h / 30)
        return trs(0, ground - h, 0, sx, sy, pivot=(BX, 439.5))
    track(bg, "transform", "trs", sample(ball_fn, 2), interp="linear")
    track(bs, "transform", "trs", sample(
        lambda t: trs(0, path_y(BX + PATH_SHIFT * t / LOOP) - path_y(BX), 0,
                      1 - 0.35 * (4 * ((t % 24) / 24.0) * (1 - (t % 24) / 24.0)), 1, pivot=(BX, 447)), 2),
          interp="linear")


def main():
    global m
    os.makedirs(OUT, exist_ok=True)
    base = os.path.join(OUT, "walk_home_legacy_base.svg")
    with open(base, "w") as fh:
        fh.write('<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512"></svg>')
    for leftover in ("walk_home_legacy.vexel", "walk_home_legacy.gif", "walk_home_legacy_a.png",
                     "walk_home_legacy_b.png"):
        if os.path.exists(os.path.join(OUT, leftover)):
            os.unlink(os.path.join(OUT, leftover))
    m = Mcp()
    atexit.register(m.close)
    t0 = time.time()
    try:
        global DOC
        m.call("document_open", path=base)
        DOC = m.call("document_list")["documents"][0]["documentId"]
        m.call("native_timeline_set_duration", documentId=DOC, durationTicks=LOOP, ticksPerSecond=24)
        m.call("native_timeline_set_loop", documentId=DOC, mode="loop")
        build()
        if "--probe" in sys.argv or FAST:
            m.call("render_png", documentId=DOC, path=OUT + "/walk_home_legacy_a.png", width=512, height=512, time=0)
            if FAST:
                return
        m.call("save_as", documentId=DOC, path=OUT + "/walk_home_legacy.vexel")
        m.call("render_png", documentId=DOC, path=OUT + "/walk_home_legacy_a.png", width=512, height=512, time=0)
        m.call("render_png", documentId=DOC, path=OUT + "/walk_home_legacy_b.png", width=512, height=512, time=2)
        m.call("export_animation", documentId=DOC, path=OUT + "/walk_home_legacy.gif", format="gif", fps=24, duration=4)
    finally:
        m.close()
    a = open(OUT + "/walk_home_legacy_a.png", "rb").read()
    b = open(OUT + "/walk_home_legacy_b.png", "rb").read()
    print("requests=%d elapsed=%.0fs a!=b: %s" % (m.sent, time.time() - t0, a != b))


if __name__ == "__main__":
    main()
