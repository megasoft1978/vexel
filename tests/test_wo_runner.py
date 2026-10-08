"""Offline fake-Codex coverage for frozen work orders and runner failures."""

from __future__ import annotations

import hashlib
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import wo_driver
import wo_validate


def temporary_directory() -> tempfile.TemporaryDirectory[str]:
    cache = wo_validate.ROOT / ".cache"
    cache.mkdir(exist_ok=True)
    return tempfile.TemporaryDirectory(dir=cache)


def fake_work_order(wo_id: str, base: str, commands: str = "python3 -c 'print(1)'") -> str:
    body = (f"---\nid: {wo_id}\nrevision: 1\nstatus: READY\nmilestone: M0\n"
            f"owner: test\nexecutor: test\nbase_sha: {base}\nparallel_safe: false\n"
            "spec_sha256: PENDING\n---\n# Files allowed to touch\n- `scripts/**`\n"
            "# Acceptance criteria (machine-checkable)\n- AC-1 passes\n"
            f"# Verification commands\n\n```text\n{commands}\n```\n")
    digest = hashlib.sha256(body.replace("spec_sha256: PENDING\n", "").encode()).hexdigest()
    return body.replace("spec_sha256: PENDING", f"spec_sha256: {digest}")


class FrozenWorkOrderTests(unittest.TestCase):
    def test_current_spec_hash_is_valid(self) -> None:
        fields, digest = wo_validate.load_wo("WO-BOOT-04")
        self.assertEqual(fields["status"], "READY")
        self.assertEqual(len(digest), 64)

    def test_hash_mutation_is_rejected(self) -> None:
        with temporary_directory() as name:
            root = Path(name); (root / "work").mkdir()
            source = (wo_validate.ROOT / "work/WO-BOOT-04.md").read_text()
            (root / "work/WO-BOOT-04.md").write_text(source + "\nmutation\n")
            with self.assertRaises(wo_validate.ValidationError):
                wo_validate.load_wo("WO-BOOT-04", root)

    def test_work_order_symlink_escape_is_rejected(self) -> None:
        with temporary_directory() as name:
            sandbox = Path(name); root = sandbox / "repo"; root.mkdir()
            outside = sandbox / "outside.md"
            outside.write_text("not a work order")
            (root / "work").mkdir(); (root / "work/WO-ESCAPE.md").symlink_to(outside)
            with self.assertRaisesRegex(wo_validate.ValidationError, "escapes"):
                wo_validate.load_wo("WO-ESCAPE", root)

    def test_root_work_order_snapshot_rejects_mutation(self) -> None:
        with temporary_directory() as name:
            root = Path(name); (root / "work").mkdir()
            source = (wo_validate.ROOT / "work/WO-BOOT-HARDEN-01.md").read_bytes()
            (root / "work/WO-BOOT-HARDEN-01.md").write_bytes(source)
            _, digest = wo_validate.load_wo("WO-BOOT-HARDEN-01", root)
            wo_driver.assert_root_work_order_snapshot(root, "WO-BOOT-HARDEN-01", digest, source)
            (root / "work/WO-BOOT-HARDEN-01.md").write_bytes(source + b"\nchanged\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "frozen root work order"):
                wo_driver.assert_root_work_order_snapshot(root, "WO-BOOT-HARDEN-01", digest, source)

    def test_scope_rejects_escape_and_protected_paths(self) -> None:
        allow, protected = ["scripts/**"], ["scripts/oracle/**"]
        self.assertTrue(wo_validate.changed_path_allowed("scripts/wo_driver.py", allow, protected))
        for path in ("scripts/oracle/frozen.py", "../outside", "/tmp/outside"):
            self.assertFalse(wo_validate.changed_path_allowed(path, allow, protected))

    def test_work_order_path_rules_read_all_comma_separated_entries(self) -> None:
        body = (wo_validate.ROOT / "work/WO-BOOT-04.md").read_text()
        allowed, protected = wo_driver.path_rules(body)
        self.assertIn("scripts/wo-verify.sh", allowed)
        self.assertIn("docs/adr/0003-codex-cli-compatibility-amendment.md", allowed)
        self.assertIn("tests/golden/manifest.lock", protected)
        self.assertIn(".github/**", protected)


class RunnerPrimitiveTests(unittest.TestCase):
    def test_recursive_work_order_path_rules_allow_nested_evidence_and_protect_subtrees(self) -> None:
        allowed, protected = ["artifacts/WO-BOOT-05/**", "tests/fuzz/**"], ["tests/oracle/**"]
        self.assertTrue(wo_driver.work_order_path_allowed("artifacts/WO-BOOT-05/probes/probes.json", allowed, protected))
        self.assertTrue(wo_driver.work_order_path_allowed("tests/fuzz/cases/README.md", allowed, protected))
        self.assertFalse(wo_driver.work_order_path_allowed("tests/oracle/frozen.json", allowed + ["tests/**"], protected))
        self.assertFalse(wo_driver.work_order_path_allowed("../outside", allowed, protected))

    def test_frozen_paths_are_hard_denied_under_overlapping_allowlists(self) -> None:
        allowed = ["**", "work/**", "tests/**", "bench/**", "OWNER-DIRECTIVES.md"]
        for path in ("work/WO-OTHER.md", "tests/oracle/frozen.json",
                     "tests/golden/manifest.lock", "bench/corpus/manifest.lock",
                     "OWNER-DIRECTIVES.md"):
            with self.subTest(path=path):
                self.assertFalse(wo_driver.work_order_path_allowed(path, allowed, []))

    def test_executor_sandbox_defaults_closed_and_requires_explicit_network(self) -> None:
        help_text = "-s --sandbox --dangerously-bypass-approvals-and-sandbox"
        self.assertEqual(wo_driver.executor_sandbox_args({}, help_text), ["-s", "workspace-write"])
        self.assertEqual(wo_driver.executor_sandbox_args({"network": "false"}, help_text),
                         ["-s", "workspace-write"])
        self.assertEqual(wo_driver.executor_sandbox_args({"network": "not-a-boolean"}, help_text),
                         ["-s", "workspace-write"])
        with self.assertRaisesRegex(wo_driver.RunnerError, "requires the approved"):
            wo_driver.executor_sandbox_args({"network": "true"}, "-s --sandbox")
        self.assertEqual(wo_driver.executor_sandbox_args({"network": "true"}, help_text),
                         ["--dangerously-bypass-approvals-and-sandbox"])

    def fake_help(self, missing: str = ""):
        help_text = "--model --config --json --output-schema --output-last-message --image " \
                    "-s --sandbox --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check"
        help_text = help_text.replace(missing, "")
        def fake(args, **kwargs):
            if args[-1] == "--version":
                return SimpleNamespace(stdout="codex-cli fake\n", stderr="")
            return SimpleNamespace(stdout=help_text, stderr="")
        return fake

    def test_preflight_supported_fake(self) -> None:
        version, exec_help, resume_help = wo_driver.preflight("fake-codex", self.fake_help())
        self.assertEqual(version, "codex-cli fake")
        self.assertIn("--output-schema", exec_help)
        self.assertIn("--output-schema", resume_help)

    def test_preflight_rejects_unsupported_flag(self) -> None:
        with self.assertRaisesRegex(wo_driver.RunnerError, "-s"):
            wo_driver.preflight("fake-codex", self.fake_help("-s"))

    def test_preflight_requires_exact_option_tokens(self) -> None:
        help_text = "--model-extra --config --json --output-schema --output-last-message --image " \
                    "--dangerously-bypass-approvals-and-sandbox --skip-git-repo-check"
        self.assertFalse(wo_driver._has_flag(help_text, "--model"))

    def test_resume_preflight_checks_every_invocation_capability(self) -> None:
        good = " ".join(wo_driver.EXEC_REQUIRED)
        def fake(args, **kwargs):
            if args[-1] == "--version": return SimpleNamespace(stdout="fake", stderr="")
            text = good if args[2] != "resume" else good.replace("--output-last-message", "")
            return SimpleNamespace(stdout=text, stderr="")
        with self.assertRaisesRegex(wo_driver.RunnerError, "resume:--output-last-message"):
            wo_driver.preflight("fake", fake)

    def test_resume_help_may_omit_inherited_short_sandbox_alias(self) -> None:
        help_text = " ".join(wo_driver.EXEC_REQUIRED)
        def fake(args, **kwargs):
            if args[-1] == "--version": return SimpleNamespace(stdout="fake", stderr="")
            result = help_text if args[2] != "resume" else help_text.replace(" -s ", " ")
            return SimpleNamespace(stdout=result, stderr="")
        self.assertEqual(wo_driver.preflight("fake", fake)[0], "fake")

    def test_driver_rejects_non_darwin_before_codex(self) -> None:
        with patch.object(wo_driver.sys, "platform", "linux"):
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(wo_driver.main(["new", "WO-BOOT-04"]), 2)

    def test_session_parser_ignores_unknown_events_and_bad_ids(self) -> None:
        events = "\n".join([json.dumps({"type": "future.event", "thread_id": "ignored"}),
            "not-json", json.dumps({"type": "thread.started", "thread_id": "bad"}),
            json.dumps({"type": "thread.started", "thread_id": "12345678-1234-1234-1234-123456789abc"})])
        self.assertEqual(wo_driver.parse_session_id(events), "12345678-1234-1234-1234-123456789abc")

    def test_report_schema_rejects_extra_fields_and_forged_failure(self) -> None:
        schema = json.loads((wo_validate.ROOT / "docs/schemas/executor-report.schema.json").read_text())
        evidence = {"commands": [{"argv": ["make"], "exit_code": 0}]}
        good = {"status": "complete", "base_sha": "a" * 40, "head_sha": "b" * 40,
                "acceptance": {"AC-1": evidence}, "summary": "done"}
        self.assertTrue(wo_driver.validate_json_schema(good, schema))
        self.assertFalse(wo_driver.validate_json_schema(good | {"extra": True}, schema))
        with temporary_directory() as name:
            p = Path(name) / "report.json"; p.write_text(json.dumps(good))
            with self.assertRaisesRegex(wo_driver.RunnerError, "acceptance"):
                wo_driver.validate_report(p, wo_validate.ROOT / "docs/schemas/executor-report.schema.json",
                                          "a" * 40, {"AC-1"}, {("python3",)})

    def test_malformed_jsonl_and_missing_thread_id_fail_closed(self) -> None:
        self.assertIsNone(wo_driver.parse_session_id("{bad json}\n"))
        self.assertIsNone(wo_driver.parse_session_id('{"type":"turn.completed"}\n'))

    def test_missing_report_and_schema_fail(self) -> None:
        with temporary_directory() as name:
            with self.assertRaisesRegex(wo_driver.RunnerError, "missing or malformed"):
                wo_driver.validate_report(Path(name) / "missing.json", Path(name) / "schema.json",
                                          "a" * 40, {"AC-1"}, set())

    def test_fake_codex_success_and_timeout(self) -> None:
        with temporary_directory() as name:
            root = Path(name); fake = root / "fake-codex"
            fake.write_text("#!/usr/bin/env python3\nimport sys,time\nprint('{\\\"type\\\":\\\"thread.started\\\",\\\"thread_id\\\":\\\"12345678-1234-1234-1234-123456789abc\\\"}',flush=True)\n")
            fake.chmod(0o755); cache = root / "threads.jsonl"
            key = {"wo_id": "WO-FAKE", "revision": 1}
            code, _, output = wo_driver.run_captured([str(fake)], root, 2, "go", root / "out", root / "err", cache, key)
            self.assertEqual(code, 0); self.assertIsNotNone(wo_driver.parse_session_id(output))
            self.assertIn("12345678-1234-1234-1234-123456789abc", cache.read_text())
            sleeper = root / "slow-codex"
            sleeper.write_text("#!/usr/bin/env python3\nimport time\ntime.sleep(3)\n"); sleeper.chmod(0o755)
            code, _, _ = wo_driver.run_captured([str(sleeper)], root, 0.05, "", root / "out2", root / "err2", cache, key)
            self.assertEqual(code, 124)

    def test_session_cache_write_error_fails_closed(self) -> None:
        with temporary_directory() as name:
            root = Path(name); fake = root / "fake-codex"
            fake.write_text("#!/usr/bin/env python3\nprint('{\\\"type\\\":\\\"thread.started\\\",\\\"thread_id\\\":\\\"12345678-1234-1234-1234-123456789abc\\\"}',flush=True)\n")
            fake.chmod(0o755); cache = root / "threads.jsonl"; cache.write_text("truncated")
            with self.assertRaisesRegex(wo_driver.RunnerError, "session event"):
                wo_driver.run_captured([str(fake)], root, 2, "", root / "out", root / "err", cache, {})
            self.assertTrue((root / "out").exists())
            self.assertTrue((root / "err").exists())

    def test_resume_identity_and_changed_cache_key(self) -> None:
        with temporary_directory() as name:
            root = Path(name); cache = root / ".cache/threads.jsonl"
            key = {"wo_id": "WO-A", "revision": 1, "spec_sha256": "a" * 64}
            wo_driver.append_jsonl_atomic(cache, {"key": key, "session_id": "12345678-1234-1234-1234-123456789abc"})
            session = "12345678-1234-1234-1234-123456789abc"
            self.assertTrue(wo_driver.cache_matches(root, key, session))
            self.assertFalse(wo_driver.cache_matches(root, key | {"revision": 2}, session))
            self.assertFalse(wo_driver.cache_matches(root, key, "not-a-session"))

    def test_scope_git_protected_and_disallowed_paths(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "scripts").mkdir(); (root / "scripts/ok.py").write_text("ok\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            body = "# Files allowed to touch\n- `scripts/**`\n# Protected paths\n- `scripts/oracle/**`\n"
            (root / "scripts/ok.py").write_text("changed\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "ok"], cwd=root, check=True)
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            self.assertEqual(wo_driver.assert_scope(root, base, head, body), ["scripts/ok.py"])
            (root / "outside.txt").write_text("bad\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "bad"], cwd=root, check=True)
            bad = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, bad, body)
            (root / "scripts/oracle").mkdir(parents=True)
            (root / "scripts/oracle/frozen.py").write_text("frozen\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "protected"], cwd=root, check=True)
            protected = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, protected, body)
            frozen = root / "work/WO-FAKE.md"
            frozen.parent.mkdir(exist_ok=True); frozen.write_text("immutable\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "frozen order"], cwd=root, check=True)
            frozen_head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, frozen_head,
                    "# Files allowed to touch\n- `work/**`\n", "WO-FAKE")

    def test_candidate_scope_accepts_unchanged_active_root_copy(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "scripts").mkdir(); (root / "scripts/ok.py").write_text("before\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            (root / "work").mkdir(); (root / "work/WO-FAKE.md").write_text(fake_work_order("WO-FAKE", base))
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze order"], cwd=root, check=True)
            wo_driver.load_wo("WO-FAKE", root)
            (root / "scripts/ok.py").write_text("candidate\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "allowed change"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            changed = wo_driver.assert_scope(root, base, candidate,
                fake_work_order("WO-FAKE", base), "WO-FAKE")
            self.assertEqual(changed, ["scripts/ok.py", "work/WO-FAKE.md"])

    def test_candidate_scope_hard_denies_frozen_paths_under_broad_allowlist(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "baseline.txt").write_text("base\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            paths = ("work/WO-OTHER.md", "tests/oracle/frozen.json",
                     "tests/golden/manifest.lock", "bench/corpus/manifest.lock",
                     "OWNER-DIRECTIVES.md")
            for relpath in paths:
                target = root / relpath; target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("mutation\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "mutate frozen inputs"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, candidate,
                    "# Files allowed to touch\n- `**`\n- `work/**`\n- `tests/**`\n"
                    "- `bench/**`\n- `OWNER-DIRECTIVES.md`\n", "WO-FAKE")

    def test_only_first_write_m0_02_baseline_additions_are_authorized(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "seed.txt").write_text("seed\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            baseline_body = ("---\nid: WO-M0-02\nbaseline: true\n---\n"
                             "# Files allowed to touch\n- `tests/oracle/**`\n- `tests/golden/**`\n"
                             "# Protected paths\n- `tests/oracle/**`\n- `tests/golden/manifest.lock`\n")
            for relpath in ("tests/oracle/metadata.json", "tests/golden/manifest.lock",
                            "tests/golden/fixtures/pixel.png"):
                path = root / relpath; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("first write\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "seed baseline"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            self.assertCountEqual(wo_driver.assert_scope(root, base, candidate, baseline_body, "WO-M0-02"),
                                  ["tests/oracle/metadata.json", "tests/golden/manifest.lock",
                                   "tests/golden/fixtures/pixel.png"])
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, candidate, baseline_body, "WO-M0-03")
            later_path = root / "tests/oracle/later.json"
            later_path.write_text("later addition\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "later addition"], cwd=root, check=True)
            later = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, candidate, later, baseline_body, "WO-M0-02")
            late = root / "tests/oracle/later.json"; late.write_text("second seed\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "extend baseline"], cwd=root, check=True)
            extension = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, candidate, extension, baseline_body, "WO-M0-02")

    def test_baseline_mutations_deletions_and_unrelated_frozen_paths_fail(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            oracle = root / "tests/oracle/metadata.json"; oracle.parent.mkdir(parents=True)
            oracle.write_text("baseline\n")
            golden = root / "tests/golden/fixtures/pixel.png"; golden.parent.mkdir(parents=True)
            golden.write_bytes(b"png baseline\n")
            (root / "seed.txt").write_text("seed\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            oracle.write_text("modified\n")
            golden.write_bytes(b"changed png\n")
            lock = root / "tests/golden/manifest.lock"; lock.parent.mkdir(parents=True, exist_ok=True)
            lock.write_text("unrelated first write\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "mutate and add"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            body = ("---\nid: WO-M0-02\nbaseline: true\n---\n# Files allowed to touch\n"
                    "- `tests/oracle/**`\n- `tests/golden/**`\n# Protected paths\n"
                    "- `tests/oracle/**`\n- `tests/golden/manifest.lock`\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, base, candidate, body, "WO-M0-02")

            subprocess.run(["git", "rm", "tests/oracle/metadata.json"], cwd=root, check=True,
                           capture_output=True)
            subprocess.run(["git", "commit", "-qm", "delete oracle"], cwd=root, check=True)
            deletion = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, candidate, deletion, body, "WO-M0-02")

            subprocess.run(["git", "checkout", "--", "tests/golden/fixtures/pixel.png"],
                           cwd=root, check=True)
            subprocess.run(["git", "rm", "tests/golden/fixtures/pixel.png"], cwd=root,
                           check=True, capture_output=True)
            subprocess.run(["git", "commit", "-qm", "delete golden png"], cwd=root, check=True)
            golden_deletion = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "disallowed"):
                wo_driver.assert_scope(root, deletion, golden_deletion, body, "WO-M0-02")

            # A broad pattern cannot turn root-scope checks into authorization for frozen paths.
            (root / "OWNER-DIRECTIVES.md").write_text("untracked frozen root file\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "out-of-scope"):
                wo_driver.assert_root_scope(root, "# Files allowed to touch\n- `**`\n")

    def test_root_scope_hard_denies_frozen_paths_under_broad_allowlist(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            body = ("# Files allowed to touch\n- `**`\n- `work/**`\n- `tests/**`\n"
                    "- `bench/**`\n- `OWNER-DIRECTIVES.md`\n")
            for relpath in ("work/WO-OTHER.md", "tests/oracle/frozen.json",
                            "tests/golden/manifest.lock", "bench/corpus/manifest.lock",
                            "OWNER-DIRECTIVES.md"):
                target = root / relpath; target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("mutation\n")
                with self.subTest(path=relpath), self.assertRaisesRegex(wo_driver.RunnerError, "out-of-scope"):
                    wo_driver.assert_root_scope(root, body)


    def test_recursive_work_order_globs_match_nested_files(self) -> None:
        allowed, protected = ["artifacts/WO-BOOT-05/**"], ["artifacts/WO-BOOT-05/private/**"]
        self.assertTrue(wo_driver.work_order_path_allowed(
            "artifacts/WO-BOOT-05/probes/heap_overflow.stderr.txt.gz", allowed, protected))
        self.assertFalse(wo_driver.work_order_path_allowed(
            "artifacts/WO-BOOT-05/private/key.txt", allowed, protected))
        self.assertFalse(wo_driver.work_order_path_allowed(
            "artifacts/WO-BOOT-06/result.json", allowed, protected))

    def test_review_diff_excludes_old_transcripts_and_stays_bounded(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "scripts").mkdir(); (root / "scripts/code.py").write_text("code\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            transcript = root / "artifacts/WO-FAKE/verifier-events.jsonl"
            transcript.parent.mkdir(parents=True); transcript.write_text("x" * 300_000)
            (transcript.parent / "verdict-round2.json").write_text('{"verdict":"fail"}')
            (root / "scripts/code.py").write_text("reviewable code\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "candidate"], cwd=root, check=True)
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            diff = wo_driver.bounded_review_diff(root, base, head)
            self.assertLess(len(diff), 1000)
            self.assertNotIn(b"xxxx", diff)
            self.assertNotIn(b'verdict', diff)

    def test_dirty_base_is_rejected(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "file").write_text("clean\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            wo_driver.require_clean_base(root, base)
            (root / "file").write_text("dirty\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "clean main"):
                wo_driver.require_clean_base(root, base)

    def test_new_run_rejects_clean_non_main_branch(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "feature"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "file").write_text("clean\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            with self.assertRaisesRegex(wo_driver.RunnerError, "main branch"):
                wo_driver.require_clean_base(root, base)

    def test_resume_rejects_dirty_task_worktree(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "file").write_text("base\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            (root / "file").write_text("dirty\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "clean task worktree"):
                wo_driver.require_clean_task(root)

    def test_executor_report_rejects_empty_acceptance_evidence(self) -> None:
        with temporary_directory() as name:
            root = Path(name)
            report = root / "report.json"
            report.write_text(json.dumps({"status": "complete", "base_sha": "a" * 40,
                                          "head_sha": "b" * 40, "acceptance": {}, "summary": "ok"}))
            schema = wo_validate.ROOT / "docs/schemas/executor-report.schema.json"
            with self.assertRaisesRegex(wo_driver.RunnerError, "acceptance evidence"):
                wo_driver.validate_report(report, schema, "a" * 40, {"AC-1"}, set())
            report.write_text(json.dumps({"status": "complete", "base_sha": "a" * 40,
                                          "head_sha": "b" * 40, "summary": "missing acceptance"}))
            with self.assertRaisesRegex(wo_driver.RunnerError, "schema validation"):
                wo_driver.validate_report(report, schema, "a" * 40, {"AC-1"}, set())

    def test_strict_executor_schema_expands_work_order_acceptance_ids(self) -> None:
        schema = json.loads((wo_validate.ROOT / "docs/schemas/executor-report.schema.json").read_text())
        result = wo_driver.strict_executor_schema(schema, {"AC-1", "AC-3"})
        acceptance = result["properties"]["acceptance"]
        self.assertEqual(acceptance["required"], ["AC-1", "AC-3"])
        self.assertEqual(set(acceptance["properties"]), {"AC-1", "AC-3"})
        self.assertIs(acceptance["additionalProperties"], False)
        with self.assertRaisesRegex(wo_driver.RunnerError, "no acceptance IDs"):
            wo_driver.strict_executor_schema(schema, set())

    def test_verifier_stops_when_deterministic_check_fails(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "file").write_text("x\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            (root / "work").mkdir(); (root / "work/WO-FAKE.md").write_text(
                fake_work_order("WO-FAKE", base, "python3 -c 'raise SystemExit(1)'"))
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze order"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            fields, spec_hash = wo_validate.load_wo("WO-FAKE", root)
            with self.assertRaisesRegex(wo_driver.RunnerError, "command 1 failed"):
                wo_driver.run_verifier(root, "WO-FAKE", fields, spec_hash, candidate, "unused", 2)

    def test_fake_codex_verifier_uses_read_only_exact_commit(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            schema_dir = root / "docs/schemas"; schema_dir.mkdir(parents=True)
            for schema in ("executor-report.schema.json", "verifier-verdict.schema.json"):
                (schema_dir / schema).write_text((wo_validate.ROOT / "docs/schemas" / schema).read_text())
            (root / "file").write_text("candidate\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            (root / "work").mkdir(); (root / "work/WO-FAKE.md").write_text(fake_work_order("WO-FAKE", base))
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze order"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            fake = root / ".cache/fake-verifier"; fake.parent.mkdir()
            fake.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
flags = '--model --config --sandbox -s --json --output-schema --output-last-message --image --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check'
if '--version' in sys.argv: print('codex-cli fake'); raise SystemExit(0)
if '--help' in sys.argv: print(flags); raise SystemExit(0)
target = pathlib.Path(sys.argv[sys.argv.index('--output-last-message') + 1])
target.parent.mkdir(parents=True, exist_ok=True)
(target.parent / 'argv.txt').write_text(' '.join(sys.argv))
target.write_text(json.dumps({'verdict':'pass','findings':[],'summary':'reviewed'}))
print(json.dumps({'type':'thread.started','thread_id':'12345678-1234-1234-1234-123456789abc'}), flush=True)
"""); fake.chmod(0o755)
            fields, spec_hash = wo_validate.load_wo("WO-FAKE", root)
            result = wo_driver.run_verifier(root, "WO-FAKE", fields, spec_hash, candidate, str(fake), 10)
            self.assertEqual(result["verdict"]["verdict"], "pass")
            self.assertEqual(result["candidate_sha"], candidate)
            argv_record = next((root / "artifacts/runs/WO-FAKE").glob("verify-*/argv.txt")).read_text()
            self.assertIn("-s read-only", argv_record)
            self.assertTrue(result["session_id"])

    def assert_fake_executor_lifecycle(self, root: Path, body: str,
                                       old_evidence: Path, old_bytes: bytes) -> None:
        candidate = root / ".worktrees/WO-FAKE"
        branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=candidate, text=True).strip()
        self.assertEqual(branch, "wo/WO-FAKE")
        self.assertTrue((root / ".cache/threads.jsonl").exists())
        self.assertEqual(old_evidence.read_bytes(), old_bytes)
        run_dir = next((root / "artifacts/runs/WO-FAKE").iterdir())
        self.assertIn("codex-cli fake", (run_dir / "codex-preflight.txt").read_text())
        self.assertIn("-s workspace-write", (run_dir / "argv.txt").read_text())
        wo_driver.assert_root_scope(root, body)

    def test_fake_codex_completes_new_worktree_lifecycle(self) -> None:
        with temporary_directory() as name:
            root = Path(name)
            subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "scripts").mkdir(); (root / "scripts/base.py").write_text("base\n")
            schema_dir = root / "docs/schemas"; schema_dir.mkdir(parents=True)
            (schema_dir / "executor-report.schema.json").write_text(
                (wo_validate.ROOT / "docs/schemas/executor-report.schema.json").read_text())
            old_evidence = root / "artifacts/WO-BOOT-04/codex-preflight.txt"
            old_evidence.parent.mkdir(parents=True)
            old_evidence.write_text("previous work order evidence\n")
            old_evidence_bytes = old_evidence.read_bytes()
            (root / ".gitignore").write_text("/.worktrees/\n/.cache/\n/artifacts/runs/\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            body = ("---\nid: WO-FAKE\nrevision: 1\nstatus: READY\nmilestone: M0\n"
                    "owner: test\nexecutor: test\nbase_sha: " + base + "\nparallel_safe: false\n"
                    "spec_sha256: PENDING\n---\n# Files allowed to touch\n- `scripts/**`\n"
                    "# Protected paths\n- `scripts/oracle/**`\n"
                    "# Acceptance criteria (machine-checkable)\n- AC-1 passes\n"
                    "# Verification commands\n\n```text\npython3 -c 'print(1)'\n```\n")
            digest = hashlib.sha256(body.replace("spec_sha256: PENDING\n", "").encode()).hexdigest()
            (root / "work").mkdir()
            (root / "work/WO-FAKE.md").write_text(body.replace("spec_sha256: PENDING", f"spec_sha256: {digest}"))
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze order"], cwd=root, check=True)
            fake = root / ".cache/fake-codex"; fake.parent.mkdir(exist_ok=True)
            fake.write_text("""#!/usr/bin/env python3
import json, pathlib, re, subprocess, sys
flags = '--model --config --json --output-schema --output-last-message --image -s --sandbox --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check'
if '--version' in sys.argv: print('codex-cli fake'); raise SystemExit(0)
if '--help' in sys.argv: print(flags); raise SystemExit(0)
target = pathlib.Path(sys.argv[sys.argv.index('--output-last-message') + 1])
(target.parent / 'argv.txt').write_text(' '.join(sys.argv))
text = pathlib.Path('work/WO-FAKE.md').read_text()
base = re.search(r'^base_sha: ([0-9a-f]{40})$', text, re.M).group(1)
pathlib.Path('scripts/generated.py').write_text('generated\\n')
subprocess.run(['git', 'add', 'scripts/generated.py'], check=True)
subprocess.run(['git', 'commit', '-m', 'fake candidate'], check=True)
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps({'status':'complete','base_sha':base,'head_sha':head,'acceptance':{'AC-1':{'commands':[{'argv':['python3','-c','print(1)'],'exit_code':0}]}},'summary':'ok'}))
print(json.dumps({'type':'thread.started','thread_id':'12345678-1234-1234-1234-123456789abc'}), flush=True)
""")
            fake.chmod(0o755)
            with patch.object(wo_driver, "ROOT", root), patch.object(wo_driver.sys, "platform", "darwin"):
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(wo_driver.main(["new", "WO-FAKE", "--codex", str(fake)]), 0)
            self.assert_fake_executor_lifecycle(root, body, old_evidence, old_evidence_bytes)

    def test_concurrent_lock_and_interrupted_atomic_append(self) -> None:
        with temporary_directory() as name:
            path = Path(name) / "threads.jsonl"; path.write_text('{"old":true}\n')
            with self.assertRaises(OSError):
                wo_driver.append_jsonl_atomic(path, {"new": True}, interrupted=True)
            self.assertEqual(path.read_text(), '{"old":true}\n')
            import fcntl
            lock = Path(name) / "lock"; first = lock.open("a+b"); second = lock.open("a+b")
            fcntl.flock(first.fileno(), fcntl.LOCK_EX)
            with self.assertRaises(BlockingIOError): fcntl.flock(second.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            first.close(); second.close()

    def test_runner_help_does_not_start_work(self) -> None:
        result = subprocess.run([str(wo_validate.ROOT / "scripts/wo-run.sh"), "--help"],
                                text=True, capture_output=True, check=True)
        self.assertIn("usage:", result.stdout.lower())


if __name__ == "__main__":
    unittest.main()
