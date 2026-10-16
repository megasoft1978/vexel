# W16 integration verification continuation (2026-10-11)

The W16 integration candidate is `integrate/w16` at `3030c99`. A stale merge marker from the already-integrated `lane/w14-rulers` merge was
left in the worktree metadata. The five former conflict paths had no unresolved
index entries; I cleared only the stale metadata with `git merge --quit`.

## Verification

- `./dev t --all`: build passed; 78/79 CTests; 69/69 Python tests.
- `./dev t --full`: 78/79 CTests; 66/66 Python tests.
- `./dev verify`: Debug build passed; Debug CTest 78/79. The full command
  stops at CTest before its Release and Python steps. The separately run
  `cmake --preset mac-release && cmake --build --preset mac-release` passed.
- `./dev asan`: 78/79 CTests; no sanitizer reports.
- `./dev ubsan`: 78/79 CTests; no sanitizer reports.

Each CTest run fails only `native-shell-goldens`, reporting that
`welcome-empty-light.png` differs. The committed image is 600x650; the native
renderer emits 1200x1300 on this host's 2x HiDPI main display. An explicit
logical-size bitmap experiment produced 600x650 but still differed in 3,100
RGB pixels around text edges, so I reverted it and left the renderer and baseline
unchanged. All other CTests passed, including native viewer smoke and index
replay. Python suite counts are shown above.

No real window was driven or captured. Maintainer real-window checks remain
listed in `docs/lanes/integrate.md`.
