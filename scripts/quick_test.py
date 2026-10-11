#!/usr/bin/env python3
"""Fast, precise test runner for the inner loop (e2e/soak/native/fuzz belong to ./dev final).

  quick_test.py                  tests mapped to files changed vs main (+ uncommitted), plus 'always'
  quick_test.py -k PATTERN       ctest -R PATTERN and python -k PATTERN
  quick_test.py --all            every CTest (parallel) + the cheap Python modules (no shape-render/realworld/e2e)
  quick_test.py --list           show what would run

Prints one compact line per test group with timings, then the exact failure output of the FIRST failing test.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAP = json.loads((ROOT / "tests" / "test_map.json").read_text(encoding="utf-8"))
FULL = [False]
SLOW_PY = ("tests.test_shape_render", "tests.realworld", "tests.e2e", "tests.test_wo_runner",
           "tests.test_benchmark_harness", "tests.test_xml_fuzz_campaign", "tests.test_ui_driver",
           "tests.test_platform_driver")


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, **kw)


def changed_files(base: str) -> list[str]:
    names: set[str] = set()
    for args in (["git", "diff", "--name-only", f"{base}...HEAD"], ["git", "diff", "--name-only"],
                 ["git", "diff", "--name-only", "--cached"],
                 ["git", "ls-files", "--others", "--exclude-standard"]):
        out = run(args)
        if out.returncode == 0:
            names.update(line for line in out.stdout.splitlines() if line)
    return sorted(names)


def select(files: list[str]) -> tuple[list[str], list[str], list[str]]:
    ctest = [MAP["always"]["ctest"]]
    python = list(MAP["always"]["python"])
    unmapped = []
    for path in files:
        hit = False
        if path.startswith("tests/") and path.endswith(".py") and Path(path).name.startswith("test_"):
            python.append(".".join(Path(path).with_suffix("").parts))
            continue
        for entry in MAP["map"]:
            if path.startswith(entry["prefix"]):
                ctest.append(entry["ctest"])
                python.extend(entry["python"])
                hit = True
                break
        if not hit and not path.startswith(("docs/", "work/", "artifacts/", "reports/")) and path.endswith(
                (".c", ".h", ".m", ".py", ".txt", ".svg")):
            unmapped.append(path)
    return ctest, sorted(set(python)), unmapped


def build() -> bool:
    t = time.monotonic()
    cfg = run(["cmake", "--preset", "mac-debug"])
    out = run(["cmake", "--build", "--preset", "mac-debug"])
    ok = cfg.returncode == 0 and out.returncode == 0
    print(f"{'ok  ' if ok else 'FAIL'} build            {time.monotonic() - t:5.1f}s")
    if not ok:
        errors = [l for l in (cfg.stderr + cfg.stdout + out.stdout + out.stderr).splitlines()
                  if re.search(r"error|Error|FAILED", l)]
        print("\n".join(errors[:25]))
    return ok


def run_ctest(pattern: str | None) -> bool:
    t = time.monotonic()
    cmd = ["ctest", "--test-dir", "build/mac-debug", "-j8", "--timeout", "120", "--output-on-failure"]
    if pattern and pattern != ".":
        cmd += ["-R", pattern]
    out = run(cmd)
    text = out.stdout
    m = re.search(r"(\d+)% tests passed, (\d+) tests failed out of (\d+)", text)
    summary = m.group(0) if m else "no tests matched"
    ok = out.returncode == 0
    print(f"{'ok  ' if ok else 'FAIL'} ctest -R {pattern or 'all':<14.14} {time.monotonic() - t:5.1f}s  {summary}")
    if not ok:
        failed = re.findall(r"\d+/\d+ Test\s+#\d+: (\S+) \.+\*\*\*Failed|\d+ - (\S+) \((?:Failed|Timeout)", text)
        names = sorted({a or b for a, b in failed})
        print("  failed:", ", ".join(names) or "(see output)")
        first = names[0] if names else None
        if first:
            detail = run(["ctest", "--test-dir", "build/mac-debug", "-R", f"^{first}$", "--output-on-failure"])
            print("  --- first failure: " + first)
            print("\n".join("  " + l for l in detail.stdout.splitlines()[-30:]))
    return ok


def expand(modules: list[str]) -> list[str]:
    """Packages (directories) become their test_*.py modules; names stay dotted."""
    out: list[str] = []
    for name in modules:
        path = ROOT / name.replace(".", "/")
        if path.is_dir():
            out.extend(sorted(".".join(p.relative_to(ROOT).with_suffix("").parts) for p in path.rglob("test_*.py")))
        else:
            out.append(name)
    return sorted(set(out))


def run_python(modules: list[str], pattern: str | None) -> bool:
    if not modules and not pattern:
        return True
    modules = expand(modules)
    if not FULL[0]:
        skipped = [m for m in modules if m.startswith(SLOW_PY)]
        modules = [m for m in modules if not m.startswith(SLOW_PY)]
        if skipped:
            print(f"skip slow python: {', '.join(m.replace('tests.', '') for m in skipped)}  (use --full)")
        if not modules and not pattern:
            return True
    t = time.monotonic()
    cmd = [sys.executable, "-m", "unittest"] + (["-v"] + modules if modules else ["discover", "-v", "-s", "tests", "-t", "."])
    if pattern:
        cmd += ["-k", pattern]
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    out = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True, env=env)
    text = out.stderr
    m = re.search(r"Ran (\d+) tests? in ([\d.]+)s", text)
    ok = out.returncode == 0 or (pattern is not None and not modules and "Ran 0 tests" in text)
    label = ",".join(m.split(".")[-1] if False else m for m in [x.replace("tests.", "") for x in modules])[:28] or "python"
    print(f"{'ok  ' if ok else 'FAIL'} python {label:<20.20} {time.monotonic() - t:5.1f}s  {m.group(0) if m else text[-120:]}")
    if not ok:
        blocks = re.split(r"\n={20,}\n", text)
        for block in blocks[1:2]:
            print("  --- first failure:")
            print("\n".join("  " + l for l in block.splitlines()[:40]))
    return ok


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("-k", dest="pattern")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--base", default="main")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--full", action="store_true", help="also run the slow Python modules (librsvg comparisons, corpus)")
    args = ap.parse_args()
    FULL[0] = args.full
    started = time.monotonic()

    if args.pattern:
        ctest, python = [args.pattern], []
        unmapped: list[str] = []
    elif args.all:
        ctest, unmapped = ["."], []
        python = ["tests.test_document_model", "tests.test_source_save", "tests.test_golden_diff", "tests.test_dev",
                  "tests.test_viewer_frame", "tests.test_viewer_navigation", "tests.test_xml_tokenizer",
                  "tests.test_svg_index", "tests.test_jobs", "tests.test_spatial_cache", "tests.test_ui_chrome_golden",
                  "tests.test_platform_media", "tests.svg", "tests.mcp"]
    else:
        files = changed_files(args.base)
        ctest, python, unmapped = select(files)
        print(f"changed files: {len(files)}" + (f"  (unmapped sources: {', '.join(unmapped[:5])})" if unmapped else ""))
    pattern = "|".join(sorted(set(ctest))) if ctest and "." not in ctest else None
    if args.list:
        print("ctest -R", pattern or "(all)")
        print("python", " ".join(python) or "(none)")
        return 0
    if not args.no_build and not build():
        return 1
    ok = run_ctest(pattern)
    ok = run_python(python, args.pattern) and ok
    print(f"{'PASS' if ok else 'FAIL'}  total {time.monotonic() - started:.1f}s"
          + ("" if args.full or args.pattern else "   (slow Python skipped; --full or ./dev t -k <name>)"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
