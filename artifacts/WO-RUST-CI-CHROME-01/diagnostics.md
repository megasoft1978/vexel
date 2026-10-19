# Chrome golden mismatch diagnostics

Added opt-in diagnostics to `rust/crates/vexel-app/tests/ui_chrome_goldens.rs`.
Set `VEXEL_CHROME_GOLDEN_DIAGNOSTICS=1` to print one stderr summary for each
frame that already fails the existing comparison. Each summary includes the
mismatch bounding box, differing pixel count, number of disjoint horizontal
row bands, worst channel delta, and 8-connected mismatch region count. Region
counting stops at 256 and reports `>=256`, keeping the diagnostic bounded.
No diagnostic files are written. With the variable unset, normal output and
comparison behavior are unchanged.

Verification on the local Apple Silicon macOS host:

```text
$ cargo test -p vexel-app --locked --test ui_chrome_goldens -- --test-threads=1
   Compiling vexel-app v0.1.0 (/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-app)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.49s
     Running tests/ui_chrome_goldens.rs (build/rust/aarch64-apple-darwin/debug/deps/ui_chrome_goldens-f00d21d5505d7cfe)

running 3 tests
test chrome_frames_are_repeatable_and_differ_by_state ... ok
test chrome_frames_match_first_party_goldens ... ok
test rle_round_trips_and_tolerance_rejects_large_changes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 37.75s
```
