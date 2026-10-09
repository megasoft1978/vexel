"""Offline tests for the pinned SVG corpus manifest and fetcher."""

from __future__ import annotations

import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import fetch_corpus as corpus


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class Response(io.BytesIO):
    def __init__(self, data: bytes, url: str):
        super().__init__(data)
        self.headers = {"Content-Length": str(len(data))}
        self.url = url

    def geturl(self):
        return self.url

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.close()


class MockOpener:
    def __init__(self, response):
        self.response = response
        self.calls = []

    def open(self, request, timeout):
        self.calls.append((request.full_url, timeout))
        return self.response


class CorpusFetcherTests(unittest.TestCase):
    def test_checked_in_manifest_schema_coverage_and_license(self):
        manifest, records = corpus.load_manifest()
        corpus.verify_license_files(manifest, records)
        self.assertEqual(len(records), 24)
        self.assertEqual([r["fixture_id"] for r in records], sorted(r["fixture_id"] for r in records))
        self.assertTrue(corpus.REQUIRED_FEATURES <= {feature for rec in records for feature in rec["features"]})
        self.assertEqual(manifest["licenses"]["wpt_sha256"], corpus.WPT_LICENSE_SHA256)
        for rec in records:
            self.assertEqual(set(rec), corpus.REQUIRED_FIELDS)
            self.assertEqual(rec["source_origin"], "project-authored")
            self.assertEqual(rec["license_id"], "MPL-2.0")

    def test_feature_filter_is_stable_and_sorted(self):
        output_a = io.StringIO()
        output_b = io.StringIO()
        with mock.patch("sys.stdout", output_a):
            self.assertEqual(corpus.main(["--list", "--feature", "path"]), 0)
        with mock.patch("sys.stdout", output_b):
            self.assertEqual(corpus.main(["--list", "--feature", "path"]), 0)
        self.assertEqual(output_a.getvalue(), output_b.getvalue())
        ids = json.loads(output_a.getvalue())["fixture_ids"]
        self.assertGreater(len(ids), 0)
        self.assertEqual(ids, sorted(ids))

    def test_verify_only_never_opens_network(self):
        output = io.StringIO()
        with mock.patch("sys.stdout", output), mock.patch.object(corpus.urllib.request, "build_opener", side_effect=AssertionError("network access")), mock.patch.object(corpus.urllib.request, "urlopen", side_effect=AssertionError("network access")):
            self.assertEqual(corpus.main(["--verify-only"]), 0)
        self.assertEqual(json.loads(output.getvalue())["mode"], "verify-only")

    def _temp_manifest(self, folder: Path, record: dict, count=1):
        manifest = {
            "schema_version": 1,
            "fixture_count": count,
            "licenses": {"wpt_sha256": corpus.WPT_LICENSE_SHA256},
            "fixtures": [record],
        }
        raw = (json.dumps(manifest, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (folder / "manifest.json").write_bytes(raw)
        (folder / "manifest.sha256").write_text(digest(raw) + "\n", encoding="ascii")

    def _wpt_record(self, data=b"<svg/>", path="bench/corpus/fixtures/mock.svg", url=None):
        return {
            "fixture_id": "mock-fixture",
            "path": path,
            "sha256": digest(data),
            "source_url": url or corpus.WPT_PREFIX + "mock.svg",
            "source_origin": "wpt",
            "source_revision": corpus.WPT_REVISION,
            "source_sha256": digest(data),
            "license_id": "BSD-3-Clause",
            "license_url": corpus.WPT_LICENSE_URL,
            "license_sha256": corpus.WPT_LICENSE_SHA256,
            "features": ["path"],
            "created_utc": "2026-10-08T00:00:00Z",
            "modification_note": "Unmodified upstream bytes.",
        }

    def test_traversal_path_and_unapproved_url_rejected(self):
        with self.assertRaises(corpus.CorpusError):
            corpus.safe_path("bench/corpus/fixtures/../../outside.svg")
        with self.assertRaises(corpus.CorpusError):
            corpus.validate_wpt_url("https://example.com/web-platform-tests/wpt/x/svg/a.svg")
        with self.assertRaises(corpus.CorpusError):
            corpus.validate_wpt_url(corpus.WPT_PREFIX + "../LICENSE.md")

    def test_redirect_rejected(self):
        record = self._wpt_record()
        opener = MockOpener(Response(b"<svg/>", "https://evil.invalid/file.svg"))
        with self.assertRaisesRegex(corpus.CorpusError, "redirect"):
            corpus.download(record, opener)

    def test_per_file_size_cap(self):
        record = self._wpt_record(data=b"x" * (corpus.MAX_FILE_SIZE + 1))
        opener = MockOpener(Response(b"x" * (corpus.MAX_FILE_SIZE + 1), record["source_url"]))
        with self.assertRaisesRegex(corpus.CorpusError, "2 MiB"):
            corpus.download(record, opener)

    def test_hash_mismatch_preserves_existing_destination(self):
        expected = b"<svg>correct</svg>"
        wrong = b"<svg>wrong</svg>"
        record = self._wpt_record(data=expected)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / record["path"]
            target.parent.mkdir(parents=True)
            target.write_bytes(b"old destination")
            opener = MockOpener(Response(wrong, record["source_url"]))
            with mock.patch.object(corpus, "ROOT", root), mock.patch.object(corpus.urllib.request, "build_opener", return_value=opener):
                with self.assertRaisesRegex(corpus.CorpusError, "hash mismatch"):
                    corpus.fetch([record])
            self.assertEqual(target.read_bytes(), b"old destination")
            self.assertEqual(list(target.parent.glob("*.partial")), [])

    def test_partial_download_failure_leaves_destination_and_no_temp_file(self):
        class InterruptedResponse(Response):
            def __init__(self, url):
                super().__init__(b"partial", url)
                self.read_count = 0

            def read(self, size=-1):
                self.read_count += 1
                if self.read_count > 1:
                    raise OSError("simulated interrupted response")
                return super().read(size)

        record = self._wpt_record(data=b"complete fixture")
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            target = root / record["path"]
            target.parent.mkdir(parents=True)
            target.write_bytes(b"known prior file")
            opener = MockOpener(InterruptedResponse(record["source_url"]))
            with mock.patch.object(corpus, "ROOT", root), mock.patch.object(corpus.urllib.request, "build_opener", return_value=opener):
                with self.assertRaisesRegex(corpus.CorpusError, "download failed"):
                    corpus.fetch([record])
            self.assertEqual(target.read_bytes(), b"known prior file")
            self.assertEqual(list(target.parent.glob("*.partial")), [])

    def test_partial_install_cleanup(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / "fixture.svg"
            with mock.patch.object(corpus.os, "replace", side_effect=OSError("simulated replace failure")):
                with self.assertRaisesRegex(OSError, "simulated"):
                    corpus.atomic_install(target, b"verified bytes")
            self.assertFalse(target.exists())
            self.assertEqual(list(Path(temp).iterdir()), [])

    def test_successful_mocked_pinned_host_fetch(self):
        data = b"<svg xmlns='http://www.w3.org/2000/svg'><path d='M0 0'/></svg>"
        record = self._wpt_record(data=data)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            opener = MockOpener(Response(data, record["source_url"]))
            with mock.patch.object(corpus, "ROOT", root), mock.patch.object(corpus.urllib.request, "build_opener", return_value=opener):
                result = corpus.fetch([record])
            target = root / record["path"]
            self.assertEqual(target.read_bytes(), data)
            self.assertEqual(result[0]["status"], "fetched")
            self.assertEqual(opener.calls, [(record["source_url"], corpus.TIMEOUT_SECONDS)])
            self.assertEqual(list(target.parent.glob("*.partial")), [])


if __name__ == "__main__":
    unittest.main()
