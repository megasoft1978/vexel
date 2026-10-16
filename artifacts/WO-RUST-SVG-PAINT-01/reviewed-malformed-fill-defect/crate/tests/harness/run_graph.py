"""Compile the graph proof against current locked offline Cargo artifacts.
Animation is an existing scene dependency, used solely to construct real test
fixtures; no manifest, dependency, or shared source changes are performed.
"""
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[5]
command = ['cargo', '+stable', 'test', '-p', 'vexel-svg-paint', '--locked', '--offline', '--no-run', '--message-format=json']
result = subprocess.run(command, cwd=root, text=True, capture_output=True)
if result.returncode:
    print(result.stderr)
    sys.exit(result.returncode)
needed = {'vexel_svg_paint', 'vexel_scene', 'vexel_svg_animation', 'vexel_geometry', 'vexel_xml'}
libs = {}
for line in result.stdout.splitlines():
    record = json.loads(line)
    if record.get('reason') != 'compiler-artifact':
        continue
    name = record['target']['name']
    if name in needed:
        for file in record['filenames']:
            if file.endswith('.rlib'):
                libs[name] = file
if set(libs) != needed:
    raise RuntimeError(f'Missing current Cargo artifacts: {needed - set(libs)}')
output = root / '.cache/tmp/paint-graph-publication-tests'
output.parent.mkdir(parents=True, exist_ok=True)
compile_command = ['rustc', '+stable', '--edition=2024', '--test', str(Path(__file__).with_name('graph_publication.rs')), '--target', 'aarch64-apple-darwin', '-L', 'dependency=' + str(Path(libs['vexel_svg_paint']).parent), '-o', str(output)]
for name in sorted(libs):
    compile_command += ['--extern', name + '=' + libs[name]]
compiled = subprocess.run(compile_command, cwd=root, text=True, capture_output=True)
if compiled.returncode:
    print(compiled.stderr)
    sys.exit(compiled.returncode)
run = subprocess.run([str(output)], cwd=root, text=True, capture_output=True)
print(run.stdout)
if run.stderr:
    print(run.stderr)
evidence = {'build_command': command, 'build_exit_code': result.returncode, 'compile_command': compile_command, 'compile_exit_code': compiled.returncode, 'test_command': [str(output)], 'test_exit_code': run.returncode, 'test_stdout': run.stdout, 'test_stderr': run.stderr, 'fixture_dependency': 'Already accepted locked vexel-scene dependency vexel-svg-animation; test construction only.'}
(root / 'artifacts/WO-RUST-SVG-PAINT-01/graph-verification.json').write_text(json.dumps(evidence, indent=2) + '\n')
sys.exit(run.returncode)
