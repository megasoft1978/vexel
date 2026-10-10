#!/usr/bin/env python3
"""Bounded native viewer interaction smoke for an interactive macOS session."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BINARY = ROOT / "build/mac-debug/bin/svgworkbench"
DEFAULT_FIRST = ROOT / "tests/viewer/fixtures/simple-shape.svg"
DEFAULT_SECOND = ROOT / "tests/viewer/fixtures/unsupported.svg"
APPLE_SCRIPT = r'''on run argv
    set targetPid to (item 1 of argv) as integer
    set secondDocument to item 2 of argv
    set waitAttempts to (item 3 of argv) as integer
    tell application "System Events"
        set targetProcess to missing value
        repeat waitAttempts times
            try
                set targetProcess to first process whose unix id is targetPid
                if (count of windows of targetProcess) > 0 then exit repeat
            end try
            delay 0.1
        end repeat
        if targetProcess is missing value then error "viewer process did not become visible"
        tell targetProcess
            set frontmost to true
            if (count of windows) = 0 then error "viewer window did not open"

            keystroke "r" using {command down}
            delay 0.3
            keystroke "+"
            delay 0.3
            keystroke "t"
            delay 0.3
            keystroke "o" using {command down}

            set openPanelReady to false
            repeat 100 times
                try
                    if exists sheet 1 of window 1 then
                        set openPanelReady to true
                        exit repeat
                    end if
                end try
                delay 0.1
            end repeat
            if not openPanelReady then error "Open panel sheet did not appear"

            keystroke "g" using {command down, shift down}
            delay 0.3
            keystroke secondDocument
            key code 36
            delay 0.4
            key code 36

            set panelClosed to false
            repeat 100 times
                try
                    if not (exists sheet 1 of window 1) then
                        set panelClosed to true
                        exit repeat
                    end if
                on error
                    set panelClosed to true
                    exit repeat
                end try
                delay 0.1
            end repeat
            if not panelClosed then error "Open panel sheet did not close after selection"

            set frontmost to true
            delay 0.2
            keystroke "1"
            delay 0.3
            keystroke "q" using {command down}
        end tell
    end tell
end run
'''


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


def diagnostic_dump(viewer: subprocess.Popen[bytes] | None,
                    automation: subprocess.Popen[bytes] | None,
                    log_path: Path, phase: str, automation_output: str = "") -> str:
    details = [
        f"native viewer smoke diagnostic: phase={phase}",
        f"viewer pid={viewer.pid if viewer is not None else 'none'} state={process_state(viewer)}",
        f"osascript pid={automation.pid if automation is not None else 'none'} state={process_state(automation)}",
    ]
    if viewer is not None or automation is not None:
        pids = [str(process.pid) for process in (viewer, automation) if process is not None]
        try:
            ps = subprocess.run(["/bin/ps", "-o", "pid,ppid,state,etime,command", "-p",
                                 ",".join(pids)], capture_output=True, text=True,
                                check=False, timeout=2.0)
            details.append("process table:\n" + (ps.stdout or ps.stderr).strip())
        except (OSError, subprocess.TimeoutExpired) as error:
            details.append(f"process table unavailable: {error}")
    if automation_output:
        details.append("osascript output:\n" + automation_output[-8000:])
    try:
        log = log_path.read_text(encoding="utf-8", errors="replace")
        details.append("viewer log tail:\n" + "\n".join(log.splitlines()[-80:]))
    except OSError as error:
        details.append(f"viewer log unavailable: {error}")
    details.append(f"viewer log retained at: {log_path}")
    return "\n".join(details)


def stop_process_group(process: subprocess.Popen[bytes] | None) -> None:
    if process is None or process.poll() is not None:
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


def run_smoke(binary: Path, first: Path, second: Path) -> None:
    startup_limit = timeout_seconds("VEXEL_VIEWER_SMOKE_START_TIMEOUT", 15.0)
    automation_limit = timeout_seconds("VEXEL_VIEWER_SMOKE_AUTOMATION_TIMEOUT", 35.0)
    quit_limit = timeout_seconds("VEXEL_VIEWER_SMOKE_QUIT_TIMEOUT", 10.0)
    for name, path in (("viewer binary", binary), ("first SVG", first), ("second SVG", second)):
        if not path.is_file():
            raise SmokeFailure(f"{name} does not exist: {path}")

    descriptor, raw_log_path = tempfile.mkstemp(prefix="vexel-viewer-smoke-", suffix=".log")
    os.close(descriptor)
    log_path = Path(raw_log_path)
    viewer: subprocess.Popen[bytes] | None = None
    automation: subprocess.Popen[bytes] | None = None
    automation_output = ""
    phase = "launch"
    try:
        with log_path.open("wb") as log:
            viewer = subprocess.Popen([str(binary), str(first)], stdout=log,
                                      stderr=subprocess.STDOUT, start_new_session=True)
            if viewer.poll() is not None:
                raise SmokeFailure("viewer exited before automation began")

            phase = "automation"
            automation = subprocess.Popen(["/usr/bin/osascript", "-", str(viewer.pid),
                str(second), str(max(1, int(startup_limit * 10.0)))],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, start_new_session=True)
            try:
                output, _ = automation.communicate(APPLE_SCRIPT.encode("utf-8"),
                    timeout=startup_limit + automation_limit)
                automation_output = output.decode("utf-8", errors="replace")
            except subprocess.TimeoutExpired as timeout:
                partial = timeout.output or b""
                automation_output = partial.decode("utf-8", errors="replace")
                raise SmokeFailure("osascript automation timed out")
            if automation.returncode != 0:
                raise SmokeFailure(f"osascript exited with status {automation.returncode}")
            if viewer.poll() is not None:
                raise SmokeFailure("viewer exited before Cmd+Q completed")

            phase = "quit"
            try:
                exit_status = viewer.wait(timeout=quit_limit)
            except subprocess.TimeoutExpired as error:
                raise SmokeFailure("viewer did not quit after Cmd+Q") from error
            if exit_status != 0:
                raise SmokeFailure(f"viewer exited with status {exit_status}")
    except BaseException as error:
        report = diagnostic_dump(viewer, automation, log_path, phase,
                                 automation_output)
        stop_process_group(automation)
        stop_process_group(viewer)
        if isinstance(error, KeyboardInterrupt):
            raise
        raise SmokeFailure(f"{error}\n{report}") from error
    else:
        if automation is not None and automation.poll() is None:
            stop_process_group(automation)
        try:
            log_path.unlink()
        except OSError:
            pass
        print("native viewer smoke: open panel, reload, zoom, actual size, quit passed")


def main(arguments: list[str]) -> int:
    if len(arguments) > 3:
        print("usage: native_smoke.sh [<binary> [<first.svg> [<second.svg>]]]", file=sys.stderr)
        return 2
    paths = [Path(value).expanduser().resolve() for value in arguments]
    binary = paths[0] if len(paths) >= 1 else DEFAULT_BINARY
    first = paths[1] if len(paths) >= 2 else DEFAULT_FIRST
    second = paths[2] if len(paths) >= 3 else DEFAULT_SECOND
    try:
        run_smoke(binary, first, second)
    except SmokeFailure as error:
        print(str(error), file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("native viewer smoke interrupted; started processes were stopped", file=sys.stderr)
        return 130
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
