"""Audit the exact r2 execution delta, including pre-existing shared changes."""
from pathlib import Path, PurePosixPath
import difflib
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'artifacts/WO-MCP-TRANSPORT-01'
BASE = OUT / 'execution-r2-baseline'
manifest = json.loads((OUT / 'execution-r2-start-manifest.json').read_text())
allowed = ['work/WO-MCP-TRANSPORT-01.md', 'app/vexel_mcp_main.c',
           'tests/mcp/test_e2e.py', 'tests/test_map.json', 'CMakeLists.txt',
           'docs/mcp.md', 'src/mcp/README.md',
           'docs/product/vexel-goal-and-delivery-plan.md',
           'artifacts/WO-MCP-TRANSPORT-01/**', 'STATE.md', 'LEDGER.md', 'ledger.jsonl']
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
assert all(name in current for name in manifest['files']), 'baseline file removed'
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip() == manifest['head']
assert hashlib.sha256((ROOT / '.git/index').read_bytes()).hexdigest() == manifest['index_sha256']
for name in ['LEDGER.md', 'ledger.jsonl', 'artifacts/WO-MCP-TRANSPORT-01/verification.txt']:
    assert (ROOT / name).read_bytes().startswith((BASE / name).read_bytes()), name
for name, digest in manifest['files'].items():
    if name.startswith('artifacts/') and name != 'artifacts/WO-MCP-TRANSPORT-01/verification.txt':
        assert current[name] == digest, name
for name in ['work/WO-MCP-TRANSPORT-01.md', 'src/mcp/mcp_task_store.c',
             'src/mcp/mcp_task_store.h', 'tests/mcp/mcp_async_task_test.c']:
    assert current[name] == manifest['files'][name], name
# Ensure existing broker E2E assertions and unrelated dirty CMake/test-map edits
# are retained exactly; only the focused transport registrations were changed.
name = 'tests/mcp/test_e2e.py'
marker = 'class McpStdioEndToEndTests'
suffix = (BASE / name).read_text().split(marker, 1)[1]
current_suffix = (ROOT / name).read_text().split(marker, 1)[1]
old_tools = '''            "animation_remove", "export_animation", "selection_get", "selection_set", "export_svg",
        }'''
new_tools = '''            "animation_remove", "export_animation", "selection_get", "selection_set", "export_svg",
            "native_timeline_get", "native_timeline_add_track",
            "native_timeline_add_drawing_asset", "native_timeline_add_property_track",
            "native_timeline_set_property_key", "native_timeline_set_cel",
            "native_timeline_remove_cel", "native_timeline_move_item",
            "native_timeline_set_loop", "native_timeline_undo", "native_timeline_redo",
        }'''
assert old_tools in suffix
assert current_suffix == suffix.replace(old_tools, new_tools, 1)
name = 'CMakeLists.txt'
expected = (BASE / name).read_text().replace(
    'add_test(NAME mcp-stdio-e2e COMMAND python3 ${CMAKE_SOURCE_DIR}/tests/mcp/test_e2e.py -v)',
    'add_test(NAME mcp-stdio-relay COMMAND python3 ${CMAKE_SOURCE_DIR}/tests/mcp/test_e2e.py McpStdioRelayTests -v)\n'
    '    add_test(NAME mcp-stdio-e2e COMMAND python3 ${CMAKE_SOURCE_DIR}/tests/mcp/test_e2e.py McpStdioEndToEndTests -v)'
).replace('set_tests_properties(mcp-stdio-e2e PROPERTIES',
          'set_tests_properties(mcp-stdio-relay mcp-stdio-e2e PROPERTIES')
assert (ROOT / name).read_text() == expected
mapping = json.loads((ROOT / 'tests/test_map.json').read_text())
old_mapping = json.loads((BASE / 'tests/test_map.json').read_text())
focused = next(item for item in mapping['map'] if item['prefix'] == 'tests/mcp/test_e2e.py')
assert focused['ctest'] == 'mcp-stdio-(relay|e2e)' and focused['python'] == []
mapping['map'].remove(focused)
assert mapping == old_mapping
for test, test_class in [('mcp-stdio-relay', 'McpStdioRelayTests'),
                         ('mcp-stdio-e2e', 'McpStdioEndToEndTests')]:
    assert re.fullmatch(focused['ctest'], test)
    assert f'add_test(NAME {test} COMMAND python3 ${{CMAKE_SOURCE_DIR}}/tests/mcp/test_e2e.py {test_class} -v)' in expected
    assert f'add_test([=[{test}]=] ' in (ROOT / 'build/mac-debug/CTestTestfile.cmake').read_text()
print('PASS: frozen r2 order, HEAD and Git index unchanged; no staging/commit.')
print('PASS: all baseline files retained; all prior artifact bytes preserved (verification.txt appended).')
print('PASS: task-store slice and existing broker E2E checks byte-identical.')
print('PASS: unrelated pre-existing dirty CMake and test-map entries retained exactly.')
print('PASS: JSON parses; focused test-map selector and both CMake/generated registrations match.')
print('PASS: exact execution delta is within the frozen r2 allowlist:')
for name in changed:
    print(name)
unchanged = sum(current[name] == digest for name, digest in manifest['files'].items())
print(f'PASS: {unchanged}/{len(manifest["files"])} run-start files byte-identical; remaining edits are allowed.')
for name in changed:
    if name.endswith('.c') and not name.startswith('artifacts/'):
        lines = len((ROOT / name).read_text().splitlines())
        assert lines < 800, (name, lines)
        print(f'C line report: {name}: {lines} lines (<800).')
# Reviewable diff against run start, rather than against the dirty tree's HEAD.
delta = []
for name in changed:
    if name.startswith('artifacts/') or name not in manifest['files']:
        continue
    delta.extend(difflib.unified_diff((BASE / name).read_text().splitlines(True),
                                     (ROOT / name).read_text().splitlines(True),
                                     fromfile='run-start/' + name, tofile='execution-r2/' + name))
(OUT / 'execution-r2-delta.diff').write_text(''.join(delta))
