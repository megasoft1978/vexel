# WO-RUST-SVG-REFERENCE-01 candidate

Implemented the complete borrowed reference index and helper API. The nonempty index uses a 131,072-slot `u32` table and accepts at most 65,536 raw-byte IDs. Before filling or publishing the index, creation now checks the reserved Vec's actual capacity. An oversized successful reservation is consumed and dropped, returning `Limit`; allocation failure remains `OutOfMemory`. Empty indexes still allocate no table. Exact raw IDs, hashing, collisions, probe accounting, allowlists, and error offsets are preserved.

All ten reference integration tests pass. A unit test constructs a successfully reserved oversized Vec and asserts the internal exact-capacity gate rejects it. Existing actual-storage assertions verify a nonempty index reports exactly `131072 * size_of::<u32>()` bytes; no allocator failure is simulated or claimed.

The parity map records this adaptation. Result is an uncommitted candidate for independent review. Downstream shape-error conversion and use traversal/cycle handling remain outside this work order.
