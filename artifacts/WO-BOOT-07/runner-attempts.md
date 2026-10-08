# Runner attempt record

The first executor run passed its focused UI tests and smoke replay. Its
workspace-write Codex process reported seven existing benchmark `sandbox-exec`
tests failing with macOS `sandbox_apply: Operation not permitted`; a serial
host run of the exact full suite later passed all 129 tests. The first runner
replay also found the smoke output already existed, so revision 2 adds cleanup
before the replay. Revision 3 adds a standalone validator for the required PNG
and JSON smoke artifacts.

Fresh verifier round 1 passed all 11 deterministic commands but failed because
the AC evidence files were uncommitted and absent from the candidate snapshot.
Those evidence files, including the exact sequential command transcript, are
now committed. The verifier finding is retained in
`verdict-round1-missing-evidence.json`.
