"""Reject hand-counted byte lengths that follow a string literal in src/app."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PAIR = re.compile(r'"((?:[^"\\\n]|\\.)*)"\s*,\s*(\d+)U?\b')
ESCAPES = {"n": 1, "t": 1, "r": 1, "0": 1, "\\": 1, '"': 1, "'": 1}


def literal_bytes(text):
    """UTF-8 byte length of a C literal body, or None for escapes not modelled."""
    out, i = 0, 0
    while i < len(text):
        if text[i] == "\\":
            if i + 1 >= len(text) or text[i + 1] not in ESCAPES:
                return None
            out, i = out + 1, i + 2
        else:
            out, i = out + len(text[i].encode("utf-8")), i + 1
    return out


class LiteralLengthTests(unittest.TestCase):
    def test_no_hand_counted_length_disagrees_with_literal(self):
        bad = []
        for path in sorted((ROOT / "src/app").glob("*.c")):
            for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                for match in PAIR.finditer(line):
                    expected = literal_bytes(match.group(1))
                    given = int(match.group(2))
                    # memcpy of a literal legitimately includes its NUL; any
                    # other length past the literal is an over-read.
                    allowed = expected + 1 if expected is not None and "memcpy(" in line else expected
                    if expected is not None and given > allowed:
                        bad.append(f"{path.name}:{lineno}: \"{match.group(1)}\" len {given} > {allowed}")
        self.assertEqual(bad, [])


if __name__ == "__main__":
    unittest.main()
