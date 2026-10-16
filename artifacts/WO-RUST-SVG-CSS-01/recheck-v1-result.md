**ACCEPT**

`CssSheet` retains the create-time source and token slice, and `compute` accepts neither as a replacement. Computed paints carry source provenance; a parent paint from a different source is rejected with `Limit`. The equal-length regression checks that the foreign parent’s value cannot contaminate the result.

The tests now assert exact values for all 39 properties and preserve the original C fixture’s meaningful value assertions.

CDATA handling is documented as an owner-approved additive compatibility adaptation, with a direct test checking that unknown declaration bytes remain unchanged. The parity map and verification report distinguish this from C behavior.

Read-only review; I did not run tests or builds.