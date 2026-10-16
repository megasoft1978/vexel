## ACCEPT — source acceptance

I found no concrete blocker in the shared raster job contracts. The owning buffers use one job ledger and source owner; payload and metadata capacity are admitted before allocation, and replacement keeps old storage charged until the new buffer passes capacity and control checks. Cleanup and lease invalidation are explicit and consistent across release, `release_all`, `finish`, and job drop. The boundary limit remains **200M marking-work units per mask**, not a bits or bytes cap.

The controlled paint path passes `WorkControl` into the provider loop and propagates cancellation and work-limit errors. The new typed seams have no implicit adapter from legacy `KernelResources`; the fixture demonstrates controlled behavior without registering a fake production renderer.

The preservation records match the original integration and source-unit test files. The worker log reports 111 passing tests: 86 original, 23 new integration, and 2 new unit. I did not rerun tests or builds.

**Remaining gates:** coordinator fresh-main/protected-path audit; production index/sample engines and coordinator adapters; preserved 603/157 renderer oracle acceptance; then the separate native/app, performance, MCP, final QA, release, and demo gates. These remain outside this source-acceptance review.