#!/usr/bin/env python3
"""Generate a CycloneDX SBOM for the built application.

The project has no third-party dependencies, so the inventory is the application
component plus the macOS system frameworks and libraries the binary links
against, taken from `otool -L` rather than hand-written. Output is deterministic:
components are sorted by name and no timestamps are included.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def workspace_version() -> str:
    """Workspace package version from the root Cargo.toml."""
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    section = re.search(r"^\[workspace\.package\]\s*$(.*?)(?=^\[|\Z)", text, re.MULTILINE | re.DOTALL)
    match = re.search(r'^version\s*=\s*"([^"]+)"', section[1] if section else "", re.MULTILINE)
    if match is None:
        raise SystemExit("no [workspace.package] version in Cargo.toml")
    return match[1]


VERSION = workspace_version()
SYSTEM_PREFIXES = ("/usr/lib/", "/System/Library/")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def linked_libraries(binary: Path) -> list[str]:
    result = subprocess.run(["otool", "-L", str(binary)], capture_output=True, text=True)
    if result.returncode != 0:
        return []
    dependencies = []
    for line in result.stdout.splitlines()[1:]:
        name = line.strip().split(" (")[0]
        if name:
            dependencies.append(name)
    return dependencies


def purl_for(path: str) -> str:
    name = Path(path).name
    return f"pkg:generic/{name}?os=macos"


def build_sbom(binary: Path, source_root: Path) -> dict:
    components = [
        {
            "type": "application",
            "bom-ref": f"pkg:generic/vexel@{VERSION}",
            "name": "vexel",
            "version": VERSION,
            "licenses": [{"license": {"id": "MPL-2.0"}}],
            "hashes": [{"alg": "SHA-256", "content": sha256(binary)}],
        }
    ]
    for library in sorted(set(linked_libraries(binary))):
        entry = {
            "type": "library",
            "bom-ref": purl_for(library),
            "name": Path(library).name,
            "version": "system",
            "scope": "required",
        }
        entry["description"] = (
            "macOS system library" if library.startswith(SYSTEM_PREFIXES)
            else f"Linked library at {library}")
        components.append(entry)
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": "vexel",
                "version": VERSION,
                "licenses": [{"license": {"id": "MPL-2.0"}}],
            },
            "tools": [{"name": "scripts/make_sbom.py", "version": VERSION}],
        },
        "components": components,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path,
                        default=ROOT / "build" / "release" / "Vexel.app" / "Contents" / "MacOS" / "vexel")
    parser.add_argument("--out", type=Path,
                        default=ROOT / "build" / "release" / "sbom.cyclonedx.json")
    arguments = parser.parse_args()
    binary = arguments.binary
    if not binary.is_file():
        raise SystemExit(f"no built binary at {binary}; run `cargo run -p vexel-dev -- bundle` first")
    sbom = build_sbom(binary, ROOT)
    arguments.out.parent.mkdir(parents=True, exist_ok=True)
    arguments.out.write_text(json.dumps(sbom, indent=2) + "\n")
    print(f"wrote {arguments.out} with {len(sbom['components'])} components")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
