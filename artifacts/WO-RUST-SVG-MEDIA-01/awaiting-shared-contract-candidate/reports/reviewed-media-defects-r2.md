# Independent root review supplied with frozen R2

The preceding candidate was rejected: **BLOCKER — owned-source review not
accepted.** The user supplied these findings, retained here with their R2
dispositions. Earlier candidate reports remain in r1-evidence-snapshot.json.

1. **Text style errors change the original classification.** Size, weight and
   spacing parsers returned Number for malformed values; original text.c routes
   any parse_text_style failure to Unsupported. The typed renderer propagated
   the wrong class. R2 corrects class, original evaluation order and source
   offsets; malformed-style and Number-position tests now exercise the real
   typed renderer before/after provider work as appropriate.
2. **Image append can leave partial draft state on failure.** Pixels are
   appended before path allocation and shape append/finalization. Later OOM,
   capacity or cancellation can leave that image in the caller's draft.
   Caller-owned cleanup was neither enforced nor proven. R2 reproduces late
   capacity-admission, shape-limit and cancellation failures with actual
   SceneBuilder/Draft/ChargeLedger fixtures. Root shared savepoints are absent;
   this defect remains unresolved. Text request suffix/state/cache rollback has
   the same required shared dependency and is also reproduced.
3. **Required pure behavior lacks meaningful proof.** Family/style/anchor/
   spacing parsing, the 256 KiB and command cache caps, and compound glyph
   contours were untested despite ported/covered claims. R2 adds direct and
   real-scene proofs for these behaviors, corrects compound subpath topology,
   and updates parity/coverage to distinguish functional tests from unresolved
   defect reproducers.

The supplied review did not rerun commands. It identified native providers,
mixed CLI/C suites, complete renderer/selection composition, CoreText/librsvg,
and application/UI validation as later mandatory integration gates. R2 has run
only the three scoped commands named by the frozen order; their exact outputs
are check-r2.log, test-r2.log and wo-r2.log. No acceptance is claimed.
