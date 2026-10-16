"""Read-only audit of this blocked execution; never rewrite prior evidence."""
from pathlib import Path
import hashlib
import json
import re
import subprocess

root = Path(__file__).resolve().parents[2]
folder = root / 'artifacts/WO-MCP-ASYNC-03'
baseline = json.loads((folder / 'execution-r2-start-manifest.json').read_text())
order = (root / 'work/WO-MCP-ASYNC-03.md').read_text()
allowed = re.findall(r'`([^`]+)`', order.split('# Files allowed to touch\n')[1]
                     .split('# Protected paths')[0])
paths = set(subprocess.check_output(
    ['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=root
).decode().split('\0')) - {''}
paths.update(str(p.relative_to(root)) for p in (root / 'artifacts').rglob('*') if p.is_file())
mutable = {'STATE.md', 'LEDGER.md', 'ledger.jsonl', 'docs/mcp.md',
           'docs/product/vexel-goal-and-delivery-plan.md'}
changed = []
for path in sorted(paths | baseline['files'].keys()):
    source = root / path
    current = hashlib.sha256(source.read_bytes()).hexdigest() if source.is_file() else None
    if current == baseline['files'].get(path):
        continue
    changed.append(path)
    assert any(path.startswith(a[:-2]) if a.endswith('/**') else path == a
               for a in allowed), f'Outside r2 allowlist: {path}'
    if path in baseline['files']:
        assert path in mutable, f'Changed protected source/test/evidence: {path}'
    else:
        assert path.startswith('artifacts/WO-MCP-ASYNC-03/'), f'Unexpected new product file: {path}'
index = root / subprocess.check_output(['git', 'rev-parse', '--git-path', 'index'], cwd=root).decode().strip()
assert hashlib.sha256(index.read_bytes()).hexdigest() == baseline['index_sha256']
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip() == baseline['head_sha']
saved = folder / 'execution-r2-baseline'
for name in ['LEDGER.md', 'ledger.jsonl']:
    assert (root / name).read_bytes().startswith((saved / name).read_bytes())
old_ledger = (saved / 'ledger.jsonl').read_bytes()
suffix = (root / 'ledger.jsonl').read_bytes()[len(old_ledger):]
assert [json.loads(line) for line in suffix.splitlines()] == [json.loads(
    (folder / 'blocked-execution-r2-entry.json').read_text())]
changed_c = [p for p in changed if p.endswith('.c')]
for path in changed_c:
    assert len((root / path).read_text().splitlines()) < 800, path
assert not changed_c
for path in ['src/mcp/mcp_task_store.c', 'src/mcp/mcp_task_store.h',
             'tests/mcp/mcp_async_task_test.c', 'app/vexel_mcp_main.c']:
    assert hashlib.sha256((root / path).read_bytes()).hexdigest() == baseline['files'][path]
print(f'PASS: execution delta inside exact r2 allowlist; {len(baseline["files"])} existing files audited.')
print('PASS: all prior C/header/test/build files, task-store slice, frozen order/freeze and evidence unchanged.')
print('PASS: all unrelated shared dirty files preserved; HEAD/Git index unchanged; ledgers append-only.')
print('Changed C sources in execution: 0 (no new source line-limit violation).')
for path in ['src/mcp/mcp_broker_render.c', 'src/mcp/mcp_task_store.c', 'tests/mcp/mcp_async_task_test.c']:
    print(f'Preserved {path}: {len((root / path).read_text().splitlines())} lines')
print('Execution delta paths:\n' + '\n'.join(changed))
