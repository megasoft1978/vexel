"""Focused tests for the M3 owner-packet generator's frozen/living split."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "reports" / "M3" / "validate_packet.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("m3_validate_packet", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


packet = _load_module()


class PacketInputTests(unittest.TestCase):
    def test_current_tree_passes_the_frozen_check(self) -> None:
        sources = packet.validate_inputs()
        self.assertEqual(len(sources), 41)
        packet.validate_schema(packet.build_packet())

    def test_a_changed_frozen_input_fails_the_check(self) -> None:
        original = dict(packet.PINNED)
        try:
            target = sorted(original)[0]
            packet.PINNED[target] = "0" * 64
            with self.assertRaises(ValueError):
                packet.validate_inputs()
        finally:
            packet.PINNED.clear()
            packet.PINNED.update(original)

    def test_a_changed_living_input_does_not_fail_the_check(self) -> None:
        living = tuple(packet.LIVING_PATHS)
        self.assertTrue(living, "the packet must record at least one living input")
        original_sha = packet.sha
        try:
            def changed(path: str) -> str:
                if path in living:
                    return "f" * 64
                return original_sha(path)

            packet.sha = changed
            sources = packet.validate_inputs()
        finally:
            packet.sha = original_sha
        for item in sources:
            if item["path"] in living:
                self.assertIs(item.get("living"), True)
                self.assertNotIn("sha256", item)

    def test_packet_records_the_living_input_without_a_hash(self) -> None:
        sources = packet.source_list()
        living = [item for item in sources if item.get("living") is True]
        self.assertEqual([item["path"] for item in living], list(packet.LIVING_PATHS))
        for item in living:
            self.assertNotIn("sha256", item)
        frozen = [item for item in sources if "sha256" in item]
        self.assertTrue(all(set(item) == {"path", "sha256"} for item in frozen))
        self.assertNotIn("docs/PERF.md", [item["path"] for item in frozen])

    def test_committed_packet_matches_the_generator(self) -> None:
        # --check semantics, without invoking the CLI.
        import json

        expected = json.dumps(packet.build_packet(), indent=2, sort_keys=True,
                              ensure_ascii=False).encode() + b"\n"
        self.assertEqual((ROOT / "reports/M3/evidence.json").read_bytes(), expected)


if __name__ == "__main__":
    unittest.main()
