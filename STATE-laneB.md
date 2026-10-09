# Lane B state

- Complete: WO-M6-01 retained UI layout/paint and screenshot-golden editor chrome.
- Evidence: `bench/results/WO-M6-01.json`; dark/light 1280x800 golden PNGs in `tests/ui/chrome-golden/`.
- Verification: Debug and Release builds passed; `./dev asan` and `./dev ubsan` passed 14/14 CTests each. Debug CTest passed 14/14, `./dev test-fast` passed 200/200, and ASan/UBSan CTest passed 14/14 each. The final combined `./dev verify` passed: CTest 14/14 and Python 200/200. Earlier runs exposed transient native-cycle and benchmark-timeout failures; isolated retries passed.
- Performance: Release `ui_demo_render` 105,280-byte Release binary; 1280x800 dark chrome median 9.699 ms and peak child RSS 6,275,072 bytes (5 fresh processes).
- Next: M6-02 focus, keyboard dispatch, command registry, and semantics tree; retain existing frame and demo interfaces.
