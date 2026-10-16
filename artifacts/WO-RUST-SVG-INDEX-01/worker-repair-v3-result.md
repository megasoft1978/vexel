Corrected collector growth to clamp requests to the remaining whole-token capacity. Actual reserved capacity is checked before replacement; rejected candidates retain the original buffer.

Added one deterministic boundary regression and preserved all 11 prior tests. All named checks pass; 12 tests pass. Updated result, parity map, and verification evidence.

Candidate only, pending root review and main checks.