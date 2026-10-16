ACCEPT

The source-limit branch now reports `InputLimit` at offset `0`, matching C’s zero-initialized diagnostic. The 16 MiB cap remains in place, and the corresponding test now expects `0`.

The archived prior-candidate test retains the erroneous `MAX_INPUT_BYTES` expectation. Comparing it with the current test shows only that offset assertion changed; the other eight tests and assertions are unchanged. The current parser still builds fallibly reserved, bounded owned node and attribute tables.

The approved XML files still match the coordinator delta’s post-change hashes. The latest verification artifact reports all 9 tests passed. I did not run tests or builds.