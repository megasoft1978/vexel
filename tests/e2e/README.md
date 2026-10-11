# E2E and soak support

`render_soak_driver.c` repeatedly loads, rasterizes, and composes a small SVG in
one process while sampling its resident memory on macOS. CMake builds it as a
test-only target. `test_e2e_svg.py`, `test_packaging.py`, `test_soak.py`, and
`test_workflows.py` cover harness inputs, output schemas, app metadata, icon
generation, command bounds, and workflow policy.

The full generated SVG matrix and long soak are invoked with `./dev e2e` and
`./dev soak`; they keep inputs and reports in ignored `.cache/` paths.
