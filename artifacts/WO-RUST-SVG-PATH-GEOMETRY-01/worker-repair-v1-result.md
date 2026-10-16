Documented `flatten_cubic`’s upfront validation and partial-append behavior, including how callers can truncate to a recorded length after failure. Updated the parity map and verification record. No implementation or test assertions changed; the result is 57 words.

The locked offline check and tests passed (12 integration tests), as did `git diff --check`.