#!/usr/bin/env bash
# Headless benchmark: vexel-corpus compose (`--one`) vs Inkscape PNG export.
# Fixed list of 5 corpus SVGs (small -> large), read-only, paths passed as args.
# Per file: median wall-clock ms and median peak RSS over RUNS runs.
# Writes bench/results/<date>/results.json and results.md. Measures only.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_HOME="$REPO/.cache/cargo"

CORPUS="${VEXEL_CORPUS_DIR:-$HOME/Desktop/Freelance/vexel-corpus-real}"
SIZE="${BENCH_SIZE:-128x128}"
RUNS="${BENCH_RUNS:-3}"
TIMEOUT_SECS="${BENCH_TIMEOUT_SECS:-600}"
DATE="$(date +%Y-%m-%d)"
OUT_DIR="$REPO/bench/results/$DATE"

FILES=(
  "files/real/world-map-blank.svg"
  "files/generated/cnc_t1.svg"
  "files/real_inkscape/greater-coat-of-arms-of-blas-de-lezo.svg"
  "files/real_inkscape/kymijoki-drainage-basin-map-svg.svg"
  "files/real/geographic-map-of-norway-sweden-finland-and-denmark-svg.svg"
)

cd "$REPO"

# Release CLI for vexel-corpus; resolve the built executable from cargo's JSON output.
BIN="$(cargo build --release --offline --locked -p vexel-corpus --message-format=json | python3 -I -c '
import json, sys
exe = ""
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m["target"]["name"] == "vexel-corpus" \
            and "bin" in m["target"]["kind"] and m.get("executable"):
        exe = m["executable"]
print(exe)
')"
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "bench: vexel-corpus release binary not found" >&2
  exit 1
fi

# Inkscape: PATH first, then the macOS app bundle. Empty means not installed.
INK=""
if command -v inkscape >/dev/null 2>&1; then
  INK="$(command -v inkscape)"
elif [ -x /Applications/Inkscape.app/Contents/MacOS/inkscape ]; then
  INK=/Applications/Inkscape.app/Contents/MacOS/inkscape
fi

mkdir -p "$OUT_DIR"
ABS_FILES=()
for f in "${FILES[@]}"; do
  ABS_FILES+=("$CORPUS/$f")
done

python3 -I - "$BIN" "$INK" "$OUT_DIR" "$SIZE" "$RUNS" "$TIMEOUT_SECS" "$DATE" "${ABS_FILES[@]}" <<'PY'
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import time

bin_path, ink, out_dir, size, runs, timeout, date = sys.argv[1:8]
paths = sys.argv[8:]
runs = int(runs)
timeout = int(timeout)
TIME = "/usr/bin/time"
RSS_RE = re.compile(r"(\d+)\s+maximum resident set size")


def measure(argv):
    t0 = time.perf_counter()
    try:
        p = subprocess.run([TIME, "-l"] + argv, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": f"timeout after {timeout}s"}
    wall_ms = (time.perf_counter() - t0) * 1000.0
    err = p.stderr.decode("utf-8", "replace")
    m = RSS_RE.search(err)
    return {
        "ok": p.returncode == 0,
        "rc": p.returncode,
        "wall_ms": wall_ms,
        "rss_bytes": int(m.group(1)) if m else None,
        "stdout": p.stdout.decode("utf-8", "replace").strip(),
        "error": None if p.returncode == 0 else err.strip()[-300:],
    }


def vexel_argv(path):
    return [bin_path, "--one", path, "--size", size]


def inkscape_argv(path):
    return [ink, "--export-type=png", "--export-filename=/dev/null", path]


def summarize(samples):
    # samples: list of measure() dicts, all ok
    ms = [s["wall_ms"] for s in samples]
    rss = [s["rss_bytes"] for s in samples if s["rss_bytes"] is not None]
    return {
        "status": "ok",
        "runs_ms": [round(x, 3) for x in ms],
        "median_ms": round(statistics.median(ms), 3),
        "runs_rss_bytes": rss,
        "median_rss_bytes": int(statistics.median(rss)) if rss else None,
    }


def bench(argv_fn, path, check_vexel=False):
    samples = []
    for _ in range(runs):
        s = measure(argv_fn(path))
        if not s["ok"]:
            return {"status": "failed", "error": s.get("error") or f"rc={s.get('rc')}"}
        if check_vexel:
            parts = s["stdout"].split("\t")
            if len(parts) < 2 or parts[1] != "ok":
                return {"status": "failed",
                        "error": "compose verdict: " + (s["stdout"][:200] or "no output")}
        samples.append(s)
    return summarize(samples)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def tool_version(argv):
    try:
        p = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
        return p.stdout.decode("utf-8", "replace").strip().splitlines()[0]
    except Exception as e:  # version probe only
        return f"unavailable: {e}"


files = []
for path in paths:
    name = path.split("/vexel-corpus-real/", 1)[-1]
    print(f"bench: {name}", file=sys.stderr)
    entry = {
        "name": name,
        "bytes": os.path.getsize(path),
        "sha256": sha256(path),
        "vexel": bench(vexel_argv, path, check_vexel=True),
        "inkscape": bench(inkscape_argv, path) if ink else {"status": "not measured"},
    }
    files.append(entry)

mac = platform.mac_ver()[0]
results = {
    "date": date,
    "method": {
        "vexel": f"{os.path.basename(bin_path)} --one <file> --size {size} (compose only, raster frame)",
        "inkscape": "inkscape --export-type=png --export-filename=/dev/null <file> (document size)",
        "timing": "wall-clock perf_counter around /usr/bin/time -l; peak RSS = 'maximum resident set size'",
        "runs": runs,
        "statistic": "median",
    },
    "host": {"machine": platform.machine(), "macos": mac, "python": platform.python_version()},
    "vexel_binary": bin_path,
    "inkscape": {"path": ink, "version": tool_version([ink, "--version"])} if ink else None,
    "files": files,
}

with open(os.path.join(out_dir, "results.json"), "w") as f:
    json.dump(results, f, indent=2)
    f.write("\n")


def mib(b):
    return "n/a" if b is None else f"{b / 1048576:.1f}"


def cell(r, key):
    if r["status"] == "ok":
        return f"{r[key]:.1f}"
    return r["status"]


lines = [
    f"# Vexel vs Inkscape benchmark ({date})",
    "",
    f"Host: {results['host']['machine']}, macOS {mac}. Runs per file: {runs} (median).",
    f"Vexel: `vexel-corpus --one <file> --size {size}` (compose only).",
    f"Inkscape: {results['inkscape']['version'] if ink else 'not measured'} "
    "`--export-type=png --export-filename=/dev/null` (document size).",
    "Different workloads (compose vs PNG export): not a like-for-like speed claim.",
    "",
    "| File | Bytes | Vexel median ms | Vexel median peak RSS MiB | Inkscape median ms | Inkscape median peak RSS MiB | Notes |",
    "|---|---:|---:|---:|---:|---:|---|",
]
for e in files:
    v, i = e["vexel"], e["inkscape"]
    notes = []
    if v["status"] != "ok":
        notes.append("Vexel: " + v.get("error", ""))
    if i["status"] not in ("ok", "not measured"):
        notes.append("Inkscape: " + i.get("error", ""))
    vrss = mib(v.get("median_rss_bytes")) if v["status"] == "ok" else "-"
    irss = mib(i.get("median_rss_bytes")) if i["status"] == "ok" else "-"
    lines.append(
        f"| {e['name']} | {e['bytes']} | {cell(v, 'median_ms') if v['status'] == 'ok' else '-'} | {vrss} | "
        f"{cell(i, 'median_ms') if i['status'] == 'ok' else i['status']} | {irss} | "
        f"{'; '.join(notes).replace('|', '/')} |"
    )
lines.append("")
with open(os.path.join(out_dir, "results.md"), "w") as f:
    f.write("\n".join(lines))

print(f"bench: wrote {out_dir}/results.json and results.md", file=sys.stderr)
PY
