# 10 — Owner decisions

Defaults below keep work moving; decide before the named milestone. Product locks are not questions.

| Question | Recommended default | Need by | Why it matters |
|---|---|---|---|
| Is the first paid/OSS wedge GIS/science large-document editing? | Yes; test against 12–20 interviews and user corpus | M0 | Avoid general-purpose parity scope |
| License? | MPL-2.0; Apache-2.0 if maximum permissive adoption is more important | Before first public repo | Contributor and dependency obligations |
| Product name? | Invented name; no Inkscape in brand/domain | Before public beta | Trademark and confusion risk |
| Minimum macOS? | 13 initially, verify APIs and user distribution before M1 package | M1 | Limits support burden and framework choices |
| Linux GPU presentation? | OpenGL/EGL-compatible fallback or Vulkan after hardware test | M11 design | Driver and Wayland distribution tradeoff |
| Linux text? | HarfBuzz + FreeType as explicit system deps | M6/M11 | Shaping consistency vs system package breadth |
| Raster backend? | M4 benchmark PlutoVG behind adapter against own scalar reference | M4 | Avoid expensive duplicate stroke work without evidence |
| Build tool? | CMake presets, C17 | Kickoff | All-platform toolchain generation |
| Unity build? | No initially | Kickoff | Readable, explicit dependencies |
| Accessibility deadline? | Mac core flows in M6; Linux/Windows in respective port milestones | M6 | Native services require architecture early |
| Linux support tier? | Wayland + X11 best effort, specify compositor matrix | M11 | GTK-free service scope can still be large |
| Animation private namespace? | None until SMIL/CSS prototype proves a gap; static SVG fallback required | M13 | Avoid custom-format lock-in |
| Price/funding? | Free open-source prototype; test sponsorship/support interest; no monetization claim | M10 | Ongoing support economics unknown |
| What performance advantage is worth shipping? | Proposed ≥20% on a primary user operation or strong fidelity/round-trip advantage | M10 | Keeps comparison decision concrete |

Owner should record final selections in accepted ADRs. Unknowns may remain unknown until evidence is collected; do not turn estimates into promises.
