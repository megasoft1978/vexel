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

    def test_final_gate_runs_every_fuzzer_with_a_fixed_seed(self):
        block = self._final_block()
        for step in ("shape-fuzz", "fuzz-xml", "animation-fuzz", "mcp-fuzz"):
            with self.subTest(step=step):
                match = re.search(rf'"\$repo_root/dev" {step}([^\n]*)', block)
                self.assertIsNotNone(match, f"final gate must run {step}")
                args = match.group(1)
                if step == "fuzz-xml":
                    self.assertRegex(args, r"(?:^|\s)--seed(?:\s|$)")
                elif step == "mcp-fuzz":
                    continue
                else:
                    self.assertRegex(args, r"\b\d{6,}\b")

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


if __name__ == "__main__":
    unittest.main()
