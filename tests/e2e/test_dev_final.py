import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class DevFinalCommandTests(unittest.TestCase):
    def test_shape_fuzz_step_supplies_its_required_seed(self):
        script = (ROOT / "dev").read_text()
        final_block = script.split("            final)\n", 1)[1].split("                ;;\n", 1)[0]
        match = re.search(r'"\$repo_root/dev" shape-fuzz([^\n]*)', final_block)

        self.assertIsNotNone(match, "final gate must run shape-fuzz")
        self.assertRegex(match.group(1), r"(?:^|\s)--seed(?:\s|$)")


if __name__ == "__main__":
    unittest.main()
