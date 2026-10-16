**ACCEPT.** The repaired over-limit branch returns `InputLimit` with a default parse diagnostic and native result flags. Its test asserts `None`/offset 0 and untouched hooks; the empty-source test asserts `Tokenizer`/`IncompleteDocument` at offset 0 with all adapter gates untouched.

The current legacy source and header hashes match the recorded source baseline. The inherited C fixture hash also matches `0f32aa5b026962b38df47be97ceb367506292a5d4f14aa16115092dd2497c7a8`; I did not use the Git HEAD diff as a preservation baseline or restore the fixture.

I found no remaining scoped defect in the repaired code, tests, parity map, or verification record. I ran no tests or builds. This does not certify the full application.