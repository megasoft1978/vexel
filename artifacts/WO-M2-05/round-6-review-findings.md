# Fresh verifier round 6 findings and fixes

The exact-commit read-only review of `ad58dbde7a4c193089e460fadfd9d448ea4ce853` found:

- The sanitizer wrapper captured stderr but ignored it, so recoverable UBSan reports could pass. The wrapper now checks the process result, writes stderr to `fuzz-report.stderr.txt`, rejects any stderr output, and reports the retained path and byte count. A fake-binary test emits a simulated UBSan report and verifies failure plus retention.
- Invalid or out-of-range root `height` values reported the width attribute offset. Width and height parsing/range checks now report their own source spans. Focused tests assert exact offsets for malformed and oversized heights.

All frozen machine checks pass on the corrected candidate. Fresh verifier round 7 is pending.
