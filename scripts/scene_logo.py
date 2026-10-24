#!/usr/bin/env python3
"""Deterministic MCP driver: kinetic "VEXEL" logo reveal, 512x512, built with `batch`.

Run:   python3 -I scripts/scene_logo.py [--fast | --paced] [--no-loop-variant]
Saves  /private/tmp/vexel-demo/logo.vexel, logo_a.png (1.0 s), logo_b.png (2.75 s), logo.mp4
       (72 ticks, plays once) and the seamless variant logo_loop.vexel / logo_loop.mp4
       (96 ticks, letters exit with in_back, loops).

Text rendering decision: text_add only offers the platform's regular-weight default font
(no font-family/weight/stroke attributes), which is too thin for a logo. The letters V E X E L
are therefore filled polygon paths (bold geometric), one group per letter (extruded shadow,
gradient face, white flash overlay). text_add is used for the tagline.

Motion (24 ticks/s): letters pop with scale 0 -> 1.15 (ease_out) -> 1 (ease_out_back settle),
rotate/translate settle and opacity fade, staggered 4 ticks; underline wipes in (scaleX from its
left pivot) with a travelling glow head; 12 sparks in a rotating group ping their opacity; a flash
sweeps across the letters; tagline fades in. Loop variant adds an in_back exit and a right-pivot
underline wipe-out so tick N-1 matches tick 0.
"""
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scene_common as C  # noqa: E402
from scene_common import f, trs, TAU  # noqa: E402

CX, CY = 256, 236          # centre of the word and of the spark ring
LW, LH, ADV = 76, 150, 88  # letter width, height, advance
X0 = CX - (4 * ADV + LW) / 2.0
LY = CY - LH / 2.0 - 8
UL_Y = LY + LH + 28        # underline
UL_X0, UL_X1 = X0, X0 + 4 * ADV + LW


def poly(pts):
    return "M" + " L".join("%s %s" % (f(x), f(y)) for x, y in pts) + " Z"


def letter_d(ch, x, y, w, h, t):
    if ch == "V":
        a, c = 22.0, 9.0
        return poly([(x, y), (x + a, y), (x + w / 2, y + h * 0.5), (x + w - a, y), (x + w, y),
                     (x + w / 2 + c, y + h), (x + w / 2 - c, y + h)])
    if ch == "E":
        m = y + h / 2
        return poly([(x, y), (x + w, y), (x + w, y + t), (x + t, y + t), (x + t, m - t / 2),
                     (x + w * 0.86, m - t / 2), (x + w * 0.86, m + t / 2), (x + t, m + t / 2),
                     (x + t, y + h - t), (x + w, y + h - t), (x + w, y + h), (x, y + h)])
    if ch == "X":
        a = 25.0
        cx, cy = x + w / 2, y + h / 2
        e = (a / 2) * h / (w - a)
        return poly([(x, y), (x + a, y), (cx, cy - e), (x + w - a, y), (x + w, y), (cx + a / 2, cy),
                     (x + w, y + h), (x + w - a, y + h), (cx, cy + e), (x + a, y + h), (x, y + h),
                     (cx - a / 2, cy)])
    if ch == "L":
        return poly([(x, y), (x + t, y), (x + t, y + h - t), (x + w, y + h - t), (x + w, y + h),
                     (x, y + h)])
    raise ValueError(ch)


EPS = 0.001  # scale 0 makes a non-invertible matrix; use a hair above zero
SPARK_COLORS = ["#6ff3ff", "#ff7ad1", "#ffc56b", "#9d86ff"]


def build(b, T, loop):
    """T = total ticks. Declares the whole scene on Builder `b`."""
    P = T  # sparks / bg breathing period
    # ---- background ----
    bg = b.add("rect", None, x=0, y=0, width=512, height=512, fill="#0a0f2e")
    b.paint(bg, b.gradient("linear", "0:#070b26:1,0.5:#1c0f5c:1,1:#4b1472:1", units="userSpace",
                           x1=60, y1=0, x2=452, y2=512))
    glow = b.add("ellipse", None, cx=CX, cy=CY, rx=250, ry=190, fill="#000000")
    b.paint(glow, b.gradient("radial", "0:#8a5cff:0.62,0.55:#5b34d6:0.25,1:#3a1c9a:0",
                             cx=0.5, cy=0.5, r=0.5))
    warm = b.add("ellipse", None, cx=CX, cy=466, rx=250, ry=44, fill="#000000")
    b.paint(warm, b.gradient("radial", "0:#ff5fb0:0.5,1:#ff5fb0:0", cx=0.5, cy=0.5, r=0.5))
    b.track(glow, "opacity", "scalar",
            [(0, 0.0, "ease_out"), (20, 1.0)] + ([(T - 14, 1.0, "ease_in"), (T - 1, 0.0)] if loop else []),
            "linear")
    b.track(warm, "opacity", "scalar",
            [(0, 0.0, "ease_out"), (30, 1.0)] + ([(T - 14, 1.0, "ease_in"), (T - 1, 0.0)] if loop else []),
            "linear")
    # rings (depth) behind the word
    for i, (r, op) in enumerate([(176, 0.34), (214, 0.22), (246, 0.10)]):
        ring = b.add("ellipse", None, cx=CX, cy=CY, rx=r, ry=min(r, CY - 4), fill="#000000")
        b.paint(ring, b.gradient("radial", "0:#7a5cff:0,0.86:#7a5cff:0,0.93:#a58bff:%s,1:#7a5cff:0" % op,
                                 cx=0.5, cy=0.5, r=0.5))
        s0 = 0.82 + 0.05 * i
        b.track(ring, "transform", "trs",
                [(0, trs(sx=s0, sy=s0, pivot=(CX, CY)), "ease_out"),
                 (26 + 3 * i, trs(sx=1, sy=1, pivot=(CX, CY)), "linear"),
                 (T - 1, trs(sx=1.0 + 0.03 * (i % 2), sy=1.0 + 0.03 * (i % 2), pivot=(CX, CY)))],
                "linear")
        b.track(ring, "opacity", "scalar",
                [(0, 0.0, "ease_out"), (26 + 3 * i, 1.0)] + ([(T - 12, 1.0, "ease_in"), (T - 1, 0.0)] if loop else []),
                "linear")
    # bokeh
    for k, (x, y, r, col) in enumerate([(70, 88, 20, "#6ff3ff"), (448, 70, 26, "#ff7ad1"),
                                        (40, 370, 30, "#ff7ad1"), (470, 340, 22, "#6ff3ff"),
                                        (130, 458, 18, "#ffc56b"), (400, 450, 28, "#9d86ff"),
                                        (92, 232, 12, "#ffffff"), (428, 196, 14, "#ffffff")]):
        bk = b.add("ellipse", None, cx=x, cy=y, rx=r, ry=r, fill="#000000")
        b.paint(bk, b.gradient("radial", "0:%s:0.5,0.55:%s:0.22,1:%s:0" % (col, col, col),
                               cx=0.5, cy=0.5, r=0.5))
        ph = TAU * k / 8.0
        pts = [(t, 0.35 + 0.35 * (0.5 + 0.5 * math.sin(TAU * t / P + ph))) for t in C.ticks_every(T, 6)]
        b.track(bk, "opacity", "scalar", pts, "cubic")

    # ---- letters ----
    face_grad = b.gradient("linear", "0:#52e6ff:1,0.38:#8f7bff:1,0.7:#ff62c2:1,1:#ffb860:1",
                           units="userSpace", x1=X0, y1=LY, x2=UL_X1, y2=LY + LH)
    letters = []
    for i, ch in enumerate("VEXEL"):
        x = X0 + i * ADV
        d = letter_d(ch, x, LY, LW, LH, 29)
        shade = b.add("path", None, d=d, fill="#06041c", stroke="none", stroke_width=1)
        shade.opacity = 0.55
        face = b.add("path", None, d=d, fill="#ffffff", stroke="none", stroke_width=1)
        b.paint(face, face_grad)
        flash = b.add("path", None, d=d, fill="#ffffff", stroke="none", stroke_width=1)
        g = b.group(shade, face, flash)
        letters.append((g, shade, flash, x + LW / 2.0))
    for i, (g, shade, flash, lx) in enumerate(letters):
        piv = (lx, LY + LH / 2.0)
        s = 4 * i + 2
        S = lambda sc, rot, dy: trs(0, dy, rot, sc, sc, pivot=piv)
        pts = [(0, S(EPS, -16, 26), "linear"), (s, S(EPS, -16, 26), "ease_out"),
               (s + 9, S(1.15, 3, -4), "ease_out_back"), (s + 20, S(1.0, 0, 0), "linear")]
        if loop:
            e = 68 + 3 * i
            pts += [(e, S(1.0, 0, 0), "ease_in_back"), (e + 12, S(EPS, 10, 18), "linear"),
                    (T - 1, S(EPS, 10, 18))]
        else:
            pts.append((T - 1, S(1.0, 0, 0)))
        b.track(g, "transform", "trs", pts, "linear")
        op = [(0, 0.0, "linear"), (s, 0.0, "linear"), (s + 5, 1.0, "linear")]
        if loop:
            op += [(68 + 3 * i + 6, 1.0, "linear"), (68 + 3 * i + 12, 0.0, "linear"), (T - 1, 0.0)]
        else:
            op.append((T - 1, 1.0))
        b.track(g, "opacity", "scalar", op, "linear")
        # extrusion settles in with the pop
        b.track(shade, "transform", "trs",
                [(0, trs(0, 0), "linear"), (s + 8, trs(0, 0), "ease_out"), (s + 20, trs(5, 7), "linear"),
                 (T - 1, trs(5, 7))], "linear")
        # white flash sweeps over the letters left to right
        fs = 46 + 3 * i
        b.track(flash, "opacity", "scalar",
                [(0, 0.0, "linear"), (fs, 0.0, "ease_out"), (fs + 2, 0.7, "ease_in"), (fs + 8, 0.0, "linear"),
                 (T - 1, 0.0)], "linear")

    # ---- underline wipe + glow head ----
    ul = b.add("path", None, d=poly([(UL_X0, UL_Y - 4), (UL_X1, UL_Y - 4), (UL_X1 - 5, UL_Y + 4), (UL_X0, UL_Y + 4)]),
               fill="#ffffff", stroke="none", stroke_width=1)
    b.paint(ul, b.gradient("linear", "0:#52e6ff:1,0.5:#ff62c2:1,1:#ffb860:1", units="userSpace",
                           x1=UL_X0, y1=0, x2=UL_X1, y2=0))
    w0, w1 = 30, 52
    pivL = (UL_X0, UL_Y)
    pts = [(0, trs(sx=EPS, sy=1, pivot=pivL), "linear"), (w0, trs(sx=EPS, sy=1, pivot=pivL), "bezier:0.6,0,0.2,1"),
           (w1, trs(sx=1, sy=1, pivot=pivL), "linear"), (T - 1, trs(sx=1, sy=1, pivot=pivL))]
    if loop:  # wipe out toward the right edge
        pivR = (UL_X1, UL_Y)
        pts = [(0, trs(sx=EPS, sy=1, pivot=pivL), "linear"), (w0, trs(sx=EPS, sy=1, pivot=pivL), "bezier:0.6,0,0.2,1"),
               (w1, trs(sx=1, sy=1, pivot=pivL), "linear"), (T - 22, trs(sx=1, sy=1, pivot=pivR), "ease_in"),
               (T - 8, trs(sx=EPS, sy=1, pivot=pivR), "linear"), (T - 1, trs(sx=EPS, sy=1, pivot=pivR))]
    b.track(ul, "transform", "trs", pts, "linear")
    b.track(ul, "opacity", "scalar",  # hide the 0.1% sliver before the wipe starts
            [(0, 0.0, "linear"), (w0, 0.0, "linear"), (w0 + 1, 1.0, "linear")] +
            ([(T - 9, 1.0, "linear"), (T - 8, 0.0, "linear")] if loop else [(T - 1, 1.0)]), "linear")
    head = b.add("ellipse", None, cx=UL_X0, cy=UL_Y, rx=26, ry=26, fill="#000000")
    b.paint(head, b.gradient("radial", "0:#ffffff:0.95,0.25:#bfe9ff:0.55,1:#6ff3ff:0", cx=0.5, cy=0.5, r=0.5))
    span = UL_X1 - UL_X0
    b.track(head, "transform", "trs",
            [(0, trs(0, 0), "linear"), (w0, trs(0, 0), "bezier:0.6,0,0.2,1"), (w1, trs(span, 0), "linear"),
             (T - 1, trs(span, 0))], "linear")
    b.track(head, "opacity", "scalar",
            [(0, 0.0, "linear"), (w0, 0.0, "linear"), (w0 + 2, 1.0, "linear"), (w1 - 1, 1.0, "linear"),
             (w1 + 5, 0.0, "linear"), (T - 1, 0.0)], "linear")

    # ---- tagline ----
    tag = b.text(CX, UL_Y + 50, "VECTOR  MOTION  NATIVE", 20, "#c9bcff", "middle")
    tg = b.group(tag)
    b.track(tg, "opacity", "scalar",
            [(0, 0.0, "linear"), (44, 0.0, "ease_out"), (58, 0.85, "linear")] +
            ([(T - 20, 0.85, "ease_in"), (T - 8, 0.0, "linear"), (T - 1, 0.0)] if loop else [(T - 1, 0.85)]),
            "linear")
    b.track(tg, "transform", "trs",
            [(0, trs(0, 8), "linear"), (44, trs(0, 8), "ease_out"), (58, trs(0, 0), "linear"),
             (T - 1, trs(0, 0))], "linear")

    # ---- sparks ----
    sparks = []
    for k in range(12):
        ang = TAU * k / 12.0
        rr = 214 if k % 2 else 176  # ride the two orbit rings
        R = rr * 0.965
        x, y = CX + R * math.cos(ang), CY + R * math.sin(ang) * min(1.0, (CY - 4) / float(rr))
        col = SPARK_COLORS[k % 4]
        r = 4.0 + 3.0 * ((k * 5) % 3) / 2.0
        halo = b.add("ellipse", None, cx=x, cy=y, rx=r * 3.6, ry=r * 3.6, fill="#000000")
        b.paint(halo, b.gradient("radial", "0:%s:0.7,1:%s:0" % (col, col), cx=0.5, cy=0.5, r=0.5))
        core = b.add("ellipse", None, cx=x, cy=y, rx=r, ry=r, fill=col)
        sparks.append((b.group(halo, core), x, y, k))
    sg = b.group(*[s[0] for s in sparks])
    rot = [(0, trs(rot=0, pivot=(CX, CY)), "linear"), (T - 1, trs(rot=360.0 * (T - 1) / T, pivot=(CX, CY)))]
    b.track(sg, "transform", "trs", rot, "linear")
    gop = [(0, 0.0, "linear"), (14, 1.0, "linear")]
    if loop:
        gop += [(T - 14, 1.0, "linear"), (T - 1, 0.0)]
    else:
        gop.append((T - 1, 1.0))
    b.track(sg, "opacity", "scalar", gop, "linear")
    for g, x, y, k in sparks:
        c = (6 * k + 8) % (T - 14)
        pts = [(0, 0.4, "linear")]
        if c > 0:
            pts.append((c, 0.4, "ease_out"))
        pts += [(c + 3, 1.0, "ease_in"), (c + 12, 0.4, "linear"), (T - 1, 0.4)]
        b.track(g, "opacity", "scalar", pts, "linear")
        b.track(g, "transform", "trs",
                [(0, trs(sx=0.8, sy=0.8, pivot=(x, y)), "linear"), (c, trs(sx=0.8, sy=0.8, pivot=(x, y)), "ease_out"),
                 (c + 3, trs(sx=1.9, sy=1.9, pivot=(x, y)), "ease_in"), (c + 12, trs(sx=0.8, sy=0.8, pivot=(x, y)), "linear"),
                 (T - 1, trs(sx=0.8, sy=0.8, pivot=(x, y)))] if c > 0 else
                [(0, trs(sx=0.8, sy=0.8, pivot=(x, y)), "linear"), (T - 1, trs(sx=0.8, sy=0.8, pivot=(x, y)))],
                "linear")


def run(name, T, loop, stills, extra=()):
    m = C.Mcp("scene_logo")
    t0 = time.time()
    try:
        m.open_blank(name)
        m.call("native_timeline_set_duration", documentId=m.doc, durationTicks=T, ticksPerSecond=24)
        m.call("native_timeline_set_loop", documentId=m.doc, mode="loop" if loop else "once")
        b = C.Builder(m, "l")
        build(b, T, loop)
        tb = time.time()
        stats = b.flush()
        t_build = time.time() - tb
        census = C.element_count(m)
        tm = C.export_all(m, name, stills, [dict(file=name + ".mp4", format="mp4", width=512, height=512)] + list(extra))
        print("[%s] flush %.1fs %s" % (name, t_build, stats))
        print("[%s] elements %s" % (name, {k: v for k, v in census.items() if k != "tags"}))
        print("[%s] timing %s" % (name, {k: round(v, 2) for k, v in tm.items() if not k.startswith("reply")}))
        for line in C.timing_report(m):
            print("[%s] %s" % (name, line))
        print("[%s] requests=%d cost=%.1f elapsed=%.0fs" % (name, m.sent, m.cost, time.time() - t0))
    finally:
        m.close()
    print("[%s] ffprobe %s" % (name, C.ffprobe(C.OUT + "/%s.mp4" % name)["streams"]))


def main():
    os.makedirs(C.OUT, exist_ok=True)
    run("logo", 72, False, (1.0, 2.75))
    if "--no-loop-variant" not in sys.argv:
        run("logo_loop", 96, True, (1.5, 2.5))


if __name__ == "__main__":
    main()
