#!/usr/bin/env python3
"""macOS work-order executor and read-only verifier orchestration."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Sequence

try:
    from .wo_validate import ID_RE, ROOT, ValidationError, changed_path_allowed, load_wo, validate_json_schema
except ImportError:
    from wo_validate import ID_RE, ROOT, ValidationError, changed_path_allowed, load_wo, validate_json_schema

SESSION_RE = re.compile(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
EXEC_REQUIRED = ("--model", "--config", "--json", "--output-schema", "--output-last-message", "--image", "--dangerously-bypass-approvals-and-sandbox", "--skip-git-repo-check")
RESUME_REQUIRED = EXEC_REQUIRED
VERIFY_REQUIRED = ("--model", "--config", "--sandbox", "--output-schema", "--output-last-message", "--json")


class RunnerError(RuntimeError):
    """Raised when a run fails a safety or integrity check."""


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def parse_session_id(jsonl: str) -> str | None:
    for line in jsonl.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(event, dict) and event.get("type") == "thread.started":
            value = event.get("thread_id")
            if isinstance(value, str) and SESSION_RE.fullmatch(value):
                return value
    return None


def preflight(codex: str, runner: Any = subprocess.run,
              evidence: Path | None = None) -> tuple[str, str, str]:
    version = runner([codex, "--version"], capture_output=True, text=True, check=True).stdout.strip()
    helps: list[str] = []
    for args in ([codex, "exec", "--help"], [codex, "exec", "resume", "--help"]):
        result = runner(args, capture_output=True, text=True, check=True)
        helps.append(result.stdout + result.stderr)
    if evidence is not None:
        evidence.parent.mkdir(parents=True, exist_ok=True)
        evidence.write_text("version: " + version + "\n\nexec help:\n" + helps[0] +
                            "\nresume help:\n" + helps[1], encoding="utf-8")
    missing = [flag for flag in EXEC_REQUIRED if not _has_flag(helps[0], flag)]
    missing.extend(f"resume:{flag}" for flag in RESUME_REQUIRED if not _has_flag(helps[1], flag))
    if missing:
        raise RunnerError("Codex CLI missing executor capabilities: " + ", ".join(missing))
    return version, helps[0], helps[1]


def _has_flag(help_text: str, flag: str) -> bool:
    return re.search(rf"(?<![A-Za-z0-9_-]){re.escape(flag)}(?![A-Za-z0-9_-])", help_text) is not None


def append_jsonl_atomic(path: Path, item: dict[str, Any], interrupted: bool = False) -> None:
    """Append under a lock using fsync and atomic replacement."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.with_suffix(path.suffix + ".lock").open("a+b") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        old = path.read_bytes() if path.exists() else b""
        if old and not old.endswith(b"\n"):
            raise RunnerError("JSONL cache has a truncated final record")
        line = json.dumps(item, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        fd, name = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
        try:
            with os.fdopen(fd, "wb") as stream:
                stream.write(old + line)
                stream.flush()
                if interrupted:
                    raise OSError("injected interruption before fsync")
                os.fsync(stream.fileno())
            os.replace(name, path)
        finally:
            if os.path.exists(name):
                os.unlink(name)


def repo(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    if check and result.returncode:
        raise RunnerError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout.strip()


def path_rules(body: str) -> tuple[list[str], list[str]]:
    sections: dict[str, list[str]] = {"allowed": [], "protected": []}
    active = ""
    for line in body.splitlines():
        if line.strip() == "# Files allowed to touch":
            active = "allowed"
        elif line.strip() == "# Protected paths":
            active = "protected"
        elif line.startswith("# "):
            active = ""
        elif active:
            for group in re.findall(r"`([^`]+)`", line):
                sections[active].extend(part.strip() for part in group.split(",") if part.strip())
    return sections["allowed"], sections["protected"]


def assert_scope(root: Path, base: str, head: str, body: str,
                 wo_id: str | None = None) -> list[str]:
    allowed, protected = path_rules(body)
    result = subprocess.run(["git", "diff", "--name-only", "-z", base, head],
                            cwd=root, capture_output=True, check=True)
    changed = [part.decode("utf-8") for part in result.stdout.split(b"\0") if part]
    frozen_path = f"work/{wo_id}.md" if wo_id else None
    rejected = []
    for name in changed:
        if name == frozen_path and _frozen_work_order_unchanged(root, head, name):
            continue
        if name == frozen_path or not _scope_path_allowed(name, allowed, protected):
            rejected.append(name)
    if rejected:
        raise RunnerError("candidate changed disallowed paths: " + ", ".join(rejected))
    return changed


def _matches_work_order_pattern(path: str, pattern: str) -> bool:
    if changed_path_allowed(path, [pattern], []):
        return True
    if pattern.endswith("/**"):
        prefix = pattern[:-3].rstrip("/")
        return path.startswith(prefix + "/")
    return False


def _scope_path_allowed(path: str, allowed: list[str], protected: list[str]) -> bool:
    """Apply recursive `/**` work-order rules consistently with their meaning."""
    if path.startswith("/") or "\\" in path or ".." in Path(path).parts:
        return False
    if any(_matches_work_order_pattern(path, pattern) for pattern in protected):
        return False
    return any(_matches_work_order_pattern(path, pattern) for pattern in allowed)


def _frozen_work_order_unchanged(root: Path, head: str, path: str) -> bool:
    candidate = subprocess.run(["git", "show", f"{head}:{path}"], cwd=root, capture_output=True)
    return candidate.returncode == 0 and candidate.stdout == (root / path).read_bytes()


def cache_matches(root: Path, key: dict[str, Any], session: str) -> bool:
    cache = root / ".cache" / "threads.jsonl"
    if not cache.exists():
        return False
    for line in cache.read_text(encoding="utf-8").splitlines():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if entry.get("key") == key and entry.get("session_id") == session:
            return True
    return False


def run_captured(argv: Sequence[str], cwd: Path, timeout: float, prompt: str,
                 stdout_path: Path, stderr_path: Path, cache_path: Path,
                 cache_key: dict[str, Any]) -> tuple[int, float, str]:
    """Run one child process; record thread.started as soon as its line arrives."""
    started = time.monotonic()
    proc = subprocess.Popen(list(argv), cwd=cwd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, start_new_session=True, bufsize=0)
    assert proc.stdin is not None and proc.stdout is not None and proc.stderr is not None
    output = bytearray(); errors = bytearray(); thread_errors: list[BaseException] = []

    def drain(stream: Any, destination: bytearray, parse_events: bool = False) -> None:
        while True:
            line = stream.readline()
            if not line:
                return
            destination.extend(line)
            if parse_events:
                session = parse_session_id(line.decode("utf-8", errors="replace"))
                if session:
                    try:
                        append_jsonl_atomic(cache_path, {"key": cache_key, "session_id": session,
                                                         "ts": utc_now()})
                    except Exception as exc:
                        thread_errors.append(exc)

    out_thread = threading.Thread(target=drain, args=(proc.stdout, output, True), daemon=True)
    err_thread = threading.Thread(target=drain, args=(proc.stderr, errors), daemon=True)
    out_thread.start(); err_thread.start()
    try:
        proc.stdin.write(prompt.encode("utf-8"))
    except BrokenPipeError:
        pass
    proc.stdin.close()
    timed_out = False
    try:
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True; os.killpg(proc.pid, signal.SIGTERM)
        try: proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL); proc.wait()
    out_thread.join(); err_thread.join()
    proc.stdout.close(); proc.stderr.close()
    stdout_path.write_bytes(output); stderr_path.write_bytes(errors)
    if thread_errors:
        raise RunnerError(f"could not persist executor session event: {thread_errors[0]}")
    return (124 if timed_out else int(proc.returncode or 0), time.monotonic() - started,
            output.decode("utf-8", errors="replace"))


def create_worktree(root: Path, wo_id: str, base: str) -> Path:
    destination = root / ".worktrees" / wo_id
    if destination.exists():
        raise RunnerError("worktree path already exists")
    destination.parent.mkdir(parents=True, exist_ok=True)
    repo(root, "worktree", "add", "-b", f"wo/{wo_id}", str(destination), base)
    metadata = root / ".cache" / "worktrees" / f"{wo_id}.json"
    metadata.parent.mkdir(parents=True, exist_ok=True)
    metadata.write_text(json.dumps({"start_sha": base, "worktree": str(destination)}) + "\n")
    return destination


def worktree_start(root: Path, wo_id: str, worktree: Path) -> str:
    path = root / ".cache" / "worktrees" / f"{wo_id}.json"
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        start = value["start_sha"]
    except (OSError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise RunnerError("resume worktree start metadata is missing or malformed") from exc
    if value.get("worktree") != str(worktree) or not re.fullmatch(r"[0-9a-f]{40}", start):
        raise RunnerError("resume worktree metadata does not match")
    return start


def require_clean_base(root: Path, base: str) -> str:
    if repo(root, "branch", "--show-current") != "main":
        raise RunnerError("new run requires the main branch")
    head = repo(root, "rev-parse", "HEAD")
    if repo(root, "status", "--porcelain"):
        raise RunnerError("new run requires a clean main checkout")
    if subprocess.run(["git", "merge-base", "--is-ancestor", base, head], cwd=root).returncode:
        raise RunnerError("declared base_sha is not an ancestor of clean main")
    return head


def require_clean_task(worktree: Path) -> None:
    if repo(worktree, "status", "--porcelain"):
        raise RunnerError("resume requires a clean task worktree")


def validate_report(path: Path, schema_path: Path, base: str,
                    expected_acceptance: set[str],
                    executed_commands: set[tuple[str, ...]]) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise RunnerError(f"missing or malformed report/schema: {exc}") from exc
    if not validate_json_schema(report, schema):
        raise RunnerError("executor report failed schema validation")
    evidence_ok = all(isinstance(item, dict) and isinstance(item.get("commands"), list)
                      and item["commands"] and all(_command_passed(cmd, executed_commands) for cmd in item["commands"])
                      for item in report["acceptance"].values())
    if report["base_sha"] != base or set(report["acceptance"]) != expected_acceptance or not evidence_ok:
        raise RunnerError("report base or acceptance evidence is invalid")
    return report


def _command_passed(command: Any, executed_commands: set[tuple[str, ...]]) -> bool:
    return (isinstance(command, dict) and command.get("exit_code") == 0
            and isinstance(command.get("argv"), list) and bool(command["argv"])
            and all(isinstance(part, str) for part in command["argv"])
            and tuple(command["argv"]) in executed_commands)


def acceptance_ids(body: str) -> set[str]:
    return set(re.findall(r"(?m)^-\s+(AC-\d+)\b", body))


def strict_executor_schema(schema: dict[str, Any], acceptance: set[str]) -> dict[str, Any]:
    """Expand dynamic acceptance keys to the explicit object required by strict output."""
    if not acceptance:
        raise RunnerError("work order has no acceptance IDs for strict report schema")
    result = json.loads(json.dumps(schema))
    try:
        acceptance_schema = result["properties"]["acceptance"]
        item_schema = acceptance_schema.pop("additionalProperties")
    except (KeyError, AttributeError, TypeError) as exc:
        raise RunnerError("executor report schema has no dynamic acceptance item schema") from exc
    acceptance_schema["properties"] = {key: item_schema for key in sorted(acceptance)}
    acceptance_schema["required"] = sorted(acceptance)
    acceptance_schema["additionalProperties"] = False
    return result


def assert_root_scope(root: Path, body: str) -> None:
    allowed, protected = path_rules(body)
    status = repo(root, "status", "--porcelain", "--untracked-files=all")
    rejected: list[str] = []
    for line in status.splitlines():
        if len(line) < 4:
            continue
        path = line[3:].split(" -> ")[-1]
        if (path.startswith((".worktrees/", ".cache/", "artifacts/runs/"))
                or _scope_path_allowed(path, allowed, protected)):
            continue
        rejected.append(path)
    if rejected:
        raise RunnerError("repository status contains out-of-scope changes: " + ", ".join(rejected))


def commands_from_wo(body: str) -> list[list[str]]:
    match = re.search(r"(?ms)^# Verification commands\s+.*?```text\n(.*?)\n```", body)
    if not match:
        raise RunnerError("work order has no Verification commands block")
    commands = [shlex.split(line) for line in match.group(1).splitlines() if line.strip()]
    if not commands:
        raise RunnerError("work order verification command list is empty")
    return commands


def run_checks(worktree: Path, run_dir: Path, body: str, timeout: float) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for index, command in enumerate(commands_from_wo(body), start=1):
        started = time.monotonic()
        result = subprocess.run(command, cwd=worktree, capture_output=True, text=True,
                                timeout=timeout, check=False)
        record = {"index": index, "argv": command, "exit_code": result.returncode,
                  "duration_s": time.monotonic() - started,
                  "stdout": result.stdout[-12000:], "stderr": result.stderr[-12000:]}
        results.append(record)
        (run_dir / f"check-{index:02d}.json").write_text(json.dumps(record, indent=2) + "\n")
        if result.returncode:
            raise RunnerError(f"deterministic verification command {index} failed")
    return results


def bounded_review_diff(root: Path, base: str, candidate: str, limit: int = 250_000) -> bytes:
    result = subprocess.run(["git", "diff", "--no-ext-diff", "--no-renames", base, candidate,
                             "--", ".", ":(exclude)artifacts/**/verifier-events*.jsonl",
                             ":(exclude)artifacts/**/verifier-stderr*.txt",
                             ":(exclude)artifacts/**/verdict*.json",
                             ":(exclude)artifacts/**/fresh-verifier*.txt",
                             ":(exclude)artifacts/**/verifier-precheck-failure.txt"],
                            cwd=root, capture_output=True, check=True).stdout
    if len(result) > limit:
        raise RunnerError("candidate diff exceeds verifier input limit")
    return result


def assert_verifier_scope(root: Path, wo_id: str, base: str, candidate: str) -> str:
    body = (root / "work" / f"{wo_id}.md").read_text(encoding="utf-8")
    start = base
    if (root / ".cache" / "worktrees" / f"{wo_id}.json").exists():
        start = worktree_start(root, wo_id, root / ".worktrees" / wo_id)
    ancestor_checks = ((base, start), (start, candidate))
    for older, newer in ancestor_checks:
        if subprocess.run(["git", "merge-base", "--is-ancestor", older, newer], cwd=root).returncode:
            raise RunnerError("verification candidate does not descend from the frozen task start")
    assert_scope(root, start, candidate, body, wo_id)
    return body


def run_verifier(root: Path, wo_id: str, fields: dict[str, str], spec_hash: str,
                 candidate: str, codex: str, timeout: float) -> dict[str, Any]:
    base = fields["base_sha"]
    body = assert_verifier_scope(root, wo_id, base, candidate)
    run_id = str(uuid.uuid4())
    run_dir = root / "artifacts" / "runs" / wo_id / f"verify-{run_id}"
    run_dir.mkdir(parents=True, exist_ok=False)
    worktree = root / ".worktrees" / f"verify-{wo_id}-{run_id}"
    worktree.parent.mkdir(parents=True, exist_ok=True)
    repo(root, "worktree", "add", "--detach", str(worktree), candidate)
    try:
        results = run_checks(worktree, run_dir, body, timeout)
        diff = bounded_review_diff(worktree, base, candidate)
        diff_path = run_dir / "candidate.diff"
        diff_path.write_bytes(diff)
        version, exec_help, _ = preflight(codex, evidence=run_dir / "codex-preflight.txt")
        missing = [flag for flag in VERIFY_REQUIRED if not _has_flag(exec_help, flag)]
        if missing:
            raise RunnerError("Codex CLI missing verifier capabilities: " + ", ".join(missing))
        report_path = run_dir / "verdict.json"
        schema = root / "docs/schemas/verifier-verdict.schema.json"
        cache_key = {"wo_id": wo_id, "revision": int(fields["revision"]), "spec_sha256": spec_hash,
                     "worktree_path": str(worktree), "base_sha": base}
        prompt = ("You are the fresh read-only verifier for this exact candidate. This invocation's "
                  "schema-valid verdict is the AC-6 artifact; do not fail solely because your own "
                  "verdict has not yet been saved. This review intentionally precedes ledger acceptance, "
                  "so pending STATE/LEDGER status is expected. The old revision-1 ledger sequence is a "
                  "terminal preflight-only block, documented in ADR-0003. Assess code and deterministic "
                  "checks for all other acceptance criteria. Return an adversarial review verdict and do "
                  "not modify files.\n\n" +
                  "WORK ORDER:\n" + body[:40_000] + "\nCHECK RESULTS:\n" +
                  json.dumps(results)[:30_000] + "\nDIFF:\n" + diff.decode("utf-8", errors="replace"))
        argv = [codex, "exec", "-m", "gpt-6-luna", "-c", "model_reasoning_effort=high",
                "-s", "read-only", "--json", "--output-schema", str(schema),
                "--output-last-message", str(report_path), "-"]
        code, duration, output = run_captured(argv, worktree, timeout, prompt,
                                               run_dir / "stdout.jsonl", run_dir / "stderr.log",
                                               root / ".cache/threads.jsonl", cache_key)
        if code:
            raise RunnerError(f"read-only verifier exited {code}")
        try:
            verdict = json.loads(report_path.read_text(encoding="utf-8"))
            schema_data = json.loads(schema.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise RunnerError(f"verifier report is missing or malformed: {exc}") from exc
        if not validate_json_schema(verdict, schema_data) or verdict["verdict"] != "pass":
            raise RunnerError("verifier verdict failed schema or reports findings")
        metadata = {"wo_id": wo_id, "revision": int(fields["revision"]), "spec_sha256": spec_hash,
                    "candidate_sha": candidate, "cli_version": version, "duration_s": duration,
                    "checks": results, "verdict": verdict, "session_id": parse_session_id(output)}
        (run_dir / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
        return metadata
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", str(worktree)], cwd=root,
                       capture_output=True, check=False)


def run_executor(args: argparse.Namespace, root: Path, fields: dict[str, str],
                 spec_hash: str, key: dict[str, Any]) -> dict[str, Any]:
    base = fields["base_sha"]
    lock_dir = root / ".cache" / "wo-locks"; lock_dir.mkdir(parents=True, exist_ok=True)
    with (lock_dir / f"{args.wo_id}.lock").open("a+b") as lock:
        try: fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc: raise RunnerError("work order is already locked") from exc
        if args.mode == "new":
            start_sha = require_clean_base(root, base); session = None
        else:
            session = args.identity or ""
            if not SESSION_RE.fullmatch(session) or not cache_matches(root, key, session):
                raise RunnerError("resume session does not match the frozen cache key")
            worktree = root / ".worktrees" / args.wo_id
            if not worktree.is_dir() or repo(worktree, "branch", "--show-current") != f"wo/{args.wo_id}":
                raise RunnerError("resume requires the known task branch and worktree")
            require_clean_task(worktree)
            start_sha = worktree_start(root, args.wo_id, worktree)
        run_id = str(uuid.uuid4()); run_dir = root / "artifacts" / "runs" / args.wo_id / run_id
        run_dir.mkdir(parents=True, exist_ok=False)
        version, _, _ = preflight(args.codex, evidence=run_dir / "codex-preflight.txt")
        if args.mode == "new":
            worktree = create_worktree(root, args.wo_id, start_sha)
        body = (root / "work" / f"{args.wo_id}.md").read_text(encoding="utf-8")
        schema = root / "docs/schemas/executor-report.schema.json"
        strict_schema = strict_executor_schema(
            json.loads(schema.read_text(encoding="utf-8")), acceptance_ids(body))
        strict_schema_path = run_dir / "executor-report.schema.json"
        strict_schema_path.write_text(json.dumps(strict_schema, indent=2) + "\n", encoding="utf-8")
        report_path = run_dir / "report.json"
        prompt = (f"Read work/{args.wo_id}.md and complete it. Run every acceptance command, "
                  f"commit only an accepted implementation, and write the required JSON report to {report_path}.")
        command = [args.codex, "exec"]
        if session: command += ["resume", session]
        command += ["-m", "gpt-6-luna", "-c", "model_reasoning_effort=high",
                    "--dangerously-bypass-approvals-and-sandbox", "--skip-git-repo-check",
                    "--json", "--output-schema", str(strict_schema_path),
                    "--output-last-message", str(report_path)]
        for image in args.image: command += ["--image", image]
        command += ["-"]
        stdout_path, stderr_path = run_dir / "stdout.jsonl", run_dir / "stderr.log"
        code, duration, output = run_captured(command, worktree, args.timeout, prompt,
                                               stdout_path, stderr_path, root / ".cache/threads.jsonl", key)
        if code != 0: raise RunnerError(f"Codex executor exited {code}")
        session = parse_session_id(output) or session
        if not session: raise RunnerError("executor JSONL did not provide a thread.started UUID")
        check_results = run_checks(worktree, run_dir, body, args.timeout)
        executed_commands = {tuple(item["argv"]) for item in check_results}
        report = validate_report(report_path, schema, base, acceptance_ids(body), executed_commands)
        head = repo(worktree, "rev-parse", "HEAD")
        if report["head_sha"] != head or repo(worktree, "status", "--porcelain"):
            raise RunnerError("reported head mismatch or candidate tree is dirty/uncommitted")
        if subprocess.run(["git", "merge-base", "--is-ancestor", base, head], cwd=worktree).returncode:
            raise RunnerError("candidate commit is not a descendant of base")
        changed = assert_scope(worktree, start_sha, head, body, args.wo_id)
        assert_root_scope(root, body)
        metadata = {"run_id": run_id, "wo_id": args.wo_id, "revision": int(fields["revision"]),
                    "spec_sha256": spec_hash, "base_sha": base, "head_sha": head, "session_id": session,
                    "model": "gpt-6-luna", "cli_version": version, "duration_s": duration,
                    "usage": {"input_tokens": None, "output_tokens": None}, "changed_paths": changed,
                    "executor_argv": command}
        (run_dir / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
        return metadata


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", nargs="?", choices=("new", "resume", "verify"))
    parser.add_argument("wo_id", nargs="?")
    parser.add_argument("identity", nargs="?")
    parser.add_argument("--codex", default="codex")
    parser.add_argument("--timeout", type=float, default=7200)
    parser.add_argument("--image", action="append", default=[])
    args = parser.parse_args(argv)
    if args.mode is None:
        parser.print_help(); return 0
    if not 0 < args.timeout <= 7200:
        parser.error("--timeout must be between 0 and 7200 seconds")
    try:
        if sys.platform != "darwin": raise RunnerError("work-order runners support macOS only")
        if not args.wo_id or not ID_RE.fullmatch(args.wo_id): raise RunnerError("invalid or missing WO ID")
        root = ROOT.resolve(); fields, spec_hash = load_wo(args.wo_id, root)
        if args.mode != "verify" and fields.get("status") != "READY": raise RunnerError("work order is not READY")
        base = fields["base_sha"]
        key = {"wo_id": args.wo_id, "revision": int(fields["revision"]), "spec_sha256": spec_hash,
               "worktree_path": str(root / ".worktrees" / args.wo_id), "base_sha": base}
        if args.mode == "verify":
            if not args.identity or not re.fullmatch(r"[0-9a-f]{40}", args.identity):
                raise RunnerError("verify requires a full candidate commit SHA")
            if subprocess.run(["git", "merge-base", "--is-ancestor", base, args.identity], cwd=root).returncode:
                raise RunnerError("candidate is not a descendant of the WO base")
            result = run_verifier(root, args.wo_id, fields, spec_hash, args.identity, args.codex, args.timeout)
        else:
            result = run_executor(args, root, fields, spec_hash, key)
        print(json.dumps(result, indent=2))
    except (OSError, subprocess.SubprocessError, ValidationError, RunnerError) as exc:
        print(f"runner error: {exc}", file=sys.stderr); return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
