**ACCEPT.** Static review found no must-fix issue in the named JSON core. The parser’s token links and source spans preserve order and duplicate-key lookup behavior; nesting, byte, and token limits match the C definitions. String parsing validates UTF-8, escapes, and surrogate pairs. `uint` uses checked arithmetic. Owned parsing and decoding use fallible reservation, and buffer append/quoting reserve before mutation so limit or allocation errors leave contents intact.

The five Rust parity tests cover the named parser, accessor, and serializer behavior, including malformed input, embedded NUL, raw spans, checked integer overflow, and append rollback. The C reference and focused cases support the stated scope; protocol dispatch assertions are correctly excluded.

Coverage gaps: tests reject inputs above the byte and token limits but do not assert acceptance exactly at each limit. Allocation failure paths also lack deterministic fault-injection coverage. These are test gaps, not defects found in the reviewed code.

No builds or tests were run, per request.