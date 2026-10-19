# ADR-0033: app dependency on native timeline semantics

- Status: accepted
- Date: 2026-10-19
- Decision owner: project coordinator
- Related: ADR-0020, ADR-0032

## Context

The app's native property evaluator and project authoring flow need typed
timeline views, sampling, and source-preserving timeline edits. Rust does not
allow a crate to use another workspace crate through a transitive dependency;
the app therefore needs an explicit edge to `vexel-timeline`.

## Decision

Add the direct workspace path dependency from `vexel-app` to
`rust/crates/vexel-timeline`. Keep schema validation, sampling, and timeline
source edits in that crate; app rendering and project-session orchestration
remain in `vexel-app`. The edge adds no third-party package or algorithm and
does not change the timeline schema or SMIL path. The workspace path and
version are pinned by the repository revision and `Cargo.lock`.

## Consequences

The app may now call the public native timeline APIs directly. The dependency
graph remains acyclic: `vexel-timeline` depends on `vexel-project` and
`vexel-base`, neither of which depends on the app. The lockfile records the
workspace edge; no external dependency provenance entry is needed.
