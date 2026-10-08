#!/usr/bin/env python3
"""Verify and explicitly fetch the pinned, legally documented SVG corpus."""

from __future__ import annotations

import argparse
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
import tempfile
import urllib.error
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "bench" / "corpus"
MANIFEST = CORPUS / "manifest.json"
MANIFEST_DIGEST = CORPUS / "manifest.sha256"
WPT_REVISION = "da1355b3fc629f97f8194884ab113da1ba3c7990"
WPT_PREFIX = f"https://raw.githubusercontent.com/web-platform-tests/wpt/{WPT_REVISION}/svg/"
WPT_LICENSE_URL = f"https://raw.githubusercontent.com/web-platform-tests/wpt/{WPT_REVISION}/LICENSE.md"
WPT_LICENSE_SHA256 = "5fac07febb0e2a97fb0d7b0def149ec08b642e1ba4b9c345283ab1cbd2af6570"
PROJECT_LICENSE_URL = "https://www.mozilla.org/MPL/2.0/"
MAX_FILE_SIZE = 2 * 1024 * 1024
TIMEOUT_SECONDS = 15
EXPECTED_COUNT = 24
REQUIRED_FEATURES = {
    "basic-shapes", "path", "metadata", "namespaces", "style", "text",
    "embedded-image", "stress",
}
REQUIRED_FIELDS = {
    "fixture_id", "path", "sha256", "source_url", "source_origin",
    "source_revision", "source_sha256", "license_id", "license_url",
    "license_sha256", "features", "created_utc", "modification_note",
}
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


class CorpusError(Exception):
    """A manifest, provenance, network, or content validation error."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_path(value: object) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value:
        raise CorpusError("path must be a non-empty POSIX relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise CorpusError(f"unsafe relative path: {value!r}")
    if not value.startswith("bench/corpus/fixtures/") or path.suffix.lower() != ".svg":
        raise CorpusError(f"fixture path is outside corpus SVG directory: {value!r}")
    return path


def validate_wpt_url(url: object) -> None:
    if not isinstance(url, str) or not url.startswith(WPT_PREFIX):
        raise CorpusError("source URL is not under the pinned HTTPS WPT SVG path")
    tail = url[len(WPT_PREFIX):]
    if not tail or "?" in tail or "#" in tail or "%" in tail:
        raise CorpusError("source URL contains an invalid path or query")
    safe = PurePosixPath(tail)
    if safe.is_absolute() or any(part in {"", ".", ".."} for part in safe.parts):
        raise CorpusError("source URL has an unsafe relative path")


def load_manifest() -> tuple[dict, list[dict]]:
    try:
        raw = MANIFEST.read_bytes()
        expected = MANIFEST_DIGEST.read_text(encoding="ascii").strip()
    except OSError as exc:
        raise CorpusError(f"cannot read manifest files: {exc}") from exc
    if not SHA256_RE.fullmatch(expected) or sha256(raw) != expected:
        raise CorpusError("manifest SHA-256 does not match manifest.sha256")
    try:
        manifest = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CorpusError(f"manifest is not valid UTF-8 JSON: {exc}") from exc
    if not isinstance(manifest, dict) or manifest.get("schema_version") != 1:
        raise CorpusError("manifest schema_version must be 1")
    records = manifest.get("fixtures")
    if not isinstance(records, list) or len(records) != EXPECTED_COUNT:
        raise CorpusError(f"manifest must contain exactly {EXPECTED_COUNT} fixtures")
    if manifest.get("fixture_count") != len(records):
        raise CorpusError("fixture_count does not match fixtures array")
    ids: set[str] = set()
    paths: set[str] = set()
    covered: set[str] = set()
    for rec in records:
        if not isinstance(rec, dict) or set(rec) != REQUIRED_FIELDS:
            raise CorpusError("fixture record has missing or unexpected fields")
        fixture_id = rec["fixture_id"]
        if not isinstance(fixture_id, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]*", fixture_id):
            raise CorpusError("fixture_id must be a stable lowercase slug")
        if fixture_id in ids:
            raise CorpusError(f"duplicate fixture_id: {fixture_id}")
        ids.add(fixture_id)
        safe_path(rec["path"])
        if rec["path"] in paths:
            raise CorpusError(f"duplicate fixture path: {rec['path']}")
        paths.add(rec["path"])
        for key in ("sha256", "source_sha256", "license_sha256"):
            if not isinstance(rec[key], str) or not SHA256_RE.fullmatch(rec[key]):
                raise CorpusError(f"{key} must be a lowercase SHA-256")
        if rec["sha256"] != rec["source_sha256"]:
            raise CorpusError(f"project-authored source hash differs for {fixture_id}")
        if rec["source_origin"] == "wpt":
            validate_wpt_url(rec["source_url"])
            if rec["license_id"] != "BSD-3-Clause" or rec["license_url"] != WPT_LICENSE_URL:
                raise CorpusError(f"WPT license metadata mismatch for {fixture_id}")
        elif not isinstance(rec["source_url"], str) or not rec["source_url"].startswith("project-authored:"):
            raise CorpusError(f"invalid project-authored source URL for {fixture_id}")
        elif rec["source_origin"] != "project-authored":
            raise CorpusError(f"unapproved source origin for {fixture_id}")
        if rec["source_revision"] != "project-authored" and rec["source_revision"] != WPT_REVISION:
            raise CorpusError(f"unapproved source revision for {fixture_id}")
        if not isinstance(rec["license_id"], str) or rec["license_id"] not in {"MPL-2.0", "BSD-3-Clause"}:
            raise CorpusError(f"unapproved license identifier for {fixture_id}")
        if not isinstance(rec["license_url"], str) or not rec["license_url"].startswith("https://"):
            raise CorpusError(f"license URL must be HTTPS for {fixture_id}")
        if rec["license_id"] == "MPL-2.0" and rec["license_url"] != PROJECT_LICENSE_URL:
            raise CorpusError(f"project license URL mismatch for {fixture_id}")
        if rec["license_id"] == "BSD-3-Clause" and rec["license_sha256"] != WPT_LICENSE_SHA256:
            raise CorpusError(f"WPT license hash mismatch for {fixture_id}")
        if not isinstance(rec["features"], list) or not rec["features"] or any(not isinstance(x, str) for x in rec["features"]):
            raise CorpusError(f"features must be a non-empty string array for {fixture_id}")
        covered.update(rec["features"])
        if not isinstance(rec["created_utc"], str) or not rec["created_utc"].endswith("Z"):
            raise CorpusError(f"created_utc must be UTC for {fixture_id}")
        try:
            datetime.fromisoformat(rec["created_utc"].replace("Z", "+00:00"))
        except ValueError as exc:
            raise CorpusError(f"created_utc is not an ISO timestamp for {fixture_id}") from exc
        if not isinstance(rec["modification_note"], str) or not rec["modification_note"]:
            raise CorpusError(f"modification_note is required for {fixture_id}")
    if covered < REQUIRED_FEATURES:
        raise CorpusError(f"feature coverage is incomplete: {sorted(REQUIRED_FEATURES - covered)}")
    if [r["fixture_id"] for r in records] != sorted(ids):
        raise CorpusError("fixture records must be sorted by fixture_id")
    licenses = manifest.get("licenses")
    if not isinstance(licenses, dict) or licenses.get("wpt_sha256") != WPT_LICENSE_SHA256:
        raise CorpusError("manifest WPT license metadata is missing or mismatched")
    if licenses.get("wpt_revision") != WPT_REVISION or licenses.get("wpt_license_url") != WPT_LICENSE_URL:
        raise CorpusError("manifest WPT source metadata is not pinned")
    if licenses.get("project_license_id") != "MPL-2.0" or licenses.get("project_license_url") != PROJECT_LICENSE_URL:
        raise CorpusError("manifest project license metadata is missing or mismatched")
    return manifest, records


def local_path(record: dict) -> Path:
    relative = safe_path(record["path"])
    target = ROOT.joinpath(*relative.parts)
    if not target.resolve().is_relative_to(ROOT.resolve()):
        raise CorpusError(f"fixture path escapes repository: {record['path']}")
    return target


def verify_files(records: list[dict]) -> list[dict]:
    results = []
    for rec in records:
        path = local_path(rec)
        try:
            data = path.read_bytes()
        except OSError as exc:
            raise CorpusError(f"missing fixture {rec['fixture_id']}: {exc}") from exc
        actual = sha256(data)
        if actual != rec["sha256"]:
            raise CorpusError(f"fixture hash mismatch for {rec['fixture_id']}")
        results.append({"fixture_id": rec["fixture_id"], "path": rec["path"], "sha256": actual, "status": "verified"})
    return results


def verify_license_files(manifest: dict, records: list[dict]) -> None:
    license_dir = CORPUS / "licenses"
    try:
        wpt_hash = sha256((license_dir / "WPT-LICENSE.md").read_bytes())
        mpl_hash = sha256((license_dir / "MPL-2.0.txt").read_bytes())
    except OSError as exc:
        raise CorpusError(f"cannot read corpus license text: {exc}") from exc
    metadata = manifest["licenses"]
    if wpt_hash != WPT_LICENSE_SHA256 or metadata["wpt_sha256"] != wpt_hash:
        raise CorpusError("pinned WPT license notice hash mismatch")
    if metadata.get("project_license_sha256") != mpl_hash:
        raise CorpusError("project MPL-2.0 license text hash mismatch")
    for rec in records:
        if rec["license_id"] == "MPL-2.0" and rec["license_sha256"] != mpl_hash:
            raise CorpusError(f"MPL license hash mismatch for {rec['fixture_id']}")


class RejectRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise CorpusError("redirect rejected")


def download(record: dict, opener=None) -> bytes:
    validate_wpt_url(record["source_url"])
    request = urllib.request.Request(record["source_url"], headers={"User-Agent": "vexel-corpus-fetch/1"})
    client = opener or urllib.request.build_opener(RejectRedirect())
    try:
        response = client.open(request, timeout=TIMEOUT_SECONDS)
    except CorpusError:
        raise
    except Exception as exc:
        raise CorpusError(f"download failed for {record['fixture_id']}: {exc}") from exc
    with response:
        final_url = response.geturl()
        if final_url != record["source_url"]:
            raise CorpusError("redirect or final URL mismatch rejected")
        length = response.headers.get("Content-Length")
        if length is not None and int(length) > MAX_FILE_SIZE:
            raise CorpusError("fixture exceeds 2 MiB limit")
        chunks = []
        total = 0
        digest = hashlib.sha256()
        try:
            while True:
                chunk = response.read(65536)
                if not chunk:
                    break
                total += len(chunk)
                if total > MAX_FILE_SIZE:
                    raise CorpusError("fixture exceeds 2 MiB limit")
                digest.update(chunk)
                chunks.append(chunk)
        except CorpusError:
            raise
        except OSError as exc:
            raise CorpusError(f"partial download failed for {record['fixture_id']}: {exc}") from exc
        if digest.hexdigest() != record["sha256"]:
            raise CorpusError(f"download hash mismatch for {record['fixture_id']}")
        return b"".join(chunks)


def atomic_install(target: Path, data: bytes) -> None:
    target.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{target.name}.", suffix=".partial", dir=target.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp_name, target)
    except Exception:
        try:
            os.unlink(temp_name)
        except FileNotFoundError:
            pass
        raise


def fetch(records: list[dict]) -> list[dict]:
    results = []
    for rec in records:
        target = local_path(rec)
        if target.exists():
            actual = sha256(target.read_bytes())
            if actual == rec["sha256"]:
                results.append({"fixture_id": rec["fixture_id"], "path": rec["path"], "sha256": actual, "status": "verified-existing"})
                continue
        if rec["source_origin"] == "project-authored":
            raise CorpusError(f"project-authored fixture missing or corrupt: {rec['fixture_id']}")
        data = download(rec)
        atomic_install(target, data)
        results.append({"fixture_id": rec["fixture_id"], "path": rec["path"], "sha256": rec["sha256"], "status": "fetched"})
    return results


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--fetch", action="store_true")
    mode.add_argument("--verify-only", action="store_true")
    mode.add_argument("--list", action="store_true")
    parser.add_argument("--feature")
    args = parser.parse_args(argv)
    try:
        if args.feature and not args.list:
            raise CorpusError("--feature is only valid with --list")
        if args.list and not args.feature:
            raise CorpusError("--list requires --feature TAG")
        manifest, records = load_manifest()
        verify_license_files(manifest, records)
        if args.list:
            selected = [r["fixture_id"] for r in records if args.feature in r["features"]]
            print(json.dumps({"feature": args.feature, "fixture_ids": selected}, sort_keys=True, separators=(",", ":")))
            return 0
        if args.fetch:
            results = fetch(records)
            status = "fetch"
        else:
            results = verify_files(records)
            status = "verify-only"
        print(json.dumps({"manifest_sha256": sha256(MANIFEST.read_bytes()), "fixture_count": len(records), "mode": status, "results": results}, sort_keys=True, separators=(",", ":")))
        return 0
    except (CorpusError, OSError, ValueError) as exc:
        print(json.dumps({"error": str(exc), "status": "error"}, sort_keys=True, separators=(",", ":")), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
