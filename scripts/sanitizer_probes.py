#!/usr/bin/env python3
"""Compile and confirm injected assertion and heap-overflow sanitizer probes."""

from __future__ import annotations

import argparse
import gzip
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCES = {
    "assertion": '#include <assert.h>\nint main(void) { assert(!"injected assertion probe"); return 0; }\n',
    "heap_overflow": '#include <stdlib.h>\nint main(void) { volatile char *p = (char *)malloc(1); p[1] = 7; free((void *)p); return 0; }\n',
}
DIAGNOSTICS = {"assertion": re.compile(r"Assertion|assertion failed", re.IGNORECASE),
               "heap_overflow": re.compile(r"AddressSanitizer|heap-buffer-overflow")}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("asan-ubsan", "ubsan"), required=True)
    args = parser.parse_args()
    evidence = ROOT / "artifacts/WO-BOOT-05/probes"
    evidence.mkdir(parents=True, exist_ok=True)
    cache = ROOT / ".cache/sanitizer-probes"
    cache.mkdir(parents=True, exist_ok=True)
    retained_files = [evidence / "assertion.c", evidence / "heap_overflow.c",
                      evidence / "assertion.stdout.txt.gz", evidence / "assertion.stderr.txt.gz",
                      evidence / "heap_overflow.stdout.txt.gz", evidence / "heap_overflow.stderr.txt.gz",
                      evidence / "probes.json"]
    retain_evidence = not all(path.is_file() for path in retained_files)
    flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"] if args.profile == "asan-ubsan" else ["-fsanitize=undefined", "-fno-omit-frame-pointer"]
    compiler = subprocess.run(["xcrun", "--find", "clang"], capture_output=True, text=True)
    if compiler.returncode:
        raise SystemExit("Xcode Apple Clang unavailable: " + compiler.stderr)
    clang = compiler.stdout.strip()
    sdk = subprocess.run(["xcrun", "--sdk", "macosx", "--show-sdk-path"], capture_output=True, text=True)
    if sdk.returncode or not sdk.stdout.strip():
        raise SystemExit("macOS SDK unavailable: " + sdk.stderr)
    results: dict[str, object] = {"profile": args.profile, "compiler": clang,
                                 "sdk": sdk.stdout.strip(), "probes": {}}
    all_detected = True

    def retain_trace(name: str, stream: str, content: str) -> str:
        artifact = f"{name}.{stream}.txt.gz"
        compressed = gzip.compress(content.encode("utf-8"), mtime=0)
        (cache / artifact).write_bytes(compressed)
        if retain_evidence:
            (evidence / artifact).write_bytes(compressed)
        return artifact

    for name, source in SOURCES.items():
        src = cache / f"{name}.c"
        binary = cache / name
        src.write_text(source, encoding="utf-8")
        retained_source = evidence / f"{name}.c"
        if retained_source.exists() and retained_source.read_text(encoding="utf-8") != source:
            raise SystemExit(f"retained {name} source does not match the probe fixture")
        if retain_evidence:
            retained_source.write_text(source, encoding="utf-8")
        built = subprocess.run([clang, "-isysroot", sdk.stdout.strip(), *flags,
                                str(src), "-o", str(binary)], capture_output=True, text=True)
        item: dict[str, object] = {"compile_exit_code": built.returncode}
        if built.returncode == 0:
            run = subprocess.run([str(binary)], capture_output=True, text=True)
            trace = run.stdout + run.stderr
            item["stdout_artifact"] = retain_trace(name, "stdout", run.stdout)
            item["stderr_artifact"] = retain_trace(name, "stderr", run.stderr)
            detected = run.returncode != 0 and DIAGNOSTICS[name].search(trace) is not None
            item.update({"exit_code": run.returncode, "detected": detected,
                         "diagnostic": DIAGNOSTICS[name].search(trace).group(0) if DIAGNOSTICS[name].search(trace) else None})
        else:
            detected = False
            item.update({"detected": False, "compiler_stderr": built.stderr})
            item["stderr_artifact"] = retain_trace(name, "stderr", built.stderr)
        results["probes"][name] = item
        all_detected = all_detected and detected
    results["completed_at"] = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    results["all_detected"] = all_detected
    results["retained_evidence_reused"] = not retain_evidence
    latest = cache / "probes.latest.json"
    latest.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    if retain_evidence:
        (evidence / "probes.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(results, indent=2))
    return 0 if all_detected else 1


if __name__ == "__main__":
    raise SystemExit(main())
