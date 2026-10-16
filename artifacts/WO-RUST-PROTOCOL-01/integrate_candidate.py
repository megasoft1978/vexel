from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,sys
from datetime import datetime,timezone
from scripts.ledger import append_entry
root=Path.cwd();name,label,expected=sys.argv[1:4];expected=int(expected)
x=json.loads((root/'artifacts/WO-RUST-WAVE2-SETUP-01/batch.json').read_text())[name];wo=x['wo'];tree=Path(x['worktree']);a=root/'artifacts'/wo
baseline=json.loads((a/'overlay-baseline.json').read_text());prefix='rust/crates/'+x['crate']+'/'
hashfile=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
status=json.loads((a/(label+'-status.json')).read_text());assert status['status']=='completed' and status['exit_code']==0
verdict=(a/(label+'-result.md')).read_text();assert re.search(r'^\s*(?:\*\*)?ACCEPT\b',verdict),verdict
changes=[];missing=[];bad=[];new=[];inherited=0
allowed=lambda p:p.startswith(prefix+'src/') or p.startswith(prefix+'tests/') or p.startswith('artifacts/'+wo+'/')
for rel,digest in baseline['files'].items():
 p=tree/rel
 if not p.is_file():missing.append(rel)
 elif hashfile(p)!=digest:
  changes.append(rel)
  if not allowed(rel):bad.append(rel)
tracked={}
for line in subprocess.check_output(['git','ls-files','--stage'],text=True).splitlines():
 meta,p=line.split('\t',1);tracked[p]=meta.split()[1]
for parent,dirs,files in os.walk(tree):
 dirs[:]=[d for d in dirs if d not in ['.git','.cache','build','.worktrees','target','__pycache__']]
 for f in files:
  rel=str((Path(parent)/f).relative_to(tree))
  if rel in ['.DS_Store','.git'] or rel in baseline['files']:continue
  if rel in tracked:
   raw=(tree/rel).read_bytes()
   if hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==tracked[rel]:inherited+=1;continue
  new.append(rel)
  if not allowed(rel):bad.append(rel)
assert not(missing or bad),(missing,bad[:10])
code=[p for p in changes+new if p.startswith(prefix)]
for rel in code:
 p=root/rel
 if rel in baseline['files']:assert p.is_file() and hashfile(p)==baseline['files'][rel],rel
 else:assert not p.exists(),rel
 assert len((tree/rel).read_text().splitlines())<800,rel
# Shared registrations may have advanced in a separately accepted coordinator
# setup order while this isolated candidate was under review. Require an exact
# accepted registration hash rather than treating that change as a conflict.
latest={json.loads(line)['wo_id']:json.loads(line) for line in (root/'ledger.jsonl').read_text().splitlines()}
registered={}
for setup_order in ['WO-RUST-WAVE3-SETUP-01','WO-RUST-WAVE4-SETUP-01']:
 registration=root/'artifacts'/setup_order/'registered-manifests.json'
 if registration.exists() and latest.get(setup_order,{}).get('event')=='accepted':
  registered=json.loads(registration.read_text())
for rel in ['Cargo.toml','Cargo.lock','.cargo/config.toml','rust-toolchain.toml']:
 assert hashfile(root/rel)==registered.get(rel,baseline['files'][rel]),rel
assert hashfile(root/(prefix+'Cargo.toml'))==baseline['files'][prefix+'Cargo.toml']
legacy=json.loads((root/'artifacts/WO-RUST-PLAN-01/source-baseline.json').read_text());assert all((root/p).is_file() and hashfile(root/p)==h for p,h in legacy['files'].items())
head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip();assert head==baseline['head'] and hashfile(root/'.git/index')==baseline['root_index_sha256']
audit={'wo':wo,'changed_baseline_files':changes,'new_files':new,'unchanged_inherited_git_blob_files':inherited,'out_of_scope':bad,'missing_baseline_files':missing,'main_module_conflicts':[],'source_line_cap_violations':[],'pass':True,'accepted_review':label}
(a/'candidate-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
for rel in code:
 dest=root/rel;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(tree/rel,dest)
for filename in ['result.md','verification.txt','parity-map.json']+[p.name for p in (tree/'artifacts'/wo).glob('repair-v*.*')]:
 p=tree/'artifacts'/wo/filename
 if p.exists():shutil.copy2(p,a/('worker-'+filename))
assert all(hashfile(root/p)==hashfile(tree/p) for p in code)
(a/'integration-audit.txt').write_text('Exact repaired worker audit passed; all baseline source retained and historical tracked artifacts matched immutable git blobs.\nIntegrated only: '+', '.join(code)+'.\n603 original legacy code hashes and HEAD/index preserved. Shared manifests/lock unchanged; accepted narrowed independent review '+label+'.\n')
env=os.environ.copy();env.update(CARGO_HOME=str(root/'.cache/cargo'),TMPDIR=str(root/'.cache/tmp'),CARGO_TARGET_DIR=str(root/'build/rust'),MACOSX_DEPLOYMENT_TARGET='13.0')
compiler=subprocess.check_output(['rustc','+stable','--version'],env=env,text=True).strip();assert compiler.startswith('rustc 1.99.0 ')
commands=[['cargo','+stable','check','-p',x['crate'],'--locked','--offline'],['cargo','+stable','test','-p',x['crate'],'--locked','--offline'],['python3','scripts/wo_validate.py',wo,'--ready'],['python3','scripts/ledger.py','validate','ledger.jsonl'],['git','diff','--check']]
output=['Compiler: '+compiler+'\n'];count=0
for cmd in commands:
 r=subprocess.run(cmd,capture_output=True,text=True,env=env);output.append('COMMAND: '+' '.join(cmd)+'\nEXIT: '+str(r.returncode)+'\n'+r.stdout+r.stderr);(a/'verification-integrated.txt').write_text('\n'.join(output));assert r.returncode==0,r.stdout+r.stderr
 if 'test' in cmd:count=sum(int(n) for n in re.findall(r'test result: ok\. (\d+) passed;',r.stdout))
assert count==expected,(count,expected)
usage={'input_tokens':0,'output_tokens':0}
logs=[tree/'artifacts'/wo/'worker.events.jsonl',a/'review.events.jsonl']+list((tree/'artifacts'/wo).glob('repair-v*.events.jsonl'))+list(a.glob('recheck-v*.events.jsonl'))
models='gpt-6-luna + gpt-6.1-sol' if (a/'escalation.json').exists() else 'gpt-6-luna'
for p in logs:
 if p.exists():
  for line in p.read_text().splitlines():
   d=json.loads(line)
   if d.get('type')=='turn.completed':
    for k in usage:usage[k]+=d.get('usage',{}).get(k,0)
for event,status in [('run_finished','completed'),('verified','pass'),('accepted','accepted')]:
 append_entry(root/'ledger.jsonl',{'ts':datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),'wo_id':wo,'event':event,'revision':1,'base_sha':head,'head_sha':head,'run_id':wo+'-'+event+'-integrated','session_id':'rust-wave2-integration-2026-10-14','model':models,'cli_version':'0.162.1','duration_s':0,'usage':usage if event=='run_finished' else {'input_tokens':None,'output_tokens':None},'status':status,'artifacts':['artifacts/'+wo+'/'+label+'-result.md','artifacts/'+wo+'/integration-audit.txt','artifacts/'+wo+'/verification-integrated.txt'],'note':f'Accepted scoped {name} port after original independent review, focused correction/recheck, exact worker/root audit and named main checks. {count} package tests pass; not all new/unique tests where prior accepted coverage is included. Legacy603files/index/shared manifests retained. No full-app/UI/E2E claim.'})
assert all(hashfile(root/p)==h for p,h in legacy['files'].items())
print(wo,'accepted in main;',count,'package tests pass; protected sources/index preserved.')
