"""Audit the scope-correction delta against the preserved pre-r2 worktree."""
from pathlib import Path
import difflib
import hashlib
import json
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
folder = root / 'artifacts/WO-MCP-ASYNC-03'
sys.path.insert(0, str(root / 'scripts'))
from wo_validate import load_wo

baseline = json.loads((folder / 'revision-r2-start-manifest.json').read_text())
saved = folder / 'revision-r2-baseline'
old_order = (saved / 'WO-MCP-ASYNC-03-r1.md').read_text()
order = (root / 'work/WO-MCP-ASYNC-03.md').read_text()
fields, spec = load_wo('WO-MCP-ASYNC-03', root)
assert fields['revision'] == '2' and fields['status'] == 'READY'
old_spec = re.search(r'^spec_sha256: ([0-9a-f]{64})$', old_order, re.M)[1]
unhashed_old = re.sub(r'(?m)^spec_sha256: [0-9a-f]{64}\n', '', old_order, count=1)
assert hashlib.sha256(unhashed_old.encode()).hexdigest() == old_spec
addition = '- `docs/product/vexel-goal-and-delivery-plan.md`\n'
anchor = '- `artifacts/WO-MCP-ASYNC-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`\n'
expected = old_order.replace('revision: 1\n', 'revision: 2\n', 1)
expected = expected.replace(anchor, addition + anchor, 1)
expected = expected.replace('spec_sha256: ' + old_spec, 'spec_sha256: ' + spec, 1)
assert order == expected, 'Revision changed beyond metadata and one allowlist path'
freeze = (folder / 'scope-freeze-r2.txt').read_text()
old_freeze = (folder / 'scope-freeze.txt').read_text()
assert freeze.startswith(old_freeze.replace('revision 1', 'revision 2', 1)
                         .replace(old_spec, spec, 1))
assert f'Frozen spec SHA-256: {spec}\n' in freeze
assert f'Base HEAD: {fields["base_sha"]}\n' in freeze
assert baseline['head_sha'] == fields['base_sha']

paths = set(subprocess.check_output(
    ['git', 'ls-files', '-c', '-o', '--exclude-standard', '-z'], cwd=root
).decode().split('\0')) - {''}
paths.update(str(p.relative_to(root)) for p in (root / 'artifacts').rglob('*') if p.is_file())
mutable = {'work/WO-MCP-ASYNC-03.md', 'STATE.md', 'LEDGER.md', 'ledger.jsonl'}
allowed = re.findall(r'`([^`]+)`', order.split('# Files allowed to touch\n')[1]
                     .split('# Protected paths')[0])
changed = []
for path in sorted(paths | baseline['files'].keys()):
    source = root / path
    current = hashlib.sha256(source.read_bytes()).hexdigest() if source.is_file() else None
    if current == baseline['files'].get(path):
        continue
    changed.append(path)
    assert any(path.startswith(pattern[:-2]) if pattern.endswith('/**')
               else path == pattern for pattern in allowed), f'Outside allowlist: {path}'
    if path in baseline['files']:
        assert path in mutable, f'Existing file/evidence changed: {path}'
    else:
        assert path.startswith('artifacts/WO-MCP-ASYNC-03/'), f'Unexpected new file: {path}'

index = root / subprocess.check_output(
    ['git', 'rev-parse', '--git-path', 'index'], cwd=root
).decode().strip()
assert hashlib.sha256(index.read_bytes()).hexdigest() == baseline['index_sha256']
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root).decode().strip() == baseline['head_sha']
old_ledger = (saved / 'ledger.jsonl').read_bytes()
new_ledger = (root / 'ledger.jsonl').read_bytes()
assert new_ledger.startswith(old_ledger), 'Existing ledger bytes changed'
suffix = [json.loads(line) for line in new_ledger[len(old_ledger):].splitlines()]
assert suffix == [json.loads((folder / name).read_text()) for name in
                  ['superseded-r1-entry.json', 'ready-r2-entry.json']]
assert [(entry['revision'], entry['event']) for entry in suffix] == [(1, 'superseded'), (2, 'ready')]
assert (root / 'LEDGER.md').read_bytes().startswith((saved / 'LEDGER.md').read_bytes())
assert 'revision 2 is frozen READY' in (root / 'STATE.md').read_text()
assert 'Revision 1 is superseded for this scope correction' in (root / 'STATE.md').read_text()
for path in sorted(folder.rglob('*.json')) + [root / 'tests/test_map.json']:
    json.loads(path.read_text())

print(f'PASS: r2 order differs only by revision/hash and the single allowlist addition; spec {spec}.')
print('PASS: r2 freeze matches revision, base, and frozen spec hash; r1 spec independently verified.')
print(f'PASS: {len(baseline["files"])} existing files audited; only order/STATE/LEDGER/ledger changed.')
print('PASS: all prior evidence, product/source/test files, and task-store slice unchanged byte-for-byte.')
print('PASS: existing ledger prefix preserved; exactly r1 superseded and r2 ready appended.')
print('PASS: work-order artifact JSON and test-map JSON parse; HEAD and Git index unchanged.')
print('Changed C source count: 0; implementation acceptance and line-limit checks not claimed.')
print('Revision delta paths:\n' + '\n'.join(changed))

deltas = []
for path, backup in [('work/WO-MCP-ASYNC-03.md', 'WO-MCP-ASYNC-03-r1.md'),
                     ('STATE.md', 'STATE.md'), ('LEDGER.md', 'LEDGER.md'),
                     ('ledger.jsonl', 'ledger.jsonl')]:
    deltas.extend(difflib.unified_diff(
        (saved / backup).read_text().splitlines(keepends=True),
        (root / path).read_text().splitlines(keepends=True),
        fromfile='pre-r2/' + path, tofile=path))
(folder / 'revision-r2.diff').write_text(''.join(deltas))
