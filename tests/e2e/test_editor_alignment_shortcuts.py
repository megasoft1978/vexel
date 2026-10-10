import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class EditorAlignmentShortcutTests(unittest.TestCase):
    def test_window_routes_arrange_shortcuts_to_alignment(self):
        source = (ROOT / "src/app/viewer_events.c").read_text()

        self.assertRegex(source, r"static bool editor_alignment_shortcut\(")
        for key_code, alignment in (
            (123, "LEFT"),
            (124, "RIGHT"),
            (126, "TOP"),
            (125, "BOTTOM"),
            (4, "HORIZONTAL_CENTER"),
            (9, "VERTICAL_CENTER"),
        ):
            self.assertRegex(
                source,
                rf"case {key_code}U:\s*\*out_alignment\s*=\s*"
                rf"VIEWER_EDITOR_ALIGN_{alignment};",
            )
        self.assertRegex(
            source,
            r"viewer_editor_align_selection\(app->editor,\s*&app->document\.shape,\s*"
            r"alignment\)",
        )


if __name__ == "__main__":
    unittest.main()
