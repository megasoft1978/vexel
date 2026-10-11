# WO-M2-02 verification history (resolved)

Three `./dev verify` attempts inside the executor's nested workspace-write
sandbox reported 136 passed, 8 failed, and 2 errors because existing
`sandbox-exec` probes returned 71 and native LaunchServices/XPC checks failed.
The same command then passed repeatedly from the host shell without changing
frozen tests; see `verify-host.txt`.

The first clean verifier found BOM handling, hard-limit enforcement, and QName
span assertions missing. The second found adjacent attributes without required
whitespace, invalid QName local starts, and incomplete span-range checks in
the deterministic malformed generator. All findings are fixed and regression
tests were added. Latest host suite, ASan/UBSan, Expat, and 60-second ASan fuzz
evidence is in this directory. The third fresh review identified an output-path escape; the fuzz driver now confines resolved destinations and tests reject traversal and absolute paths. The fourth review identified PI target/data handling and incomplete start-tag spans; these are fixed with regressions. The character/UTF-8 helpers were moved to their own source module. The 147-test host suite and all sanitizer/fuzz checks pass; final review is pending.
