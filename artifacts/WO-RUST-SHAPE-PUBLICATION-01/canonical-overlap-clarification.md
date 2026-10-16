# Exact decoder blocker and preservation requirement

Current source evidence narrows the canonical-path failure: ordinary referenced
path and marker/use metadata already pass. The failing case has canonical
post-close overlapping subpath records. `validate::path` and sealed-scene checks
call path_topology::canonical only on overlaps, and currently give that helper
shape.source.element (the use-host annotation). Its segment-source containment
check then rejects actual referenced command spans.

Preserve this distinction. Do not regress ordinary metadata/host annotation or
rewrite/detach parser records. The ready order's explicit canonical geometry
ownership route must supply the actual authored path source to BOTH append and
sealed/frame overlap validation while retaining the independent instance host.
Generated text/image geometry legitimately has no authored path command table.
Do not invent d-attribute metadata for it or impose a new canonical-path proof on
all old constructors/literals. Existing strict ordinary metadata/topology tests
remain unchanged. An explicit same-source canonical path/d-attribute witness
may strengthen the new referenced-path route without weakening old guards.

Prefer bounded sparse provenance or an equivalent layout that does not silently
change the physical accounting/limits of every existing ordinary shape merely
to store rare distinct-source cases. All actual new storage and transient
admission remains metered, fallible and request/draft rollback-aware.
