# Lane B ledger

| Date | Work order | Status | Commit | Evidence |
|---|---|---|---|---|
| 2026-10-09 | WO-M6-01 | complete | `lane/B-M6-01` | Debug/Release builds; `./dev test-fast` 200/200; ASan/UBSan/Debug CTest 14/14 each; dark/light 1280x800 golden screenshots; Release render median 9.699 ms and peak RSS 6,275,072 bytes. Final combined `./dev verify` passed CTest 14/14 and Python 200/200; earlier transient retries are recorded in STATE-laneB.md. |
| 2026-10-09 | WO-M6-02 | complete | `lane/B-M6-02` | Debug/Release builds; `./dev verify` CTest 16/16 + Python 200/200; ASan/UBSan CTest 16/16 each; keyboard/semantics tests; focus median 39.6 ns across 5M moves on 128 nodes, RSS 1,916,928 B. |
