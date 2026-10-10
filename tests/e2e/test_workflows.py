"""Policy and event checks for lane CI workflows."""

from __future__ import annotations

import unittest
from pathlib import Path

from scripts.check_ci import check_policy

ROOT = Path(__file__).resolve().parents[2]


class WorkflowTests(unittest.TestCase):
    def test_nightly_workflow_keeps_repo_policy_and_scheduled_manual_triggers(self) -> None:
        path = ROOT / ".github/workflows/nightly.yml"
        text = path.read_text(encoding="utf-8")
        check_policy(text)
        self.assertIn("schedule:", text)
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("./dev e2e", text)
        self.assertIn("./dev asan", text)
        self.assertIn("./dev ubsan", text)
        self.assertIn("./dev soak", text)
        self.assertIn("actions/upload-artifact@", text)

    def test_release_workflow_pins_actions_and_builds_dmg_for_tags(self) -> None:
        path = ROOT / ".github/workflows/release.yml"
        text = path.read_text(encoding="utf-8")
        check_policy(text)
        self.assertIn("tags:", text)
        self.assertIn("'v*'", text)
        self.assertIn("workflow_dispatch:", text)
        self.assertIn("./dev dmg", text)
        self.assertIn("path: build/Vexel.dmg", text)


if __name__ == "__main__":
    unittest.main()
