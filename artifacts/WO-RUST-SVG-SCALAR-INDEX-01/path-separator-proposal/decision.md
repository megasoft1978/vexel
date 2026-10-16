# Proposed fixture correction — awaiting owner authorization

No repository fixture, parser or acceptance assertion was changed.

The frozen `test_bounds_and_stable_diagnostics` names
`M0 0 L1.2.3 4 5` as malformed and expects the render CLI to reject it.
The current owned C and Rust path parsers allow a second decimal point to begin
the next coordinate. The existing positive C `path_number_test.c` requires
`M-.4.6L1.2.4Z` to match its spaced equivalent; the accepted Rust36test corpus
requires the same compact interpretation. W3C's path grammar explicitly shows
second-decimal separation (https://www.w3.org/TR/SVG/paths.html#PathDataBNF).
For this fixture the coordinates are `(0,0)`, `(1.2,0.3)`, `(4,5)`; its valid
lineto pair repetition follows the grammar. The exact whole legacyCLI case has
not been rerun here; source/grammar support a mistaken negative fixture, not a
parser regression. The new index test's path-only assertion also does not prove
whole renderer diagnostics.

Proposed change: keep the rejection assertion, replace only its malformed input
with `M0 0 L1.2..3 4 5` (consecutive dots with no digits), and add an affirmative
compact-versus-spaced source-number/geometry regression. The immutable original
is archived here, exact one-line diff is `proposed-fixture.patch`. Source and
all unrelated fixtures/expectations remain as they are.

AGENTS.md prohibits weakening/changing frozen acceptance fixtures and asks for
original assertions/predicates to be preserved. Owner authorization is required
to adopt this concrete correction and its intentional frozen hash exception;
unrelated implementation continues while pending. No special rejection of valid
SVG or silent removal/ignore of the failing test is proposed.
