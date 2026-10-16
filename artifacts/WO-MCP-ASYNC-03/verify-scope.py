"""Audit only this run's changes against its captured shared-tree baseline."""
from pathlib import Path
import difflib
import hashlib
import json
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(root / 'scripts'))

folder = root / 'artifacts/WO-MCP-ASYNC-03'
baseline = json.loads((folder / 'run-start-manifest.json').read_text())
order = (root / 'work/WO-MCP-ASYNC-03.md').read_text()
import re
allowed = re.findall(r'`([^`]+)`', order.split('# Files allowed to touch\n')[1].split('# Protected paths')[0])
paths = set(subprocess.check_output(['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=root).decode().split('\0')) - {''}
changed = []
for path in sorted(paths | baseline['files'].keys()):
    source = root / path
    current = hashlib.sha256(source.read_bytes()).hexdigest() if source.is_file() else None
    if current != baseline['files'].get(path):
        changed.append(path)
        assert any(path.startswith(pattern[:-2]) if pattern.endswith('/**')
                   else path == pattern for pattern in allowed), f'Outside allowlist: {path}'
bookkeeping = {'STATE.md', 'LEDGER.md', 'ledger.jsonl', 'docs/mcp.md'}
assert all(p in bookkeeping or p.startswith('artifacts/WO-MCP-ASYNC-03/') for p in changed)
index = root / subprocess.check_output(['git', 'rev-parse', '--git-path', 'index'], cwd=root).decode().strip()
assert hashlib.sha256(index.read_bytes()).hexdigest() == baseline['index_sha256'], 'Index changed'
deltas = []
for path in sorted(bookkeeping):
    before = (folder / 'baseline' / path.replace('/', '_')).read_text().splitlines(keepends=True)
    after = (root / path).read_text().splitlines(keepends=True)
    matcher = difflib.SequenceMatcher(a=before, b=after, autojunk=False)
    assert all(op in {'equal', 'insert'} for op, *_ in matcher.get_opcodes()), f'Existing content changed: {path}'
    deltas.extend(difflib.unified_diff(before, after, fromfile='baseline/' + path, tofile=path))
(folder / 'task-delta.diff').write_text(''.join(deltas))
print('PASS: run delta is inside frozen allowlist; no product C/test/build changes.')
print('PASS: all unrelated captured files unchanged; existing bookkeeping content preserved as additions only.')
print('PASS: Git index unchanged; task-store source/header/test hashes unchanged.')
print('Changed paths:\n' + '\n'.join(changed))
print('Changed C source count: 0; no new line-limit violation.')
for path in ['src/mcp/mcp_broker_render.c', 'src/mcp/mcp_task_store.c', 'tests/mcp/mcp_async_task_test.c']:
    count = len((root / path).read_text().splitlines())
    assert count < 800
    print(f'Preserved {path}: {count} lines')
