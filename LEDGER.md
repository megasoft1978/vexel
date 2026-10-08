# Work ledger

| UTC date | Work order | Status | Base | Commit | Evidence |
|---|---|---|---|---|---|
| 2026-10-08 | WO-BOOT-01 | complete; fresh verifier pass | `b90dfbc44623e9834852b19513116149c83a15f0` | `beb0cdd` | Debug/Release/size/ASan-UBSan build; six ACs pass; `artifacts/WO-BOOT-01/`, `bench/results/WO-BOOT-01.json` |
| 2026-10-08 | WO-BOOT-02 | complete; fresh verifier pass | `2d8e1ea60356d418d7a69cb6c926eacad7f16d08` | `278c227` | 8 tests pass; atomic append validated; `artifacts/WO-BOOT-02/` |
| 2026-10-08 | WO-BOOT-03 | complete; fresh verifier pass | `4cc8bfbab2ecdda7c4cd72354f99cb28c08ab477` | `44cce8f` | 14 tests pass; actionlint/policy checks pass; hosted CI pending (no remote); `artifacts/WO-BOOT-03/` |
| 2026-10-08 | WO-BOOT-04 r2 | complete; fresh verifier pass; merged | `837e88acdc247c790b0b9785efed98f544584fcb` | `05dc456` | 37 targeted tests; fresh verifier reran seven checks and 43 full-suite tests; `artifacts/WO-BOOT-04/` |
| 2026-10-08 | WO-BOOT-05 r1 | blocked before model turn | `0bc1cfdc9f990878f087dfff936d12fb175f4678` | — | Codex strict JSON output rejected `minProperties`; attempt evidence retained under `artifacts/WO-BOOT-05/runner-attempt1/` |
| 2026-10-08 | WO-BOOT-05 r2 | blocked before implementation | `98db9d414b79f30fafe1b8db9a8afec34e058ed1` | `e0f3a5a` | Strict output requires explicit object properties; dynamic AC schema generator added; evidence under `artifacts/WO-BOOT-05/runner-attempt2/` |
| 2026-10-08 | WO-BOOT-05 r3 | implementation complete; report rejected, retrying | `e0f3a5a3365e58979812d0c004f6c6523698556b` | `63f37f7` | Local acceptance checks pass; corrected executor evidence and fresh verifier pending |
| 2026-10-08 | WO-BOOT-05 r3 | complete; local acceptance verified | `e0f3a5a3365e58979812d0c004f6c6523698556b` | `8b4fccb` | 50 tests; ASan/UBSan; fuzz seed `12648430` (226 cases); both probes detected; `artifacts/WO-BOOT-05/` |
| 2026-10-08 | WO-BOOT-05 r1 | blocked before model turn | `0bc1cfdc9f990878f087dfff936d12fb175f4678` | — | Codex strict JSON output rejected `minProperties`; attempt evidence retained under `artifacts/WO-BOOT-05/runner-attempt1/` |
| 2026-10-08 | WO-BOOT-05 r2 | ready | `98db9d414b79f30fafe1b8db9a8afec34e058ed1` | pending | ADR-0004 removes unsupported schema keyword; local runner preserves exact acceptance validation |
