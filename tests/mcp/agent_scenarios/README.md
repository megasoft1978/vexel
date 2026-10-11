# MCP agent scenarios

test_scenarios.py drives vexel-mcp over real stdio and talks to the in-process
headless broker used by the existing MCP e2e harness. Every scenario probes
tools/list first; a missing required tool causes a targeted skip, so the
scenario becomes active automatically when that tool is published.

The test creates each drawing through MCP calls, inspects its structure,
renders a PNG, checks the pixels against the checked-in PPM golden with
./dev pixdiff, exports SVG text, saves the document, and compares saved bytes
with the exported source. Animation scenarios use supported SMIL and render a
sampled frame. Morphing is deliberately excluded.

The production stdio relay connects to the live application broker when Vexel
is running. The automated test uses the headless broker because it has no
window; it never sends mouse or keyboard input. Every successful tool response
reports `mode=headless` in its structured content.

The examples in tools/list document the current schemas. Add new required tool
names to manifest.json when an upcoming scene or view-animation tool is part
of a scenario; the feature probe keeps that scenario dormant until the tool
is available.
