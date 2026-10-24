#!/usr/bin/env python3
"""Deterministic MCP driver: dashboard / loader scene, 512x512, 48 ticks ping_pong, via `batch`.

Run:   python3 -I scripts/scene_loader.py [--fast | --paced] [--no-alpha-variant]
Saves  /private/tmp/vexel-demo/loader.vexel, loader_a.png (0.5 s), loader_b.png (1.5 s),
       loader.mp4 (H.264) and the transparent variant loader_alpha.vexel / loader_alpha.mov
       (HEVC with alpha, no full-bleed background) plus loader_alpha_a/b.png (RGBA).

Contents: card with header and LIVE pill, a progress gauge (cubic-path circle whose
stroke-dashoffset is animated; attribute_set adds stroke-dasharray and round caps) with a
stepping percent label (visibility tracks on 11 label groups), a 12-dot ring (opacity chase +
slow group rotate), 8 chart bars (height + y scalar tracks, out_back, 3-tick stagger) over rails,
and three metric tiles whose fill rects animate their width.
"""
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scene_common as C  # noqa: E402
from scene_common import f, trs, TAU  # noqa: E402

T = 48
K = 0.5523


def rrect_d(x, y, w, h, r):
    k = r * K
    return ("M%s %s L%s %s C%s %s %s %s %s %s L%s %s C%s %s %s %s %s %s L%s %s C%s %s %s %s %s %s "
            "L%s %s C%s %s %s %s %s %s Z" % tuple(f(v) for v in (
                x + r, y, x + w - r, y, x + w - r + k, y, x + w, y + r - k, x + w, y + r,
                x + w, y + h - r, x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h,
                x + r, y + h, x + r - k, y + h, x, y + h - r + k, x, y + h - r,
                x, y + r, x, y + r - k, x + r - k, y, x + r, y)))


def circle_d(cx, cy, R):
    k = R * K
    return ("M%s %s C%s %s %s %s %s %s C%s %s %s %s %s %s C%s %s %s %s %s %s C%s %s %s %s %s %s" % tuple(
        f(v) for v in (
            cx, cy - R, cx + k, cy - R, cx + R, cy - k, cx + R, cy,
            cx + R, cy + k, cx + k, cy + R, cx, cy + R,
            cx - k, cy + R, cx - R, cy + k, cx - R, cy,
            cx - R, cy - k, cx - k, cy - R, cx, cy - R)))


def ease_in_out(t):
    return 2 * t * t if t < 0.5 else 1 - 2 * (1 - t) * (1 - t)


def text(b, x, y, s, size, fill, anchor="start"):
    return b.text(x, y, s, size, fill, anchor)


def build(b, alpha):
    # ---- background (omitted in the alpha variant) ----
    if not alpha:
        bg = b.add("rect", None, x=0, y=0, width=512, height=512, fill="#0a1020")
        b.paint(bg, b.gradient("linear", "0:#060b18:1,0.6:#0d1633:1,1:#1a1244:1", units="userSpace",
                               x1=0, y1=0, x2=512, y2=512))
        g1 = b.add("ellipse", None, cx=150, cy=150, rx=148, ry=148, fill="#000000")
        b.paint(g1, b.gradient("radial", "0:#2ee6c5:0.20,1:#2ee6c5:0", cx=0.5, cy=0.5, r=0.5))
        g2 = b.add("ellipse", None, cx=420, cy=430, rx=90, ry=80, fill="#000000")
        b.paint(g2, b.gradient("radial", "0:#9b6bff:0.30,1:#9b6bff:0", cx=0.5, cy=0.5, r=0.5))

    # ---- card: soft shadow stack, border, body ----
    for grow, dy, op in [(8, 6, 0.10), (5, 4, 0.14), (2, 2, 0.20)]:
        sh = b.add("path", None, d=rrect_d(16 - grow, 16 - grow + dy, 480 + 2 * grow, 480 + 2 * grow, 30 + grow),
                   fill="#02040c", stroke="none", stroke_width=1)
        sh.opacity = op
    border = b.add("path", None, d=rrect_d(16, 16, 480, 480, 30), fill="#ffffff", stroke="none", stroke_width=1)
    b.paint(border, b.gradient("linear", "0:#4562c8:1,0.5:#26356f:1,1:#6a4fd0:1", units="userSpace",
                               x1=16, y1=16, x2=496, y2=496))
    card = b.add("path", None, d=rrect_d(18, 18, 476, 476, 28), fill="#ffffff", stroke="none", stroke_width=1)
    b.paint(card, b.gradient("linear", "0:#18265a:1,1:#0d1535:1", units="userSpace", x1=18, y1=18, x2=300, y2=494))

    # ---- header ----
    text(b, 46, 66, "CLUSTER 04", 20, "#e4ecff")
    text(b, 46, 86, "ingest / scheduler / replica", 12, "#7086bd")
    pill = b.add("path", None, d=rrect_d(396, 46, 82, 28, 14), fill="#0f3a3a", stroke="none", stroke_width=1)
    dot = b.add("ellipse", None, cx=414, cy=60, rx=4.5, ry=4.5, fill="#2ee6c5")
    text(b, 428, 65, "LIVE", 13, "#5ff5da")
    b.track(dot, "opacity", "scalar", [(0, 1.0, "ease_in_out"), (24, 0.25, "ease_in_out"), (47, 1.0)], "linear")
    b.track(dot, "transform", "trs", [(0, trs(sx=1.0, sy=1.0, pivot=(414, 60)), "ease_in_out"),
                                      (24, trs(sx=1.5, sy=1.5, pivot=(414, 60)), "ease_in_out"),
                                      (47, trs(sx=1.0, sy=1.0, pivot=(414, 60)))], "linear")

    # ---- gauge ----
    GX, GY, R = 170, 200, 78
    glow = b.add("ellipse", None, cx=GX, cy=GY, rx=118, ry=118, fill="#000000")
    b.paint(glow, b.gradient("radial", "0:#2ee6c5:0.28,0.6:#3a6bff:0.10,1:#3a6bff:0", cx=0.5, cy=0.5, r=0.5))
    face = b.add("ellipse", None, cx=GX, cy=GY, rx=R - 14, ry=R - 14, fill="#0c1431")
    track_ring = b.add("path", None, d=circle_d(GX, GY, R), fill="none", stroke="#22336f", stroke_width=14)
    arc = b.add("path", None, d=circle_d(GX, GY, R), fill="none", stroke="#ffffff", stroke_width=14)
    b.paint(arc, b.gradient("linear", "0:#2ee6c5:1,0.55:#4d8dff:1,1:#9b6bff:1", units="userSpace",
                            x1=GX - R, y1=GY + R, x2=GX + R, y2=GY - R), "stroke")
    circ = TAU * R
    b.attr(arc, "stroke-linecap", "round")
    b.attr(arc, "stroke-dasharray", "%s %s" % (f(circ), f(circ)))
    b.attr(arc, "stroke-dashoffset", f(circ))
    b.attr(track_ring, "stroke-opacity", "0.9")
    arc_pts = [(t, circ * (1 - ease_in_out(t / 47.0) * 0.999) + 0.01, "linear") for t in range(0, T, 4)]
    if arc_pts[-1][0] != 47:
        arc_pts.append((47, circ * (1 - 0.999) + 0.01, "linear"))
    b.track(arc, "stroke-dashoffset", "scalar", [(t, v) for t, v, _ in arc_pts], "linear")
    # stepping percent label: 11 text groups toggled with step visibility tracks
    cur = None
    runs = []  # (label index, first tick, last tick)
    for t in range(T):
        j = int(round(ease_in_out(t / 47.0) * 10))
        if j != cur:
            runs.append([j, t, t])
            cur = j
        else:
            runs[-1][2] = t
    labels = {}
    for j in range(11):
        tx = text(b, GX, GY + 10, "%d%%" % (j * 10), 34, "#f2f6ff", "middle")
        labels[j] = b.group(tx)
    text(b, GX, GY + 36, "LOADING", 11, "#7f95cf", "middle")
    for j, g in labels.items():
        mine = [r for r in runs if r[0] == j]
        pts = [(0, bool(mine and mine[0][1] == 0), "step")]
        for r in mine:
            if r[1] != 0:
                pts.append((r[1], True, "step"))
            if r[2] + 1 < T:
                pts.append((r[2] + 1, False, "step"))
        b.track(g, "visibility", "boolean", pts, "step")
    # dot ring
    dots = []
    for k in range(12):
        a = TAU * k / 12.0 - math.pi / 2
        x, y = GX + 106 * math.cos(a), GY + 106 * math.sin(a)
        d = b.add("ellipse", None, cx=x, cy=y, rx=4.2, ry=4.2, fill="#7fa7ff")
        dots.append(d)
    dg = b.group(*dots)
    b.track(dg, "transform", "trs", [(0, trs(rot=0, pivot=(GX, GY)), "ease_in_out"),
                                     (47, trs(rot=30, pivot=(GX, GY)))], "linear")
    for k, d in enumerate(dots):
        pts = [(t, 0.12 + 0.88 * (0.5 + 0.5 * math.cos(TAU * (t / 47.0 - k / 12.0))) ** 2)
               for t in range(0, T, 4)]
        if pts[-1][0] != 47:
            pts.append((47, 0.12 + 0.88 * (0.5 + 0.5 * math.cos(TAU * (1 - k / 12.0))) ** 2))
        b.track(d, "opacity", "scalar", pts, "cubic")

    # ---- bar chart ----
    PX, PY, PW, PH = 296, 112, 182, 200
    panel = b.add("path", None, d=rrect_d(PX, PY, PW, PH, 18), fill="#0c1431", stroke="none", stroke_width=1)
    text(b, PX + 14, PY + 26, "THROUGHPUT", 12, "#8fa4dc")
    text(b, PX + PW - 14, PY + 26, "req/s", 11, "#5d73ab", "end")
    base = PY + PH - 30
    top = PY + 44
    maxh = base - top
    heights = [0.46, 0.78, 0.60, 1.00, 0.70, 0.88, 0.52, 0.74]
    bw, gap = 13, 8
    bx0 = PX + (PW - (8 * bw + 7 * gap)) / 2.0
    for i, hf in enumerate(heights):
        x = bx0 + i * (bw + gap)
        rail = b.add("rect", None, x=x, y=top, width=bw, height=maxh, fill="#16224a")
        H = maxh * hf
        bar = b.add("rect", None, x=x, y=base - H, width=bw, height=H, fill="#ffffff")
        hot = hf >= 1.0
        b.paint(bar, b.gradient("linear", "0:#ffcf6b:1,1:#ff6b8a:1" if hot else "0:#6ff3ff:1,1:#3a6bff:1",
                                x1=0, y1=0, x2=0, y2=1))
        text(b, x + bw / 2.0, base + 18, "MTWTFSSA"[i], 10, "#6f84b8", "middle")
        s = 3 * i
        e = s + 22
        small = 5.0
        kh = [(0, small, "linear"), (s, small, "ease_out_back"), (e, H, "linear"), (T - 1, H)]
        ky = [(0, base - small, "linear"), (s, base - small, "ease_out_back"), (e, base - H, "linear"),
              (T - 1, base - H)]
        b.track(bar, "height", "scalar", kh, "linear")
        b.track(bar, "y", "scalar", ky, "linear")

    # ---- metric tiles ----
    tiles = [("CPU", "72%", 0.72, "#2ee6c5", "#4d8dff"), ("MEMORY", "48%", 0.48, "#4d8dff", "#9b6bff"),
             ("DISK I/O", "91%", 0.91, "#ffcf6b", "#ff6b8a")]
    for i, (lab, val, frac, c1, c2) in enumerate(tiles):
        x, y, w, h = 40 + i * 148, 340, 136, 124
        tile = b.add("path", None, d=rrect_d(x, y, w, h, 18), fill="#0c1431", stroke="none", stroke_width=1)
        text(b, x + 16, y + 30, lab, 12, "#8fa4dc")
        text(b, x + 16, y + 74, val, 30, "#f2f6ff")
        trk = b.add("rect", None, x=x + 16, y=y + 92, width=w - 32, height=8, fill="#1a2858")
        fillw = (w - 32) * frac
        fill = b.add("rect", None, x=x + 16, y=y + 92, width=fillw, height=8, fill="#ffffff")
        b.paint(fill, b.gradient("linear", "0:%s:1,1:%s:1" % (c1, c2), units="userSpace",
                                 x1=x + 16, y1=0, x2=x + w - 16, y2=0))
        s = 6 + 4 * i
        b.track(fill, "width", "scalar", [(0, 2.0, "linear"), (s, 2.0, "ease_out_back"), (s + 24, fillw, "linear"),
                                          (T - 1, fillw)], "linear")


def run(name, alpha):
    m = C.Mcp("scene_loader")
    t0 = time.time()
    try:
        m.open_blank(name)
        m.call("native_timeline_set_duration", documentId=m.doc, durationTicks=T, ticksPerSecond=24)
        m.call("native_timeline_set_loop", documentId=m.doc, mode="ping_pong")
        b = C.Builder(m, "d")
        build(b, alpha)
        tb = time.time()
        stats = b.flush()
        t_build = time.time() - tb
        census = C.element_count(m)
        if alpha:
            exports = [dict(file=name + ".mov", format="mov", codec="hevc", alpha=True, width=512, height=512)]
        else:
            exports = [dict(file=name + ".mp4", format="mp4", width=512, height=512)]
        tm = C.export_all(m, name, (0.5, 1.5), exports)
        print("[%s] flush %.1fs %s" % (name, t_build, stats))
        print("[%s] elements %s" % (name, {k: v for k, v in census.items() if k != "tags"}))
        print("[%s] timing %s" % (name, {k: round(v, 2) for k, v in tm.items() if not k.startswith("reply")}))
        for k, v in tm.items():
            if k.startswith("reply"):
                print("[%s] %s %s" % (name, k, v))
        for line in C.timing_report(m):
            print("[%s] %s" % (name, line))
        print("[%s] requests=%d cost=%.1f elapsed=%.0fs" % (name, m.sent, m.cost, time.time() - t0))
    finally:
        m.close()
    out = C.OUT + "/" + name + (".mov" if alpha else ".mp4")
    if os.path.exists(out):
        print("[%s] ffprobe %s" % (name, C.ffprobe(out)["streams"]))


def main():
    os.makedirs(C.OUT, exist_ok=True)
    run("loader", False)
    if "--no-alpha-variant" not in sys.argv:
        run("loader_alpha", True)


if __name__ == "__main__":
    main()
