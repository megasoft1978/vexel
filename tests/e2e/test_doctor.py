"""Unit tests for prerequisite reporting."""

from __future__ import annotations

import unittest
from unittest.mock import patch

from scripts import doctor


class DoctorTests(unittest.TestCase):
    def test_argument_validation_rejects_extra_arguments(self) -> None:
        doctor.parse_args([])
        with self.assertRaises(SystemExit):
            doctor.parse_args(["--unknown"])

    def test_reports_each_required_tool_and_marks_missing_prerequisites(self) -> None:
        def locate(executable: str) -> str | None:
            return None if executable == "ninja" else f"/tools/{executable}"

        with patch.object(doctor.sys, "platform", "darwin"), \
                patch.object(doctor.platform, "machine", return_value="arm64"), \
                patch.object(doctor.platform, "mac_ver", return_value=("14.0", "", "")), \
                patch.object(doctor.shutil, "which", side_effect=locate), \
                patch.object(doctor, "_version", return_value="test version"):
            status, lines = doctor.run_doctor()

        self.assertEqual(status, 1)
        self.assertTrue(any(line.startswith("OK cmake:") for line in lines))
        self.assertTrue(any(line.startswith("MISSING ninja:") for line in lines))
        self.assertTrue(any("rsvg-convert" in line for line in lines))


if __name__ == "__main__":
    unittest.main()
