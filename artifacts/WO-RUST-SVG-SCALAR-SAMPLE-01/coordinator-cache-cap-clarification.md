# Clarify aggregate-cap review and actual remaining gap

The original full-review absent-cap premise was incomplete: accepted
RasterJob::check_pool already enforces16MiB across combined live Dash+Grid
capacities. It enforces that across BOTH published and staged cache generations,
which is stricter than the original per-cache retained-capacity bound. The new
owned per-cache CacheCapacity guard makes the logical policy explicit, but does
not alone remove that real shared staging restriction. No oversized-cache pass
is asserted without the shared pool checks.

The exact near-cap replacement reproducer and aggregate-contract-blocker.md
identify the remaining fix: independent logical cache-generation limits,
physical old+new storage/metadata admitted honestly through the caller ledger,
unchanged old cache on failure, required roles/identity/counts, single successful
publication and cleanup. Default legacy job-pool behavior and original assertions
must remain compatible; new production transaction APIs are explicit. No
Scratch-pool workaround or old-cache release-before-build is authorized.
