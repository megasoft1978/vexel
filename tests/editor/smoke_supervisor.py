#!/usr/bin/env python3
"""Bound the editor UI smoke and clean up its full process group on failure."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
ACTION_SCRIPT = ROOT / "tests/editor/native_smoke_actions.sh"


class SmokeFailure(RuntimeError):
    pass


def timeout_seconds() -> float:
    value = os.environ.get("VEXEL_EDITOR_SMOKE_TIMEOUT")
    if value is None:
        return 120.0
    try:
        seconds = float(value)
    except ValueError as error:
        raise SmokeFailure("VEXEL_EDITOR_SMOKE_TIMEOUT must be a number of seconds") from error
    if not 1.0 <= seconds <= 600.0:
        raise SmokeFailure("VEXEL_EDITOR_SMOKE_TIMEOUT must be between 1 and 600 seconds")
    return seconds


def process_state(process: subprocess.Popen[bytes] | None) -> str:
    if process is None:
        return "not started"
    status = process.poll()
    return "running" if status is None else f"exited {status}"


def stop_process_group(process: subprocess.Popen[bytes] | None) -> None:
    if process is None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=2.0)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            pass
    else:
        # The shell can exit before a UI automation child does.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def diagnostic_dump(process: subprocess.Popen[bytes] | None, log_path: Path,
                    phase: str) -> str:
    details = [
        f"native editor smoke diagnostic: phase={phase}",
        f"runner pid={process.pid if process is not None else 'none'} "
        f"state={process_state(process)}",
    ]
    if process is not None:
        try:
            ps = subprocess.run(
                ["/bin/ps", "-o", "pid,ppid,pgid,state,etime,command", "-g", str(process.pid)],
                capture_output=True, text=True, check=False, timeout=2.0,
            )
            details.append("process group:\n" + (ps.stdout or ps.stderr).strip())
        except (OSError, subprocess.TimeoutExpired) as error:
            details.append(f"process group unavailable: {error}")
    try:
        log = log_path.read_text(encoding="utf-8", errors="replace")
        details.append("smoke log tail:\n" + "\n".join(log.splitlines()[-80:]))
    except OSError as error:
        details.append(f"smoke log unavailable: {error}")
    details.append(f"smoke log retained at: {log_path}")
    return "\n".join(details)


def run_smoke(arguments: list[str]) -> None:
    if len(arguments) > 1:
        raise SmokeFailure("usage: native_smoke.sh [<binary>]")
    limit = timeout_seconds()
    descriptor, raw_log_path = tempfile.mkstemp(prefix="vexel-editor-smoke-", suffix=".log")
    os.close(descriptor)
    log_path = Path(raw_log_path)
    command_arguments = [str(Path(arguments[0]).expanduser().resolve())] if arguments else []
    command = ["/bin/sh", str(ACTION_SCRIPT), *command_arguments]
    process: subprocess.Popen[bytes] | None = None
    phase = "launch"
    try:
        with log_path.open("wb") as log:
            process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT,
                                        cwd=ROOT, start_new_session=True)
            phase = "interactive editor smoke"
            try:
                status = process.wait(timeout=limit)
            except subprocess.TimeoutExpired as error:
                raise SmokeFailure(f"editor smoke timed out after {limit:g} seconds") from error
            if status != 0:
                raise SmokeFailure(f"editor smoke exited with status {status}")
    except BaseException as error:
        report = diagnostic_dump(process, log_path, phase)
        stop_process_group(process)
        if isinstance(error, KeyboardInterrupt):
            raise
        raise SmokeFailure(f"{error}\n{report}") from error
    else:
        try:
            output = log_path.read_text(encoding="utf-8", errors="replace")
            log_path.unlink()
        except OSError as error:
            raise SmokeFailure(f"editor smoke passed but its log could not be read: {error}") from error
        sys.stdout.write(output)


def main(arguments: list[str]) -> int:
    try:
        run_smoke(arguments)
    except SmokeFailure as error:
        print(str(error), file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("native editor smoke interrupted; its process group was stopped", file=sys.stderr)
        return 130
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
