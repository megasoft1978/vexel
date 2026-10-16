**ACCEPT**

Collector growth clamps to the floor of remaining legal whole-token capacity using actual XML storage bytes. Checked subtraction, nonmultiple byte remainders, overflow, final-slot use, and actual reserved capacity are handled; rejected detached candidates are dropped without replacing existing tokens.

The new boundary regression covers capacities 1, 3, 5, and 65. Immediate completed-error ordering, source/preview/cancel behavior, background validation, and safe `Arc` ownership remain intact. The approved dependency hashes match. The recorded evidence reports 6 unit and 6 integration tests, preserving the prior 11 tests/assertions. I did not run checks.

This accepts the scoped index review only; no full application certification.