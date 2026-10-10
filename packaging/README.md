# macOS application packaging

`vexel_packaging.py` creates `build/Vexel.app` and `build/Vexel.dmg` using only
Python's standard library and Apple command-line tools. The source binary is
the CMake Release target; the CMake project version populates the bundle plist.

The placeholder icon is drawn as PNG representations and assembled into a
PNG-backed `.icns` container without downloaded assets or a conversion
dependency. Signing uses ad-hoc identity `-` by default and enables the
hardened runtime. See [`docs/release.md`](../docs/release.md) for Developer ID,
notarization, and verification details.
