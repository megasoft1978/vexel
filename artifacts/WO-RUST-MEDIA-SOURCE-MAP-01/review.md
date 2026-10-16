**ACCEPT — scoped input contract.**

The implementation matches WO-RUST-MEDIA-SOURCE-MAP-01: each request carries a `SourceMap` separately from its canonical `ElementView`; validation checks builder and ledger source ownership, element token/range validity, bounded host span, finite annotation and geometry transforms independently, writable same-scene target, live transform-node identity, and cancellation. The four new tests cover distinct use-host/reference mapping, invalid ownership and handles, cancellation, and paused or sealed targets. The injected unsupported provider remains uncalled during validation.

Recorded verification reports 86 focused scene tests passing, the original 82 test files preserved, and the listed offline checks passing. I did not rerun checks.

Acceptance is limited to this contract: future MEDIA/DECODE adapters must pass and apply the `SourceMap` when annotating appended shapes. This does not accept producer, provider, renderer, or app integration.