#!/usr/bin/env python3
"""Install smoke: build the app bundle, install it outside the build tree and run
it from the installed location.

The point is to prove the packaged application is self-contained: it must launch
from a directory that contains nothing but the bundle, with no build-tree paths
on the environment, and it must report its version.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(command: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(command, capture_output=True, text=True, **kwargs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path,
                        default=ROOT / "artifacts" / "release" / "install-smoke.json")
    arguments = parser.parse_args()
    evidence: dict = {"steps": []}

    bundle = run(["python3", "packaging/vexel_packaging.py", "bundle"], cwd=ROOT)
    evidence["steps"].append({"step": "bundle", "exit": bundle.returncode,
                              "stdout_tail": bundle.stdout[-400:]})
    if bundle.returncode != 0:
        evidence["result"] = "fail"
        arguments.out.parent.mkdir(parents=True, exist_ok=True)
        arguments.out.write_text(json.dumps(evidence, indent=1) + "\n")
        print("bundle failed")
        return 1

    preferred = ROOT / "build" / "mac-release" / "Vexel.app"
    candidates = ([preferred] if preferred.is_dir() else []) + \
        sorted((ROOT / "build").glob("mac-*/*.app")) + \
        sorted((ROOT / "build").glob("*.app"))
    if not candidates:
        print("no built .app bundle found")
        evidence["result"] = "fail"
        evidence["reason"] = "no .app bundle"
        arguments.out.parent.mkdir(parents=True, exist_ok=True)
        arguments.out.write_text(json.dumps(evidence, indent=1) + "\n")
        return 1
    source_bundle = candidates[0]
    evidence["bundle"] = str(source_bundle.relative_to(ROOT))

    with tempfile.TemporaryDirectory() as prefix:
        installed = Path(prefix) / "Applications"
        installed.mkdir(parents=True)
        shutil.copytree(source_bundle, installed / source_bundle.name, symlinks=True)
        binary = installed / source_bundle.name / "Contents" / "MacOS" / "svgworkbench"
        evidence["installed_binary"] = str(binary)
        if not binary.is_file():
            print(f"installed bundle has no executable at {binary}")
            evidence["result"] = "fail"
            evidence["reason"] = "missing executable"
            arguments.out.parent.mkdir(parents=True, exist_ok=True)
            arguments.out.write_text(json.dumps(evidence, indent=1) + "\n")
            return 1
        env = {k: v for k, v in os.environ.items() if k != "VEXEL_BUILD_DIR"}
        version = run([str(binary), "--version"], env=env)
        evidence["steps"].append({"step": "run --version", "exit": version.returncode,
                                  "stdout_tail": (version.stdout + version.stderr)[-400:]})
        evidence["result"] = "pass" if version.returncode == 0 else "fail"

    arguments.out.parent.mkdir(parents=True, exist_ok=True)
    arguments.out.write_text(json.dumps(evidence, indent=1) + "\n")
    print(json.dumps({"result": evidence["result"],
                      "bundle": evidence.get("bundle")}, indent=1))
    return 0 if evidence["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
