**ACCEPT — shared DATA/INTERFACE gate.**

The dash validator now rejects malformed ranges, cell spans, nonfinite geometry and invalid globally tagged segment/join references; `SampleInput::validate` invokes it with work control. The new regression covers malformed cases and cancellation.

The prior source-pairing issue is resolved: `LexicalDocument` owns the source and takes tokens from the accepted XML stream, and `DecodeSource` is the only current constructor of `ElementView`. Its source and token fields are not public. README documents these and the other signature and data-layout adaptations.

This accepts the named shared contracts only. The renderer, decoder engine, native providers, application integration and final QA remain required. I did not run tests or builds; the 43 focused passes are coordinator-reported evidence.