from pathlib import Path
import json,hashlib,subprocess
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01');P=A/'coordinator-primary'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
fail=[];counts={}
for baseline,label in [('artifacts/WO-RUST-PLAN-01/source-baseline.json','legacy'),(str(A/'source-baseline.json'),'rust')]:
 b=json.loads(Path(baseline).read_text());changes=[]
 for name,h in b['files'].items():
  p=Path(name);actual=sha(p) if p.is_file() else None
  if actual!=h:changes.append(dict(path=name,expected=h,actual=actual))
 head=subprocess.run(['git','rev-parse','HEAD'],capture_output=True,text=True,check=True).stdout.strip();idx=subprocess.run(['git','rev-parse','--git-path','index'],capture_output=True,text=True,check=True).stdout.strip()
 result=dict(baseline=baseline,baseline_sha256=sha(Path(baseline)),files_checked=len(b['files']),changes=changes,head=head,expected_head=b['head'],index_sha256=sha(Path(idx)),expected_index_sha256=b['index_sha256'])
 (A/f'preservation-{label}.json').write_text(json.dumps(result,indent=2)+'\n');counts[label]=len(b['files'])
 if changes or head!=b['head'] or sha(Path(idx))!=b['index_sha256']:fail.append('protected mismatch: '+label)
checks=0
for name in ['provenance.json','published-api-provenance.json','symbol-excerpt-provenance.json','availability-provenance.json','local-source-provenance.json','upstream-license-verification.json']:
 for r in json.loads((A/name).read_text()):
  path=r.get('local_path',r.get('path'));p=Path(path)
  if sha(p)!=r['sha256']:fail.append('hash mismatch '+str(p))
  checks+=1
  for pathkey,hashkey in [('source_path','source_sha256'),('binding_path','binding_sha256'),('sdk_path','sdk_sha256')]:
   if r.get(pathkey) and r.get(hashkey):
    if sha(Path(r[pathkey]))!=r[hashkey]:fail.append('source hash mismatch '+r[pathkey])
index=json.loads((P/'index.json').read_text())
for r in index:
 pkg=P/f"{r['name']}-{r['version']}";registry=P/(r['name']+'-registry.json');meta=json.loads(registry.read_text());v=next(v for v in meta['versions'] if v['num']==r['version'])
 assert sha(registry)==r['registry_sha256'];assert sha(pkg/'published.crate')==r['archive_sha256']==v['checksum'];assert not v['yanked']
 for rel,h in r['selected_files'].items():assert sha(pkg/rel)==h
report=Path('docs/product/rust-native-bindings-audit.md');words=len(report.read_text().split());assert words<=1800
if any(l.rstrip()!=l for l in report.read_text().splitlines()):fail.append('report whitespace')
diff=subprocess.run(['git','diff','--no-index','--','/dev/null',str(report)],capture_output=True,text=True);(A/'report.diff').write_text(diff.stdout);assert diff.returncode==1
resolution=json.loads((A/'feature-resolution.json').read_text());symbols=json.loads((A/'symbols.json').read_text());assert not resolution['unknown_features']
for r in symbols:
 for s in r['symbols']:
  if not s['binding_lines'] or not s['required_features_enabled']:fail.append('unresolved binding '+s['rust_symbol'])
(A/'coordinator-input-snapshot.json').write_text(json.dumps(dict(index_sha256=sha(P/'index.json'),packages=[{'name':r['name'],'version':r['version'],'archive_sha256':r['archive_sha256']} for r in index]),indent=2)+'\n')
out=dict(status='PASS' if not fail else 'FAIL',audit_status='LICENSE_POLICY_AND_PROTECTED_RECONCILIATION_GATES',failures=fail,report_words=words,report_word_limit=1800,protected_files=counts,primary_packages_verified=len(index),provenance_record_checks=checks,mapped_scopes=len(symbols),mapped_symbols=sum(len(r['symbols']) for r in symbols),report_diff_exit_code=diff.returncode,validators='resume-verification.json',command='python3 artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01/verify_resume.py',independent_review='pending coordinator; no recursive reviewer launched')
(A/'checkpoint-validation.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))

raise SystemExit(1 if fail else 0)
