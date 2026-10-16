# Result

Documented `flatten_cubic`'s append behavior: upfront invalid transforms or points leave output unchanged; recursion may append edges before a later `Limit` or `OutOfMemory`, with no automatic rollback. Callers can record the previous length and truncate after failure. Updated the parity map to match.

No implementation or test assertions changed. Focused frozen checks are recorded in `verification.txt`.
