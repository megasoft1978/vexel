#!/usr/bin/env python3
"""Run the bounded, UI-script-free native viewer interaction smoke."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BINARY = ROOT / "build/mac-debug/bin/svgworkbench"
DEFAULT_OPEN = ROOT / "tests/viewer/fixtures/simple-shape.svg"


class SmokeFailure(RuntimeError):
    pass


def timeout_seconds(name: str, fallback: float) -> float:
    value = os.environ.get(name)
    if value is None:
        return fallback
    try:
        seconds = float(value)
    except ValueError as error:
        raise SmokeFailure(f"{name} must be a number of seconds") from error
    if not 1.0 <= seconds <= 300.0:
        raise SmokeFailure(f"{name} must be between 1 and 300 seconds")
    return seconds


def process_state(process: subprocess.Popen[bytes] | None) -> str:
    if process is None:
        return "not started"
    status = process.poll()
    return "running" if status is None else f"exited {status}"


def diagnostic_dump(process: subprocess.Popen[bytes] | None, log_path: Path,
                    phase: str) -> str:
    details = [
        f"native viewer smoke diagnostic: phase={phase}",
        f"viewer pid={process.pid if process is not None else 'none'} "
        f"state={process_state(process)}",
    ]
    if process is not None:
        try:
            ps = subprocess.run(
                ["/bin/ps", "-o", "pid,ppid,state,etime,command", "-p", str(process.pid)],
                capture_output=True, text=True, check=False, timeout=2.0,
            )
            details.append("process table:\n" + (ps.stdout or ps.stderr).strip())
        except (OSError, subprocess.TimeoutExpired) as error:
            details.append(f"process table unavailable: {error}")
    try:
        log = log_path.read_text(encoding="utf-8", errors="replace")
        details.append("viewer log tail:\n" + "\n".join(log.splitlines()[-80:]))
    except OSError as error:
        details.append(f"viewer log unavailable: {error}")
    details.append(f"viewer log retained at: {log_path}")
    return "\n".join(details)


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
        # The group leader may exit while a child process is still alive.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def run_smoke(binary: Path, document: Path) -> None:
    limit = timeout_seconds("VEXEL_VIEWER_SMOKE_TIMEOUT", 30.0)
    encoded_path = os.fsencode(document)
    if not encoded_path or len(encoded_path) >= 4096:
        raise SmokeFailure("test-open SVG path must be between 1 and 4095 bytes")
    if not binary.is_file():
        raise SmokeFailure(f"viewer binary does not exist: {binary}")
    if not document.is_file():
        raise SmokeFailure(f"test-open SVG does not exist: {document}")
    if document.suffix.lower() != ".svg":
        raise SmokeFailure(f"test-open path must end in .svg: {document}")

    descriptor, raw_log_path = tempfile.mkstemp(prefix="vexel-viewer-smoke-", suffix=".log")
    os.close(descriptor)
    log_path = Path(raw_log_path)
    environment = os.environ.copy()
    environment["VEXEL_TEST_OPEN"] = str(document)
    process: subprocess.Popen[bytes] | None = None
    phase = "launch"
    try:
        with log_path.open("wb") as log:
            process = subprocess.Popen(
                [str(binary), "--native-viewer-smoke"],
                stdout=log,
                stderr=subprocess.STDOUT,
                env=environment,
                start_new_session=True,
            )
            phase = "native interaction"
            try:
                status = process.wait(timeout=limit)
            except subprocess.TimeoutExpired as error:
                raise SmokeFailure(f"native viewer smoke timed out after {limit:g} seconds") from error
            if status != 0:
                raise SmokeFailure(f"native viewer smoke exited with status {status}")
    except BaseException as error:
        report = diagnostic_dump(process, log_path, phase)
        stop_process_group(process)
        if isinstance(error, KeyboardInterrupt):
            raise
        raise SmokeFailure(f"{error}\n{report}") from error
    else:
        try:
            log_path.unlink()
        except OSError:
            pass
        print("native viewer smoke: test-open, reload, zoom, fit, actual size, and quit passed")


def main(arguments: list[str]) -> int:
    if len(arguments) > 2:
        print("usage: native_smoke.sh [<binary> [<file.svg>]]", file=sys.stderr)
        return 2
    binary = Path(arguments[0]).expanduser().resolve() if arguments else DEFAULT_BINARY
    document = Path(arguments[1]).expanduser().resolve() if len(arguments) == 2 else DEFAULT_OPEN
    try:
        run_smoke(binary, document)
    except SmokeFailure as error:
        print(str(error), file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("native viewer smoke interrupted; started processes were stopped", file=sys.stderr)
        return 130
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
