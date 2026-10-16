#!/usr/bin/env python3
"""Gate `cargo fmt --check` by an explicit per-crate list.

`rust/ci/fmt-clean.txt` lists workspace members that must stay rustfmt-clean;
`rust/ci/fmt-debt.txt` lists members with known formatting drift, to be fixed
by a later mechanical order. Every workspace member must be in exactly one
list, so a new crate has to be classified. Each clean-listed crate must pass
`cargo fmt -p <crate> -- --check`.

Usage: check_rust_fmt.py [--root DIR] [--list-only]   (check the repository)
       check_rust_fmt.py --self-test                  (run the built-in tests)
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parents[1]
CLEAN = Path("rust/ci/fmt-clean.txt")
DEBT = Path("rust/ci/fmt-debt.txt")

FmtRunner = Callable[[str, Path], tuple[bool, str]]


class FmtPolicyError(ValueError):
    """Raised when the fmt lists or the formatting of a clean crate are wrong."""


def read_list(path: Path) -> list[str]:
    names = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            names.append(line)
    return names


def workspace_members(root: Path) -> list[str]:
    """Package names of every workspace member, from the member Cargo.toml files."""
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    block = re.search(r"^members\s*=\s*\[(?P<body>.*?)\]", manifest, re.MULTILINE | re.DOTALL)
    if block is None:
        raise FmtPolicyError("Cargo.toml has no workspace members list")
    names = []
    for member in re.findall(r'"([^"]+)"', block["body"]):
        package = (root / member / "Cargo.toml").read_text(encoding="utf-8")
        match = re.search(r'^name\s*=\s*"([^"]+)"', package, re.MULTILINE)
        if match is None:
            raise FmtPolicyError(f"{member}/Cargo.toml has no package name")
        names.append(match[1])
    return sorted(names)


def check_partition(members: list[str], clean: list[str], debt: list[str]) -> None:
    problems = []
    for name in sorted({n for n in clean if clean.count(n) > 1} | {n for n in debt if debt.count(n) > 1}):
        problems.append(f"{name} is listed more than once")
    for name in sorted(set(clean) & set(debt)):
        problems.append(f"{name} is in both fmt-clean.txt and fmt-debt.txt")
    for name in members:
        if name not in clean and name not in debt:
            problems.append(f"unclassified workspace member {name}: add it to {CLEAN} or {DEBT}")
    for name in sorted((set(clean) | set(debt)) - set(members)):
        problems.append(f"{name} is listed but is not a workspace member")
    if problems:
        raise FmtPolicyError("; ".join(problems))


def cargo_fmt_check(crate: str, root: Path) -> tuple[bool, str]:
    result = subprocess.run(
        ["cargo", "fmt", "-p", crate, "--", "--check"],
        cwd=root, capture_output=True, text=True,
    )
    return result.returncode == 0, (result.stdout + result.stderr).strip()


def check_repository(root: Path, runner: FmtRunner = cargo_fmt_check, run_fmt: bool = True) -> list[str]:
    members = workspace_members(root)
    clean = read_list(root / CLEAN)
    debt = read_list(root / DEBT)
    check_partition(members, clean, debt)
    if not run_fmt:
        return clean
    drifting = []
    for crate in clean:
        ok, output = runner(crate, root)
        if not ok:
            first = output.splitlines()[0] if output else "no output"
            drifting.append(f"{crate} drifts from rustfmt ({first})")
    if drifting:
        raise FmtPolicyError(
            "; ".join(drifting) + f". Run `cargo fmt -p <crate>` or move it to {DEBT}.")
    return clean


class SelfTest(unittest.TestCase):
    def fixture(self, members: list[str], clean: list[str], debt: list[str]) -> Path:
        tmp = Path(tempfile.mkdtemp(prefix="check-rust-fmt-"))
        self.addCleanup(lambda: __import__("shutil").rmtree(tmp, ignore_errors=True))
        paths = []
        for name in members:
            (tmp / "rust" / "crates" / name).mkdir(parents=True)
            (tmp / "rust" / "crates" / name / "Cargo.toml").write_text(f'[package]\nname = "{name}"\n')
            paths.append(f'"rust/crates/{name}"')
        (tmp / "Cargo.toml").write_text("[workspace]\nmembers = [\n  " + ",\n  ".join(paths) + "\n]\n")
        (tmp / "rust" / "ci").mkdir(parents=True)
        (tmp / CLEAN).write_text("# clean\n" + "".join(f"{n}\n" for n in clean))
        (tmp / DEBT).write_text("# debt\n" + "".join(f"{n}  # known\n" for n in debt))
        return tmp

    def test_exact_partition_passes_and_formats_only_clean_crates(self) -> None:
        root = self.fixture(["a", "b", "c"], ["a", "b"], ["c"])
        seen = []
        result = check_repository(root, lambda crate, _: (seen.append(crate) or True, ""))
        self.assertEqual(result, ["a", "b"])
        self.assertEqual(seen, ["a", "b"])

    def test_unclassified_member_fails_with_its_name(self) -> None:
        root = self.fixture(["a", "new-crate"], ["a"], [])
        with self.assertRaisesRegex(FmtPolicyError, "unclassified workspace member new-crate"):
            check_repository(root, lambda *_: (True, ""))

    def test_drifting_clean_crate_fails_with_its_name(self) -> None:
        root = self.fixture(["a", "b"], ["a", "b"], [])
        with self.assertRaisesRegex(FmtPolicyError, "b drifts from rustfmt"):
            check_repository(root, lambda crate, _: (crate != "b", "Diff in b/src/lib.rs"))

    def test_duplicate_and_stale_entries_fail(self) -> None:
        both = self.fixture(["a"], ["a"], ["a"])
        with self.assertRaisesRegex(FmtPolicyError, "a is in both"):
            check_repository(both, lambda *_: (True, ""))
        stale = self.fixture(["a"], ["a", "gone"], [])
        with self.assertRaisesRegex(FmtPolicyError, "gone is listed but is not a workspace member"):
            check_repository(stale, lambda *_: (True, ""))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--list-only", action="store_true", help="check the lists without running cargo fmt")
    parser.add_argument("--self-test", action="store_true", help="run the built-in fixture tests")
    args = parser.parse_args(sys.argv[1:] if argv is None else argv)
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTest)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    try:
        clean = check_repository(args.root, run_fmt=not args.list_only)
    except (OSError, FmtPolicyError) as exc:
        print(f"rust fmt policy error: {exc}", file=sys.stderr)
        return 1
    print(f"rust fmt policy: pass ({len(clean)} clean-listed crates)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
