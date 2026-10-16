# WO-RUST-MCP-ACCESS-01 candidate update

The four public path APIs accept `impl AsRef<OsStr>`. Validation, NUL detection, length bounds, and save leaf/parent splitting operate on raw macOS path bytes; existing `&str` callers remain compatible without lossy conversion. Production implementation is unchanged in this correction.

The non-UTF-8 fixture preserves the success assertions for filesystems that create the raw directory and file. If creation returns `EILSEQ` (errno 92) or `PermissionDenied`, it instead verifies native canonical/stat rejection, raw API outcomes for all four operations, NUL versus invalid-byte handling, lossy-replacement non-substitution using an actual valid U+FFFD path, and inside/outside grant rules. The previous failing fixture source and verification plus the root-host errno report are archived in this artifact directory.

All 12 tests pass on the worker host's native rejection branch; original 10 pass. Check, work-order validation, ledger validation (1197 entries), and `git diff --check` pass. Root independently reported errno 92; this worker observed `EPERM` at creation. The root may rerun the same named test outside the sandbox to exercise the filesystem-permitted success branch. The candidate remains uncommitted for coordinator review.
