"""Tests for bounded editor-smoke process-group cleanup."""

from __future__ import annotations

import os
import signal
import subprocess
import unittest
from unittest import mock

from tests.editor import smoke_supervisor


class _TimedOutProcess:
    pid = 8317

    def __init__(self) -> None:
        self.returncode: int | None = None

    def poll(self) -> int | None:
        return self.returncode

    def wait(self, timeout: float | None = None) -> int:
        if timeout == 1.0:
            raise subprocess.TimeoutExpired("editor-smoke", timeout)
        self.returncode = -signal.SIGTERM
        return self.returncode


class EditorSmokeSupervisorTests(unittest.TestCase):
    def test_timeout_terminates_the_whole_smoke_process_group(self) -> None:
        process = _TimedOutProcess()
        with mock.patch.dict(os.environ, {"VEXEL_EDITOR_SMOKE_TIMEOUT": "1"}), \
             mock.patch.object(smoke_supervisor.subprocess, "Popen", return_value=process) as popen, \
             mock.patch.object(smoke_supervisor.subprocess, "run",
                               return_value=subprocess.CompletedProcess([], 0, "", "")), \
             mock.patch.object(smoke_supervisor.os, "killpg") as killpg:
            with self.assertRaises(smoke_supervisor.SmokeFailure):
                smoke_supervisor.run_smoke([])

        self.assertEqual(popen.call_args.args[0][0:2],
                         ["/bin/sh", str(smoke_supervisor.ACTION_SCRIPT)])
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        self.assertEqual(killpg.call_args_list,
                         [mock.call(process.pid, signal.SIGTERM),
                          mock.call(process.pid, signal.SIGKILL)])

    def test_timeout_rejects_unbounded_values(self) -> None:
        with mock.patch.dict(os.environ, {"VEXEL_EDITOR_SMOKE_TIMEOUT": "601"}):
            with self.assertRaisesRegex(smoke_supervisor.SmokeFailure, "between 1 and 600"):
                smoke_supervisor.timeout_seconds()


if __name__ == "__main__":
    unittest.main()
