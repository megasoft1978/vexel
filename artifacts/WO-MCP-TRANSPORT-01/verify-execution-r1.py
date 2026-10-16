"""Read-only baseline preservation/scope checks for the blocked r1 execution."""
from pathlib import Path, PurePosixPath
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'artifacts/WO-MCP-TRANSPORT-01'
manifest = json.loads((OUT / 'execution-r1-start-manifest.json').read_text())
allowed = ['work/WO-MCP-TRANSPORT-01.md', 'app/vexel_mcp_main.c',
           'tests/mcp/test_e2e.py', 'tests/test_map.json', 'CMakeLists.txt',
           'docs/mcp.md', 'src/mcp/README.md', 'artifacts/WO-MCP-TRANSPORT-01/**',
           'STATE.md', 'LEDGER.md', 'ledger.jsonl']
record_paths = {'STATE.md', 'LEDGER.md', 'ledger.jsonl'}
current = {}
for raw in subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=ROOT).split(b'\0'):
    if raw:
        name = raw.decode()
        path = ROOT / name
        if path.is_file():
            current[name] = hashlib.sha256(path.read_bytes()).hexdigest()
changed = sorted(name for name in manifest['files'].keys() | current.keys()
                 if manifest['files'].get(name) != current.get(name))
assert all(any(PurePosixPath(name).match(pattern) for pattern in allowed) for name in changed), changed
assert all(name in record_paths or name.startswith('artifacts/WO-MCP-TRANSPORT-01/') for name in changed), changed
preserved = sum(current.get(name) == digest for name, digest in manifest['files'].items())
assert preserved == len(manifest['files']) - len(record_paths)
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip() == manifest['head']
assert hashlib.sha256((ROOT / '.git/index').read_bytes()).hexdigest() == manifest['index_sha256']
for name in ['LEDGER.md', 'ledger.jsonl']:
    assert (ROOT / name).read_bytes().startswith((OUT / 'execution-r1-baseline' / name).read_bytes())
print(f'PASS: {preserved}/{len(manifest["files"])} baseline files unchanged; only STATE and ledger records modified.')
print('PASS: HEAD and Git index unchanged; existing ledger content retained byte for byte.')
print('PASS: exact execution delta is within the frozen allowlist:')
for name in changed:
    print(name)
print('C line report: no changed C files; app/vexel_mcp_main.c unchanged at',
      len((ROOT / 'app/vexel_mcp_main.c').read_text().splitlines()), 'lines (<800).')
mapping = json.loads((ROOT / 'tests/test_map.json').read_text())
entry = next(item for item in mapping['map'] if item['prefix'] == 'tests/mcp/')
assert re.search(entry['ctest'], 'mcp-stdio-e2e')
assert 'add_test(NAME mcp-stdio-e2e COMMAND python3 ${CMAKE_SOURCE_DIR}/tests/mcp/test_e2e.py -v)' in (ROOT / 'CMakeLists.txt').read_text()
print('PASS: existing test-map JSON parses; tests/mcp/ maps to mcp-stdio-e2e, registered in CMake.')
print('New focused relay tests/mappings: NOT IMPLEMENTED; pre-product-edit scope stop.')
