# Black-box benchmark harness

`compare.py --smoke` validates the SHA-256 pinned `bench/corpus/manifest.json`
and all 24 fixture files, and records whether optional Inkscape and Donner
executables can be found. `--run` also probes the Vexel CLI `--version` and
runs render adapters whose compatible argv is explicitly configured. It never
compares a startup probe with a render task.

The only supported adapter configuration is JSON with this exact shape:

```json
{
  "schema_version": 1,
  "adapters": [{
    "id": "renderer-name",
    "path": "/absolute/path/to/renderer",
    "version_argv": ["--version"],
    "render_argv": ["/absolute/path/to/renderer", "--input", "{input}", "--output", "{output}", "--width", "{width}", "--height", "{height}"],
    "timeout_seconds": 30
  }]
}
```

Commands are argv arrays, never shell strings. Only `{input}`, `{output}`,
`{width}`, and `{height}` are substituted. Timeouts are capped at 60 seconds;
captured output and PNGs have fixed size limits. Output paths are generated
under repository `.cache/`; fixtures and outputs are hash checked. The selected
render command must start with the same resolved executable used by its version
probe. Tasks are `shapes-basic`, `stress-many-shapes`, and `stress-long-path`, each at
256x256. Each condition launches five processes. `cold-process` means a fresh
process launch; it does not mean filesystem cache eviction. `warm-repeat` runs
subsequent launches with normal OS caching.

Each adapter runs under the macOS `sandbox-exec` utility with process creation
denied. This keeps a timed adapter from leaving detached helpers running after
the trial. A renderer that requires helper processes fails explicitly, and the
harness fails closed if the macOS sandbox utility is unavailable.

JSON retains complete bounded trial observations, output hashes, argv, adapter
identity, fixture and source hashes, host metadata and limitations. CSV contains
the same raw trials plus matching summary rows. JSON embeds integrity hashes
for itself and CSV; `--verify` checks those hashes, fixtures, and any retained
PNG outputs without rewriting files. Stream hashes cover all child output while
only the first 16 KiB is retained; byte totals and truncation flags make that
distinction explicit. Pipe draining has a bounded cleanup window even if a
detached child keeps an inherited stream open.
Median and nearest-rank p95 are emitted only for five successful observations.
Failures, timeouts, unavailable RSS and unavailable adapters remain explicit;
unavailable values are never represented as zero. The standard-library process
interface does not expose reliable per-child peak RSS, so this host reports it
as unavailable. No external SVG resource is fetched or loaded by the harness.

Offline test adapter: `python3 bench/harness/compare.py --self-test-adapter
--json .cache/m0-03/fake.json --csv .cache/m0-03/fake.csv`.
