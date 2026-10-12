import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class DevFinalCommandTests(unittest.TestCase):
    def _final_block(self):
        script = (ROOT / "dev").read_text()
        return script.split("            final)\n", 1)[1].split("                ;;\n", 1)[0]

    def test_shape_fuzz_step_supplies_its_required_seed(self):
        match = re.search(r'"\$repo_root/dev" shape-fuzz([^\n]*)', self._final_block())

        self.assertIsNotNone(match, "final gate must run shape-fuzz")
        self.assertRegex(match.group(1), r"(?:^|\s)--seed(?:\s|$)")

    #: Each fuzzer step and the exact bound and seed the final gate must pass.
    EXPECTED_FUZZ_ARGS = {
        "shape-fuzz": r"--seed 20261010",
        "fuzz-xml": r"--duration 30 --seed 20261010",
        "animation-fuzz": r"200000 20261010",
        "mcp-fuzz": r"200000 20261010",
    }

    def test_final_gate_runs_every_fuzzer_with_its_bound_and_seed(self):
        block = self._final_block()
        for step, expected in self.EXPECTED_FUZZ_ARGS.items():
            with self.subTest(step=step):
                match = re.search(rf'"\$repo_root/dev" {step}([^\n]*)', block)
                self.assertIsNotNone(match, f"final gate must run {step}")
                self.assertIn(expected, match.group(1),
                              f"{step} must be bounded as {expected!r}")

    def test_animation_fuzz_command_defaults_stay_bounded(self):
        script = (ROOT / "dev").read_text()
        match = re.search(r"\n    animation-fuzz\)\n(.*?)\n        ;;", script, re.S)
        self.assertIsNotNone(match, "./dev must expose animation-fuzz")
        body = match.group(1)
        self.assertIn("cases=200000", body)
        self.assertIn("seed=20261012", body)
        self.assertIn("--target animation_fuzz", body)

    def test_no_dev_command_defaults_to_a_tracked_artifact_report(self):
        script = (ROOT / "dev").read_text()
        self.assertNotIn("report=artifacts/", script)
        self.assertNotIn('"artifacts/WO-M4-04/fuzz-report.json"', script)
        shape = (ROOT / "tests/fuzz/shape/shape_fuzz.py").read_text()
        self.assertIn('.cache/qa/shape-fuzz.json', shape)
        self.assertNotIn("artifacts/WO-M4-04/fuzz-report.json", shape)

    def test_xml_failure_evidence_follows_the_report_path(self):
        campaign = (ROOT / "scripts/xml_fuzz_campaign.py").read_text()
        self.assertIn("def failure_directory(", campaign)
        self.assertNotIn('confined_output_path("artifacts/WO-M2-03/failures', campaign)
        # The confinement policy still allows the ignored .cache tree and the
        # pinned packet directory, and nothing else.
        self.assertIn('".cache", "artifacts/WO-M2-03"', campaign)


if __name__ == "__main__":
    unittest.main()
