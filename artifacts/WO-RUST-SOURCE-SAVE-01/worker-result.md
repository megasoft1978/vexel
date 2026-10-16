# WO-RUST-SOURCE-SAVE-01 repair candidate

Corrected the oversized-source early return: it retains `InputLimit` but now leaves the parse diagnostic at `None`/offset 0 and native result flags at their defaults, matching the wrapper’s pre-tokenization return. Updated only the incorrect Rust expectation and added an empty-source regression for `Tokenizer`/`IncompleteDocument` at offset 0 with all adapter gates untouched. The five original integration tests remain.

Archived the prior candidate result, parity map, verification, and the exact incorrect assertion under `archive/prior-candidate/`. The inherited dirty legacy C fixture was not edited or restored.

Full source-save scope remains intact: borrowed byte inputs, 16 MiB limit, owned bounded tokens, tokenizer diagnostics, native replacement/create fallback, safe hooks, and durability state. The accepted XML API does not distinguish reservation OOM from token capacity exhaustion; the existing typed `OutputExhausted` adaptation remains documented. The downstream 100 MiB workflow gap and independent review/integration remain outstanding.
