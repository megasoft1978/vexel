"""Policy and safety tests for the bounded tokenizer fuzz campaign."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import check_fuzz_workflow
import xml_fuzz_campaign as campaign


class TestSeedReplay(unittest.TestCase):
    def test_known_synthetic_canary_seed_is_stable(self) -> None:
        seed = (ROOT / "tests/svg/xml/fuzz/seeds/synthetic-canary.seed").read_bytes()
        document = json.loads((ROOT / "tests/svg/xml/fuzz/regressions.json").read_text())
        entry = document["synthetic_canary"]
        self.assertIn(b"synthetic crash canary", seed)
        self.assertEqual(hashlib.sha256(seed).hexdigest(), entry["input_sha256"])
        self.assertIn("synthetic", entry["label"])

    def test_seed_iteration_replay_is_deterministic(self) -> None:
        first = campaign.seed_input(20261009, 41)
        self.assertEqual(first, campaign.seed_input(20261009, 41))
        self.assertLessEqual(len(first), campaign.MAX_INPUT)
        binary = ROOT / "build/mac-asan/bin/xml_tokenizer_fuzz"
        if binary.is_file():
            self.assertEqual(campaign.replay_input(binary, 20261009, 41), first)


class TestCrashCapture(unittest.TestCase):
    def test_canary_capture_fingerprint_and_minimization(self) -> None:
        seed = (ROOT / "tests/svg/xml/fuzz/seeds/synthetic-canary.seed").read_bytes()
        first = campaign.canary_run(seed)
        second = campaign.canary_run(seed)
        self.assertNotEqual(first["returncode"], 0)
        self.assertEqual(campaign.fingerprint(first["stderr"]), campaign.fingerprint(second["stderr"]))
        expected = campaign.fingerprint(first["stderr"])
        minimized = campaign.minimize(seed, lambda data: campaign.initial_reproduces(data, expected))
        replay = campaign.canary_run(minimized)
        self.assertLessEqual(len(minimized), len(seed))
        self.assertEqual(campaign.fingerprint(replay["stderr"]), expected)
        self.assertIn(b"synthetic crash canary", minimized)

    def test_stack_fingerprint_removes_paths_addresses_and_line_numbers(self) -> None:
        a = b'  File "/tmp/build/a.py", line 12, in crash\nRuntimeError: SYNTHETIC_HARNESS_CANARY\n0x1234 in tokenizer+0x99'
        b = b'  File "/private/tmp/other/b.py", line 840, in crash\nRuntimeError: SYNTHETIC_HARNESS_CANARY\n0x9876 in tokenizer+0x99'
        self.assertEqual(campaign.fingerprint(a), campaign.fingerprint(b))

    def test_stack_fingerprint_normalizes_macos_var_folders(self) -> None:
        a = b"#0 0x1234 in crash /var/folders/aa/bb/T/session/file.c:12"
        b = b"#0 0x9876 in crash /private/var/folders/cc/dd/T/other/file.c:91"
        self.assertEqual(campaign.fingerprint(a), campaign.fingerprint(b))

    def test_invariant_replay_preserves_failure_iteration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "synthetic-fuzzer"
            binary.write_text(
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "print(f'span/offset invariant failed at iteration {sys.argv[3]}', file=sys.stderr)\n"
                "raise SystemExit(4)\n",
                encoding="utf-8",
            )
            binary.chmod(0o755)
            expected = campaign.fingerprint("span/offset invariant failed at iteration 73")
            self.assertTrue(campaign.input_reproduces(binary, b"<a/>", expected, 73))
            self.assertFalse(campaign.input_reproduces(binary, b"<a/>", expected, 0))
            self.assertEqual(campaign.one_input_command(binary, b"<a/>", 73)[-1], "73")


class TestLimitsAndCleanup(unittest.TestCase):
    def test_timeout_kills_supervised_process_group(self) -> None:
        child = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)']); time.sleep(30)"
        result = campaign.supervised([sys.executable, "-c", child], 0.15)
        self.assertTrue(result["timed_out"])
        self.assertTrue(result["group_terminated"])

    def test_input_and_duration_bounds(self) -> None:
        with self.assertRaises(SystemExit):
            campaign.parse_args(["--duration", "601"])
        with self.assertRaises(SystemExit):
            campaign.parse_args(["--seed", "4294967296"])
        self.assertLessEqual(campaign.MAX_INPUT, 16 * 1024 * 1024)
        self.assertEqual(campaign.MAX_DURATION, 600)

    def test_output_path_confinement(self) -> None:
        self.assertTrue(campaign.confined_output_path(".cache/WO-M2-03/report.json").is_relative_to(ROOT))
        with self.assertRaises(ValueError):
            campaign.confined_output_path(".cache/WO-M2-03/../../elsewhere.json")
        with self.assertRaises(ValueError):
            campaign.confined_output_path("artifacts/WO-M2-02/report.json")


class TestRegressionMap(unittest.TestCase):
    def test_issue_ids_map_to_hashed_seed_files(self) -> None:
        document = json.loads((ROOT / "tests/svg/xml/fuzz/regressions.json").read_text())
        mapping = campaign.validate_regression_records(document["regressions"], ROOT / "tests/svg/xml/fuzz/seeds")
        self.assertEqual(mapping, {})
        seed = (ROOT / "tests/svg/xml/fuzz/seeds/synthetic-canary.seed").read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            seed_root = Path(directory)
            (seed_root / "regression.seed").write_bytes(seed)
            record = {"issue_id": "XML-FUZZ-0001", "seed_file": "regression.seed",
                      "input_sha256": hashlib.sha256(seed).hexdigest(),
                      "fingerprint": "a" * 64,
                      "replay_test": "tests.test_xml_fuzz_campaign.TestRegressionMap"}
            self.assertEqual(campaign.validate_regression_records([record], seed_root)["XML-FUZZ-0001"],
                             (seed_root / "regression.seed").resolve())
            with self.assertRaises(ValueError):
                campaign.validate_regression_records([record, record], seed_root)
            invalid = dict(record)
            del invalid["fingerprint"]
            with self.assertRaises(ValueError):
                campaign.validate_regression_records([invalid], seed_root)

    def test_every_retained_record_is_replayed_by_this_test(self) -> None:
        document = json.loads((ROOT / "tests/svg/xml/fuzz/regressions.json").read_text())
        records = document["regressions"]
        mapping = campaign.validate_regression_records(records, ROOT / "tests/svg/xml/fuzz/seeds")
        binary = ROOT / "build/mac-asan/bin/xml_tokenizer_fuzz"
        for record in records:
            issue_id = record["issue_id"]
            seed_path = mapping[issue_id]
            with self.subTest(issue_id=issue_id):
                self.assertTrue(seed_path.is_file())
                self.assertRegex(issue_id, r"^XML-FUZZ-\d{4}$")
                self.assertTrue(binary.is_file(), "build xml_tokenizer_fuzz before replaying regressions")
                result = campaign.supervised(campaign.one_input_command(
                    binary, seed_path.read_bytes(), int(record.get("iteration", 0))), 2.0)
                self.assertFalse(result["timed_out"])
                self.assertEqual(result["returncode"], 0, result["stderr"].decode("utf-8", "replace"))


class TestWorkflowPolicy(unittest.TestCase):
    def setUp(self) -> None:
        self.workflow = (ROOT / ".github/workflows/fuzz.yml").read_text()

    def test_workflow_contract_passes(self) -> None:
        check_fuzz_workflow.check(self.workflow)

    def test_rejects_wrong_duration_runner_permissions_and_timeout(self) -> None:
        mutants = [
            self.workflow.replace("--duration 30", "--duration 3"),
            self.workflow.replace("--duration 600", "--duration 60"),
            self.workflow.replace("runs-on: macos-15", "runs-on: ubuntu-latest"),
            self.workflow.replace("contents: read", "contents: write"),
            self.workflow.replace("  contents: read", "  contents: read\n  actions: write"),
            self.workflow.replace("timeout-minutes: 20", "timeout-minutes: 21"),
        ]
        for mutant in mutants:
            with self.subTest(mutant=mutant[-80:]), self.assertRaises(ValueError):
                check_fuzz_workflow.check(mutant)

    def test_rejects_unpinned_actions_secrets_and_broad_failure_paths(self) -> None:
        mutants = [
            self.workflow.replace("@3d3c42e5aac5ba805825da76410c181273ba90b1", "@main"),
            self.workflow + "\n      env:\n        TOKEN: ${{ secrets.TOKEN }}\n",
            self.workflow.replace("artifacts/WO-M2-03/failures", "artifacts"),
        ]
        for mutant in mutants:
            with self.subTest(mutant=mutant[-80:]), self.assertRaises(ValueError):
                check_fuzz_workflow.check(mutant)


if __name__ == "__main__":
    unittest.main()
