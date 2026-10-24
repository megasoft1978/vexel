"""Shared helpers for the deterministic MCP demo scenes (scene_*.py).

Stdlib only. Speaks MCP JSON-RPC over stdio to `vexel --mcp-preview`.

Builder: scenes are declared as a virtual tree (leaf shapes + groups + gradients
+ tracks) and flushed in a handful of `batch` requests:
  1. gradient_add / clip_add            (one batch)
  2. leaf element_add / text_add        (batches of <= 256, document order)
  3. element_group by nesting level     (deepest first, one batch per level)
  4. clips, transforms                  (one batch)
  5. native timeline tracks with keys[] (batches of <= 256)
Request budget is 600/min per session; a batch costs 1 + ops/8. `Mcp` tracks a
sliding 60 s window of that cost and sleeps only when needed (fast mode) or
sleeps `PACED` seconds between calls (--paced, for screen recordings).
"""
import atexit
import collections
import json
import math
import os
import struct
import subprocess
import sys
import time
import zlib

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_APP_REL = "build/rust/aarch64-apple-darwin/release/vexel"
APP = os.path.join(REPO, _APP_REL)
if not os.path.exists(APP):
    # worktrees carry no release build: fall back to the main checkout's binary
    APP = "/Users/megasoft78/Desktop/Freelance/vexel/" + _APP_REL
OUT = "/private/tmp/vexel-demo"
TAU = 2 * math.pi
BUDGET = 560.0  # requests per rolling minute we allow ourselves (limit is 600)
PACED_DELAY = 0.35

PACED = "--paced" in sys.argv
FAST = "--fast" in sys.argv or not PACED


def f(v):
    return ("%.2f" % v).rstrip("0").rstrip(".")


# --------------------------------------------------------------------------
# MCP client
# --------------------------------------------------------------------------
class Mcp:
    def __init__(self, client="scene"):
        self.p = subprocess.Popen(
            [APP, "--mcp-preview", "--grant-dir", OUT],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1)
        atexit.register(self.close)
        self.n = 0
        self.sent = 0
        self.cost = 0.0
        self.window = collections.deque()  # (monotonic time, cost)
        self.last = 0.0
        self.stats = collections.defaultdict(list)  # tool -> [seconds]
        self.rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                                "clientInfo": {"name": client, "version": "1"}}, 1)
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        self.p.stdin.flush()
        self.doc = None

    def _throttle(self, cost):
        now = time.monotonic()
        if PACED:
            wait = PACED_DELAY - (now - self.last)
            if wait > 0:
                time.sleep(wait)
        while self.window and now - self.window[0][0] > 60:
            self.window.popleft()
        used = sum(c for _, c in self.window)
        if used + cost > BUDGET and self.window:
            time.sleep(max(0.0, 60.5 - (now - self.window[0][0])))
        self.window.append((time.monotonic(), cost))

    def rpc(self, method, params=None, cost=1.0):
        self._throttle(cost)
        self.n += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method,
                                       "params": params or {}}) + "\n")
        self.p.stdin.flush()
        self.sent += 1
        self.cost += cost
        line = self.p.stdout.readline()
        self.last = time.monotonic()
        if not line:
            raise RuntimeError("MCP server closed")
        return json.loads(line)

    def call(self, name, /, **args):
        cost = 1.0 + (len(args.get("ops", [])) / 8.0 if name == "batch" else 0.0)
        for _ in range(4):
            t0 = time.monotonic()
            r = self.rpc("tools/call", {"name": name, "arguments": args}, cost)
            self.stats[name].append(time.monotonic() - t0)
            res = r.get("result", {})
            sc = res.get("structuredContent", {})
            if "error" in r and not res:
                raise RuntimeError("%s failed: %s" % (name, r["error"]))
            if res.get("isError"):
                txt = json.dumps(sc).lower()
                if "budget" in txt or "rate limit" in txt:
                    time.sleep(5)
                    continue
                raise RuntimeError("%s failed: %s args=%s" % (name, json.dumps(sc)[:600],
                                                              json.dumps(args)[:300]))
            return sc
        raise RuntimeError("%s: rate limited" % name)

    def batch(self, ops, atomic=True):
        """ops: [(tool, arguments)] -> list of per-op result dicts."""
        out = []
        for i in range(0, len(ops), 256):
            chunk = ops[i:i + 256]
            sc = self.call("batch", documentId=self.doc, atomic=atomic,
                           ops=[{"tool": t, "arguments": dict(a, documentId=self.doc)} for t, a in chunk])
            if not sc.get("ok", False):
                raise RuntimeError("batch failed: %s" % json.dumps(sc)[:800])
            out.extend(r.get("result", {}) for r in sc["results"])
        return out

    def open_blank(self, name):
        os.makedirs(OUT, exist_ok=True)
        base = os.path.join(OUT, name + "_base.svg")
        with open(base, "w") as fh:
            fh.write('<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" '
                     'viewBox="0 0 512 512"></svg>')
        self.call("document_open", path=base)
        self.doc = self.call("document_list")["documents"][0]["documentId"]
        return self.doc

    def close(self):
        p = self.p
        if p.poll() is not None:
            return
        try:
            p.stdin.close()
        except Exception:
            pass
        try:
            p.wait(timeout=2)
        except Exception:
            p.terminate()
            try:
                p.wait(timeout=5)
            except Exception:
                p.kill()
                p.wait()


# --------------------------------------------------------------------------
# virtual scene tree
# --------------------------------------------------------------------------
class Node:
    __slots__ = ("kind", "tpl", "attrs", "parent", "children", "id", "grad", "opacity",
                 "text", "depth")

    def __init__(self, kind, tpl=None, attrs=None):
        self.kind = kind  # leaf | text | group | root
        self.tpl = tpl
        self.attrs = attrs or {}
        self.parent = None
        self.children = []
        self.id = None
        self.grad = None  # (Grad, "fill"|"stroke")
        self.opacity = None
        self.text = None
        self.depth = 0


class Grad:
    def __init__(self, kind, stops, kw):
        self.kind, self.stops, self.kw, self.id = kind, stops, kw, None


class Builder:
    def __init__(self, m, prefix="e"):
        self.m = m
        self.root = Node("root")
        self.root.id = "root"
        self.prefix = prefix
        self.count = 0
        self.grads = []
        self.clips = []  # (shape, abcd tuple, [(node)])
        self.tracks = []
        self.transforms = []
        self.attrs = []
        self.tid = 0
        self.leaves = []
        self.ngroups = 0

    # ---- declaration ----
    def _new_id(self):
        self.count += 1
        return "%s%d" % (self.prefix, self.count)

    def add(self, tpl, parent=None, **a):
        attrs = {k.replace("_", "-"): (v if isinstance(v, str) else f(v)) for k, v in a.items()}
        n = Node("leaf", tpl, attrs)
        n.id = self._new_id()
        self._attach(n, parent)
        return n

    def text(self, x, y, content, size, fill, anchor="start", parent=None):
        n = Node("text")
        n.text = dict(x=x, y=y, content=content, fontSize=size, anchor=anchor, fill=fill)
        self._attach(n, parent)
        return n

    def _attach(self, n, parent):
        parent = parent or self.root
        n.parent = parent
        parent.children.append(n)

    def group(self, *nodes):
        """Wrap sibling node(s) (same parent) in a new group; returns the group."""
        first = nodes[0]
        par = first.parent
        idx = par.children.index(first)
        g = Node("group")
        for n in nodes:
            par.children.remove(n)
            n.parent = g
            g.children.append(n)
        g.parent = par
        par.children.insert(idx, g)
        self.ngroups += 1
        return g

    def before(self, node, target):
        """Move `node` to sit directly before sibling `target` (same parent)."""
        par = target.parent
        node.parent.children.remove(node)
        node.parent = par
        par.children.insert(par.children.index(target), node)

    def gradient(self, kind, stops, **kw):
        g = Grad(kind, stops, kw)
        self.grads.append(g)
        return g

    def paint(self, node, grad, prop="fill"):
        node.grad = (grad, prop)

    def clip(self, shape, abcd, nodes):
        self.clips.append((shape, abcd, nodes))

    def set_transform(self, node, **kw):
        self.transforms.append((node, kw))

    def attr(self, node, name, value):
        """attribute_set on an existing attribute-capable element (e.g. stroke-dasharray)."""
        self.attrs.append((node, name, value))

    def track(self, target, prop, vtype, pts, interp="cubic", name=None):
        """pts: [(tick, value)] or [(tick, value, ease)] ascending, first tick 0."""
        assert pts[0][0] == 0
        self.tid += 1
        self.tracks.append(dict(trackId="t%d" % self.tid, node=target, property=prop,
                                valueType=vtype, interpolation=interp, pts=pts, name=name))

    # ---- flush ----
    def _walk(self, n, out):
        for c in n.children:
            if c.kind in ("leaf", "text"):
                out.append(c)
            else:
                self._walk(c, out)
        return out

    def _levels(self):
        groups = []

        def rec(n, d):
            for c in n.children:
                if c.kind == "group":
                    c.depth = d
                    groups.append(c)
                    rec(c, d + 1)
        rec(self.root, 0)
        return groups

    def flush(self):
        m = self.m
        # 1. gradients
        if self.grads:
            res = m.batch([("gradient_add", dict(kind=g.kind, stops=g.stops, **g.kw))
                           for g in self.grads])
            for g, r in zip(self.grads, res):
                g.id = r["gradientId"]
        clip_ids = []
        if self.clips:
            res = m.batch([("clip_add", dict(shape=s, a=a, b=b, c=c, d=d))
                           for s, (a, b, c, d), _ in self.clips])
            clip_ids = [r["clipId"] for r in res]
        # 2. leaves in document order
        leaves = self._walk(self.root, [])
        ops = []
        for n in leaves:
            if n.kind == "leaf":
                a = dict(parentId="root", template=n.tpl, attributes=n.attrs, id=n.id)
                if n.grad:
                    a["gradientId"] = n.grad[0].id
                    a["paint"] = n.grad[1]
                if n.opacity is not None:
                    a["opacity"] = n.opacity
                ops.append(("element_add", a))
            else:
                ops.append(("text_add", dict(n.text)))
        res = m.batch(ops)
        for n, r in zip(leaves, res):
            if n.kind == "text":
                n.id = r["elementId"]
        # 3. groups, deepest first
        groups = self._levels()
        for d in range(max([g.depth for g in groups], default=-1), -1, -1):
            lvl = [g for g in groups if g.depth == d]
            res = m.batch([("element_group", dict(elementIds=[c.id for c in g.children]))
                           for g in lvl])
            for g, r in zip(lvl, res):
                g.id = r["elementId"]
        # 4. clips + transforms
        ops = []
        for cid, (_, _, nodes) in zip(clip_ids, self.clips):
            for n in nodes:
                ops.append(("element_set_clip", dict(elementId=n.id, clipId=cid)))
        for n, kw in self.transforms:
            ops.append(("element_set_transform", dict(elementId=n.id, **kw)))
        for n, k, v in self.attrs:
            ops.append(("attribute_set", dict(elementId=n.id, name=k, value=str(v))))
        if ops:
            m.batch(ops)
        # 5. tracks
        ops = []
        for t in self.tracks:
            pts = t["pts"]
            a = dict(trackId=t["trackId"], targetId=t["node"].id, property=t["property"],
                     valueType=t["valueType"], interpolation=t["interpolation"],
                     value=pts[0][1])
            keys = []
            for p in pts[1:]:
                k = dict(tick=p[0], value=p[1])
                if len(p) > 2 and p[2]:
                    k["interp"] = p[2]
                keys.append(k)
            if keys:
                a["keys"] = keys
            if t["name"]:
                a["name"] = t["name"]
            ops.append(("native_timeline_add_property_track", a))
        if ops:
            m.batch(ops)
        return dict(leaves=len(leaves), groups=len(groups), gradients=len(self.grads),
                    tracks=len(self.tracks), keys=sum(len(t["pts"]) for t in self.tracks))


# --------------------------------------------------------------------------
# value helpers
# --------------------------------------------------------------------------
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


def ticks_every(total, step):
    t = list(range(0, total, step))
    if t[-1] != total - 1:
        t.append(total - 1)
    return t


# --------------------------------------------------------------------------
# PNG reading and diff (8-bit RGB/RGBA, non-interlaced)
# --------------------------------------------------------------------------
def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, w, h, ct = 8, b"", 0, 0, 6
    while pos < len(data):
        ln, = struct.unpack(">I", data[pos:pos + 4])
        typ = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + ln]
        if typ == b"IHDR":
            w, h, bd, ct = struct.unpack(">IIBB", body[:10])
            assert bd == 8 and ct in (2, 6)
        elif typ == b"IDAT":
            idat += body
        pos += 12 + ln
    bpp = 4 if ct == 6 else 3
    raw = zlib.decompress(idat)
    stride = w * bpp
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        ft = raw[i]
        line = bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if ft == 1:
                line[x] = (line[x] + a) & 255
            elif ft == 2:
                line[x] = (line[x] + b) & 255
            elif ft == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 255
            elif ft == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        rows.append(line)
        prev = line
    return w, h, bpp, rows


def png_diff(p1, p2):
    """Returns (max abs channel diff, mean abs diff, differing pixel count)."""
    w1, h1, b1, r1 = read_png(p1)
    w2, h2, b2, r2 = read_png(p2)
    assert (w1, h1, b1) == (w2, h2, b2), "size mismatch"
    mx, tot, cnt = 0, 0, 0
    for x, y in zip(r1, r2):
        if x == y:
            continue
        for k in range(0, len(x), b1):
            d = max(abs(x[k + j] - y[k + j]) for j in range(b1))
            if d:
                cnt += 1
                tot += d
                mx = max(mx, d)
    return mx, tot / float(w1 * h1), cnt


# --------------------------------------------------------------------------
# outputs
# --------------------------------------------------------------------------
def fresh(path):
    if os.path.exists(path):
        os.unlink(path)
    return path


def percentile(xs, q):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(q * len(xs)))] if xs else 0.0


def element_count(m):
    """tree_get census: shapes (rect/ellipse/path/line/text), groups and total."""
    counts = collections.Counter()
    cursor = 0
    while cursor is not None:
        sc = m.call("tree_get", documentId=m.doc, cursor=cursor, limit=512)
        for e in sc.get("entries", []):
            counts[e["tag"]] += 1
        cursor = sc.get("nextCursor")
    shapes = sum(counts[t] for t in ("rect", "ellipse", "path", "line", "text", "circle"))
    return dict(shapes=shapes, groups=counts["g"], total=shapes + counts["g"],
                tags=dict(counts))


def timing_report(m, extra=None):
    lines = []
    for name in ("batch", "render_png", "export_animation", "save_as"):
        xs = m.stats.get(name, [])
        if xs:
            lines.append("%s n=%d p50=%.0fms max=%.0fms total=%.1fs" % (
                name, len(xs), 1000 * percentile(xs, .5), 1000 * max(xs), sum(xs)))
    return lines


def native_counts(m):
    sc = m.call("native_timeline_get", documentId=m.doc)
    return sc


def export_all(m, name, still_secs, exports):
    """Save the document, render two stills and run the exports. Returns timing dict.

    exports: [dict(file=..., **export_animation args)]. Existing files are unlinked first.
    """
    t = {}
    probe = os.environ.get("SCENE_PROBE")  # dev: "0.5,1,2" -> extra stills, no exports
    if probe:
        for sec in probe.split(","):
            png = fresh(os.path.join(OUT, "%s_p%s.png" % (name, sec)))
            m.call("render_png", documentId=m.doc, path=png, width=512, height=512, time=float(sec))
        return t
    path = fresh(os.path.join(OUT, name + ".vexel"))
    t0 = time.monotonic()
    m.call("save_as", documentId=m.doc, path=path)
    t["save_as"] = time.monotonic() - t0
    for i, secs in enumerate(still_secs):
        png = fresh(os.path.join(OUT, "%s_%s.png" % (name, "ab"[i])))
        t0 = time.monotonic()
        m.call("render_png", documentId=m.doc, path=png, width=512, height=512, time=secs)
        t["render_png_%s" % "ab"[i]] = time.monotonic() - t0
    for ex in exports:
        ex = dict(ex)
        fn = fresh(os.path.join(OUT, ex.pop("file")))
        t0 = time.monotonic()
        r = m.call("export_animation", documentId=m.doc, path=fn, **ex)
        t["export_" + os.path.basename(fn)] = time.monotonic() - t0
        t["reply_" + os.path.basename(fn)] = r
    return t


def ffprobe(path):
    r = subprocess.run(["ffprobe", "-v", "error", "-count_frames", "-select_streams", "v:0",
                        "-show_entries", "stream=codec_name,width,height,nb_read_frames,r_frame_rate,pix_fmt:format=duration",
                        "-of", "json", path], capture_output=True, text=True)
    return json.loads(r.stdout)
