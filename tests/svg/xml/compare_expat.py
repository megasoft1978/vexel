#!/usr/bin/env python3
"""Test-only differential check for pinned corpus fixtures."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import xml.parsers.expat as expat

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--expect-expat", required=True)
    parser.add_argument("--binary", default="build/mac-debug/bin/xml_tokenizer_cli")
    parser.add_argument("--output", default="artifacts/WO-M2-02/expat-comparison.json")
    args = parser.parse_args()
    actual = expat.EXPAT_VERSION
    if actual != args.expect_expat:
        print(f"expected {args.expect_expat}, found {actual}", file=sys.stderr)
        return 2
    manifest = json.loads((ROOT / "bench/corpus/manifest.json").read_text())
    records = []
    for fixture in manifest["fixtures"]:
        path = ROOT / fixture["path"]
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != fixture["sha256"]:
            raise RuntimeError(f"fixture hash mismatch: {fixture['fixture_id']}")
        elements, attributes = 0, 0
        p = expat.ParserCreate()
        def start(_name, attrs):
            nonlocal elements, attributes
            elements += 1
            attributes += len(attrs)
        p.StartElementHandler = start
        try:
            p.Parse(data, True)
            expat_ok = True
        except expat.ExpatError:
            expat_ok = False
        if not expat_ok:
            raise RuntimeError(f"pinned corpus fixture is not well-formed XML: {fixture['fixture_id']}")
        run = subprocess.run([str(ROOT / args.binary), str(path)], check=True, capture_output=True, text=True)
        result = json.loads(run.stdout)
        if not result["ok"] or result["elements"] != elements or result["attributes"] != attributes:
            raise RuntimeError(f"acceptance/count mismatch for {fixture['fixture_id']}: {result}, Expat={(elements, attributes)}")
        records.append({"fixture_id": fixture["fixture_id"], "sha256": digest,
                        "expat_accepted": expat_ok, "tokenizer_accepted": result["ok"],
                        "elements": elements, "attributes": attributes})
    report = {"python": sys.version.split()[0], "expat": actual, "fixtures_checked": len(records),
              "unsupported_dtd_cases": [], "results": records}
    out = ROOT / args.output
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2) + "\n")
    print(f"compared {len(records)} pinned fixtures using Python {report['python']} / {actual}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
