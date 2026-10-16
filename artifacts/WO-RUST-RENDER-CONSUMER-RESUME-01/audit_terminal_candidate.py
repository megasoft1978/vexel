"""Read-only coordinator gate; refused while a candidate writer is live."""
from pathlib import Path
import hashlib,json,subprocess,sys
from datetime import datetime,timezone
r=Path(__file__).resolve().parents[2];w=sys.argv[1];a=r/'artifacts'/w
channel=w=='WO-RUST-PAINT-CHANNEL-CONTRACT-01'
initialization=w=='WO-RUST-SCENE-DECODE-INITIALIZATION-01'
cache_publication=w=='WO-RUST-CACHE-PUBLICATION-01'
shared=channel or initialization or cache_publication
t=r/('.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker' if shared else '.cache/'+w+'/worker')
status=a/('process-status.json' if shared else 'marker-repair-status.json' if w=='WO-RUST-SVG-DEFINITIONS-01' and (a/'marker-repair-status.json').exists() else 'accepted-integration-resume-status.json');s=json.loads(status.read_text());assert s['status'] in ('completed','failed'),'writer not terminal'
assert not subprocess.run(['ps','-p',str(s['child_pid']),'-o','comm='],capture_output=True,text=True).stdout.strip(),'writer PID exists'
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
b=json.loads((a/('r3-input-baseline.json' if channel and (a/'r3-input-baseline.json').exists() else 'r2-input-baseline.json' if channel and (a/'r2-input-baseline.json').exists() else 'input-baseline.json' if shared else 'overlay-baseline.json')).read_text());expected=dict(b['files']);seeds=[]
if not shared:
 for p in sorted(a.glob('*seed.json'),key=lambda p:p.stat().st_mtime_ns):
  data=json.loads(p.read_text());changed=data.get('files',{})
  for name,entry in changed.items():expected[name]=entry['after'] if isinstance(entry,dict) else entry
  if p.name=='r2-order-seed.json':expected['work/'+w+'.md']=data['new_sha256']
  seeds.append(str(p.relative_to(r)))
crate='vexel-scene' if shared else {'WO-RUST-SVG-MEDIA-01':'vexel-svg-media','WO-RUST-SVG-DECODE-01':'vexel-svg-decode','WO-RUST-SVG-SCALAR-SAMPLE-01':'vexel-svg-scalar-sample','WO-RUST-SVG-DEFINITIONS-01':'vexel-svg-definitions'}[w]
base='rust/crates/'+crate+'/'
bases=[base]+(['rust/crates/vexel-svg-animation/'] if initialization else [])
def allowed(name):
 if cache_publication:
  return name.startswith(base+'src/') or name==base+'README.md' or name==base+'tests/cache_publication.rs'
 if initialization:
  return any(name.startswith(item+'src/') or name==item+'README.md' or name==item+'tests/decode_initialization.rs' for item in bases)
 if name.startswith(base+'src/') or name==base+'README.md':return True
 if channel:return name==base+'tests/paint_channel_contracts.rs'
 return name.startswith(base+'tests/')
bad=[];checked=0
for name,sha in expected.items():
 if allowed(name):continue
 checked+=1;p=t/name
 if not p.is_file() or h(p)!=sha:bad.append(name)
# Added files must be in owned scope, or original HEAD-tracked evidence omitted
# intentionally by the original overlay inventory. Compare exact Git blob IDs.
tracked={}
raw=subprocess.check_output(['git','ls-files','-s'],cwd=t,text=True)
for line in raw.splitlines():
 meta,name=line.split('\t',1);tracked[name]=meta.split()[1]
paths=subprocess.check_output(['rg','--files','--hidden','-g','!.git','-g','!.cache','-g','!build','-g','!artifacts','-g','!reports'],cwd=t,text=True).splitlines()
for name in paths:
 if name in expected or allowed(name):continue
 p=t/name
 if not p.is_file():continue
 raw=p.read_bytes();blob=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
 if tracked.get(name)!=blob:bad.append('unexpected:'+name)
sizes={str(p.relative_to(t)):len(p.read_bytes().splitlines()) for folder in ('src','tests') for item in bases for p in (t/item/folder).rglob('*.rs')}
over={name:n for name,n in sizes.items() if n>=800}
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=t,text=True).strip();assert head==b['head']
source={str(p.relative_to(t)):h(p) for item in bases for p in (t/item).rglob('*') if p.is_file()}
report={'recorded_utc':datetime.now(timezone.utc).isoformat(),'wo_id':w,'status':s,'protected_hashes_checked':checked,'seeds_applied':seeds,'protected_mismatches':sorted(set(bad)),'overlength':over,'source_hashes':source,'verdict':'PASS' if not bad and not over else 'FAIL','not_acceptance':True}
p=a/'accepted-input-terminal-audit.json';p.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:report[k] for k in ('wo_id','protected_hashes_checked','protected_mismatches','overlength','verdict')}));sys.exit(0 if report['verdict']=='PASS' else 1)
