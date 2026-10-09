"""Tests for local dev command dispatch and macOS CI policy."""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

from scripts.check_ci import PolicyError, check_policy

ROOT = Path(__file__).resolve().parent.parent


class DevCommandTests(unittest.TestCase):
    def test_help_lists_supported_commands(self) -> None:
        result = subprocess.run([str(ROOT / "dev"), "help"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0)
        self.assertIn("build|test-fast|test|verify|help", result.stdout)

    def test_unknown_command_fails_with_usage(self) -> None:
        result = subprocess.run([str(ROOT / "dev"), "not-a-command"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("usage:", result.stderr)

    def test_real_workflow_passes_policy(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        check_policy(workflow)

    def test_linux_runner_is_rejected(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        with self.assertRaisesRegex(PolicyError, "macos-15"):
            check_policy(workflow.replace("runs-on: macos-15", "runs-on: ubuntu-latest"))

    def test_unpinned_action_is_rejected(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        with self.assertRaisesRegex(PolicyError, "full 40-character SHA"):
            check_policy(workflow.replace("@3d3c42e5aac5ba805825da76410c181273ba90b1", "@v7"))

    def test_secret_reference_is_rejected(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        with self.assertRaisesRegex(PolicyError, "secrets"):
            check_policy(workflow + "\n# ${{ secrets.TOKEN }}\n")


if __name__ == "__main__":
    unittest.main()
