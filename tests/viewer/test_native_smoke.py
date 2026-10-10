"""Regression tests for the UI-script-free native viewer smoke runner."""

from __future__ import annotations

import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest import mock

from tests.viewer import native_smoke


class _FinishedProcess:
    pid = 8123

    def __init__(self) -> None:
        self.returncode: int | None = None

    def poll(self) -> int | None:
        return self.returncode

    def wait(self, timeout: float | None = None) -> int:
        self.returncode = 0
        return 0


class ViewerSmokeRunnerTests(unittest.TestCase):
    def test_main_passes_the_requested_open_file_to_the_smoke(self) -> None:
        binary = Path("/tmp/svgworkbench")
        document = Path("/tmp/second.svg")
        with mock.patch.object(native_smoke, "run_smoke") as run_smoke:
            self.assertEqual(native_smoke.main([str(binary), str(document)]), 0)
        run_smoke.assert_called_once_with(binary.resolve(), document.resolve())

    def test_smoke_uses_native_injection_and_sets_test_open(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "svgworkbench"
            document = Path(directory) / "document.svg"
            binary.touch()
            document.write_text("<svg/>", encoding="utf-8")
            process = _FinishedProcess()
            with mock.patch.object(native_smoke.subprocess, "Popen", return_value=process) as popen:
                native_smoke.run_smoke(binary, document)

        command, = popen.call_args.args
        options = popen.call_args.kwargs
        self.assertEqual(command, [str(binary), "--native-viewer-smoke"])
        self.assertEqual(options["env"]["VEXEL_TEST_OPEN"], str(document))
        self.assertTrue(options["start_new_session"])
        self.assertNotIn("osascript", command)

    def test_test_open_path_is_bounded(self) -> None:
        path = Path("/" + "x" * 4096)
        with self.assertRaisesRegex(native_smoke.SmokeFailure, "1 and 4095 bytes"):
            native_smoke.run_smoke(Path("/missing/svgworkbench"), path)

    def test_timeout_stops_the_smoke_process_group(self) -> None:
        class TimedOutProcess(_FinishedProcess):
            def wait(self, timeout: float | None = None) -> int:
                if timeout == 1.0:
                    raise subprocess.TimeoutExpired("svgworkbench", timeout)
                self.returncode = -signal.SIGTERM
                return self.returncode

        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "svgworkbench"
            document = Path(directory) / "document.svg"
            binary.touch()
            document.touch()
            process = TimedOutProcess()
            with mock.patch.dict(os.environ, {"VEXEL_VIEWER_SMOKE_TIMEOUT": "1"}), \
                 mock.patch.object(native_smoke.subprocess, "Popen", return_value=process), \
                 mock.patch.object(native_smoke.subprocess, "run",
                                   return_value=subprocess.CompletedProcess([], 0, "", "")), \
                 mock.patch.object(native_smoke.os, "killpg") as killpg:
                with self.assertRaises(native_smoke.SmokeFailure):
                    native_smoke.run_smoke(binary, document)

        self.assertEqual(killpg.call_args_list,
                         [mock.call(process.pid, signal.SIGTERM),
                          mock.call(process.pid, signal.SIGKILL)])


if __name__ == "__main__":
    unittest.main()
