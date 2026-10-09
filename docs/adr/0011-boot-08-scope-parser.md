# ADR-0011: Avoid inline scope tokens in protected-path prose

- Status: accepted under the owner's standing authorization to repair genuine verifier findings, 2026-10-09
- Scope: WO-BOOT-08 only

## Context

The work-order path parser treats every backticked token in the Protected paths section as a protected rule. Revision 3 explained in prose that two backticked script paths were allowed, but the parser correctly interpreted those same tokens as protected, ahead of the allowlist.

## Decision

Supersede WO-BOOT-08 revision 3 with revision 4. Keep the same exact allowed paths, but state their names without inline-code delimiters in the Protected paths prose. Set `base_sha` to the main commit containing ADR-0010, so both scope amendments are part of the verifier comparison base. Do not alter the verifier or broaden the allowlist.

## Consequences

The parser sees only the deliberately protected path patterns and applies the existing explicit allowlist to the two script paths. All other script and out-of-scope paths remain denied.
