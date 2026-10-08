# Owner directives (highest priority; supersede roadmap/CI/portability wording)

1. **macOS only initially.** Apple Silicon, macOS 13+. No Linux or Windows code, CI jobs, runners, shims, packaging or portability work until the owner lifts this.
2. Keep platform code isolated behind `platform/` interfaces (clean seams only; no stubs or ports for other OSes).
3. CI/dev scripts target macOS only. Skip any roadmap item for Linux/Windows (M11, M12) and cross-platform CI matrices.
