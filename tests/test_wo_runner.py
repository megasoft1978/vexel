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

    def test_scope_rejects_escape_and_protected_paths(self) -> None:
        allow, protected = ["scripts/**"], ["scripts/oracle/**"]
        self.assertTrue(wo_validate.changed_path_allowed("scripts/wo_driver.py", allow, protected))
        for path in ("scripts/oracle/frozen.py", "../outside", "/tmp/outside"):
            self.assertFalse(wo_validate.changed_path_allowed(path, allow, protected))


class RunnerPrimitiveTests(unittest.TestCase):
    def fake_help(self, missing: str = ""):
        help_text = "--model --config --json --output-schema --output-last-message --image " \
                    "--dangerously-bypass-approvals-and-sandbox --skip-git-repo-check"
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
        with self.assertRaisesRegex(wo_driver.RunnerError, "--dangerously-bypass"):
            wo_driver.preflight("fake-codex", self.fake_help("--dangerously-bypass-approvals-and-sandbox"))

    def test_preflight_requires_exact_option_tokens(self) -> None:
        help_text = "--model-extra --config --json --output-schema --output-last-message --image " \
                    "--dangerously-bypass-approvals-and-sandbox --skip-git-repo-check"
        self.assertFalse(wo_driver._has_flag(help_text, "--model"))

    def test_driver_rejects_non_darwin_before_codex(self) -> None:
        with patch.object(wo_driver.sys, "platform", "linux"):
            self.assertEqual(wo_driver.main(["new", "WO-BOOT-04"]), 2)

    def test_session_parser_ignores_unknown_events_and_bad_ids(self) -> None:
        events = "\n".join([json.dumps({"type": "future.event", "thread_id": "ignored"}),
            "not-json", json.dumps({"type": "thread.started", "thread_id": "bad"}),
            json.dumps({"type": "thread.started", "thread_id": "12345678-1234-1234-1234-123456789abc"})])
        self.assertEqual(wo_driver.parse_session_id(events), "12345678-1234-1234-1234-123456789abc")

    def test_report_schema_rejects_extra_fields_and_forged_failure(self) -> None:
        schema = json.loads((wo_validate.ROOT / "docs/schemas/executor-report.schema.json").read_text())
        good = {"status": "complete", "base_sha": "a" * 40, "head_sha": "b" * 40,
                "acceptance": {"AC-1": True}, "summary": "done"}
        self.assertTrue(wo_driver.validate_json_schema(good, schema))
        self.assertFalse(wo_driver.validate_json_schema(good | {"extra": True}, schema))
        good["acceptance"]["AC-1"] = False
        with temporary_directory() as name:
            p = Path(name) / "report.json"; p.write_text(json.dumps(good))
            with self.assertRaisesRegex(wo_driver.RunnerError, "acceptance"):
                wo_driver.validate_report(p, wo_validate.ROOT / "docs/schemas/executor-report.schema.json",
                                          "a" * 40, {"AC-1"})

    def test_malformed_jsonl_and_missing_thread_id_fail_closed(self) -> None:
        self.assertIsNone(wo_driver.parse_session_id("{bad json}\n"))
        self.assertIsNone(wo_driver.parse_session_id('{"type":"turn.completed"}\n'))

    def test_missing_report_and_schema_fail(self) -> None:
        with temporary_directory() as name:
            with self.assertRaisesRegex(wo_driver.RunnerError, "missing or malformed"):
                wo_driver.validate_report(Path(name) / "missing.json", Path(name) / "schema.json",
                                          "a" * 40, {"AC-1"})

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
            (root / "scripts/code.py").write_text("reviewable code\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True); subprocess.run(["git", "commit", "-qm", "candidate"], cwd=root, check=True)
            head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            diff = wo_driver.bounded_review_diff(root, base, head)
            self.assertLess(len(diff), 1000)
            self.assertNotIn(b"xxxx", diff)

    def test_dirty_base_is_rejected(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "file").write_text("clean\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            wo_driver.require_clean_base(root, base)
            (root / "file").write_text("dirty\n")
            with self.assertRaisesRegex(wo_driver.RunnerError, "clean main"):
                wo_driver.require_clean_base(root, base)

    def test_verifier_stops_when_deterministic_check_fails(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            (root / "work").mkdir(); (root / "work/WO-FAKE.md").write_text(
                "# Verification commands\n\n```text\npython3 -c 'raise SystemExit(1)'\n```\n")
            (root / "file").write_text("x\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "candidate"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            fields = {"revision": "1", "base_sha": candidate}
            with self.assertRaisesRegex(wo_driver.RunnerError, "command 1 failed"):
                wo_driver.run_verifier(root, "WO-FAKE", fields, "a" * 64, candidate, "unused", 2)

    def test_fake_codex_verifier_uses_read_only_exact_commit(self) -> None:
        with temporary_directory() as name:
            root = Path(name); subprocess.run(["git", "init", "-q", "-b", "main"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.email", "test@example.invalid"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
            schema_dir = root / "docs/schemas"; schema_dir.mkdir(parents=True)
            for schema in ("executor-report.schema.json", "verifier-verdict.schema.json"):
                (schema_dir / schema).write_text((wo_validate.ROOT / "docs/schemas" / schema).read_text())
            (root / "work").mkdir(); (root / "work/WO-FAKE.md").write_text(
                "# Verification commands\n\n```text\npython3 -c 'print(1)'\n```\n")
            (root / "file").write_text("candidate\n"); subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "candidate"], cwd=root, check=True)
            candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            fake = root / ".cache/fake-verifier"; fake.parent.mkdir()
            fake.write_text("""#!/usr/bin/env python3
import json, pathlib, sys
flags = '--model --config --sandbox --json --output-schema --output-last-message --image --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check'
if '--version' in sys.argv: print('codex-cli fake'); raise SystemExit(0)
if '--help' in sys.argv: print(flags); raise SystemExit(0)
target = pathlib.Path(sys.argv[sys.argv.index('--output-last-message') + 1])
target.parent.mkdir(parents=True, exist_ok=True)
(target.parent / 'argv.txt').write_text(' '.join(sys.argv))
target.write_text(json.dumps({'verdict':'pass','findings':[],'summary':'reviewed'}))
print(json.dumps({'type':'thread.started','thread_id':'12345678-1234-1234-1234-123456789abc'}), flush=True)
"""); fake.chmod(0o755)
            fields = {"revision": "1", "base_sha": candidate}
            result = wo_driver.run_verifier(root, "WO-FAKE", fields, "a" * 64, candidate, str(fake), 10)
            self.assertEqual(result["verdict"]["verdict"], "pass")
            self.assertEqual(result["candidate_sha"], candidate)
            argv_record = next((root / "artifacts/runs/WO-FAKE").glob("verify-*/argv.txt")).read_text()
            self.assertIn("-s read-only", argv_record)
            self.assertTrue(result["session_id"])

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
            (root / ".gitignore").write_text("/.worktrees/\n/.cache/\n/artifacts/runs/\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "base"], cwd=root, check=True)
            base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            body = ("---\nid: WO-FAKE\nrevision: 1\nstatus: READY\nmilestone: M0\n"
                    "owner: test\nexecutor: test\nbase_sha: " + base + "\nparallel_safe: false\n"
                    "spec_sha256: PENDING\n---\n# Files allowed to touch\n- `scripts/**`\n"
                    "- `artifacts/WO-BOOT-04/**`\n# Protected paths\n- `scripts/oracle/**`\n"
                    "# Acceptance criteria (machine-checkable)\n- AC-1 passes\n")
            digest = hashlib.sha256(body.replace("spec_sha256: PENDING\n", "").encode()).hexdigest()
            (root / "work").mkdir()
            (root / "work/WO-FAKE.md").write_text(body.replace("spec_sha256: PENDING", f"spec_sha256: {digest}"))
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-qm", "freeze order"], cwd=root, check=True)
            fake = root / ".cache/fake-codex"; fake.parent.mkdir(exist_ok=True)
            fake.write_text("""#!/usr/bin/env python3
import json, pathlib, re, subprocess, sys
flags = '--model --config --json --output-schema --output-last-message --image --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check'
if '--version' in sys.argv: print('codex-cli fake'); raise SystemExit(0)
if '--help' in sys.argv: print(flags); raise SystemExit(0)
target = pathlib.Path(sys.argv[sys.argv.index('--output-last-message') + 1])
text = pathlib.Path('work/WO-FAKE.md').read_text()
base = re.search(r'^base_sha: ([0-9a-f]{40})$', text, re.M).group(1)
pathlib.Path('scripts/generated.py').write_text('generated\\n')
subprocess.run(['git', 'add', 'scripts/generated.py'], check=True)
subprocess.run(['git', 'commit', '-m', 'fake candidate'], check=True)
head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps({'status':'complete','base_sha':base,'head_sha':head,'acceptance':{'AC-1':True},'summary':'ok'}))
print(json.dumps({'type':'thread.started','thread_id':'12345678-1234-1234-1234-123456789abc'}), flush=True)
""")
            fake.chmod(0o755)
            with patch.object(wo_driver, "ROOT", root), patch.object(wo_driver.sys, "platform", "darwin"):
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(wo_driver.main(["new", "WO-FAKE", "--codex", str(fake)]), 0)
            candidate = root / ".worktrees/WO-FAKE"
            self.assertEqual(subprocess.check_output(["git", "branch", "--show-current"], cwd=candidate, text=True).strip(), "wo/WO-FAKE")
            self.assertTrue((root / ".cache/threads.jsonl").exists())

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
