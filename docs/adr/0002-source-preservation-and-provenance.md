# ADR 0002: Source preservation and provenance

- Status: accepted
- Date: 2026-10-08
- Decision owner: project owner

## Context

SVG documents contain formatting, metadata, namespaces, and unsupported content
that may not be represented by the first rendering subset. A destructive parse
and reserialization would lose information before editing fidelity is known.

## Decisions

- Retain original input bytes and source spans. A no-op save must be byte exact.
- Preserve unknown XML/SVG subtrees and namespaces when editing supported data.
- Treat scripts, external entities, and external resource loads as unsupported
  and do not execute or fetch them.
- External algorithms and all vendored code/assets need source URL, exact
  revision, cryptographic hash, license, rationale, and an adapter boundary in
  `docs/provenance.md` and `third_party/README.md` before inclusion.
- Do not inspect, fetch, or copy GPL SVG-editor implementation source. Standards
  documents and file-format descriptions are allowed.

## Consequences

Parser and editing APIs carry source ranges and explicit limits. Unsupported
features must be surfaced rather than silently dropped. Vendor or corpus
changes require a dedicated work order and license review.

## Sources

- `docs/research-r3/plan/04-coding-standard.md` (source spans and safety limits).
- `docs/research-r4/06-risks-and-safety.md` (untrusted document inputs).
