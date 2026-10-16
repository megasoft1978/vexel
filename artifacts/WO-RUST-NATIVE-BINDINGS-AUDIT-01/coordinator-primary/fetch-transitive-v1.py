from pathlib import Path
import urllib.request,json,hashlib,tarfile,io,time
from datetime import datetime,timezone
out=Path(__file__).resolve().parent
names=['bitflags','objc2-exception-helper','objc2-image-io']
records=json.loads((out/'index.json').read_text())
def get(url,limit):
 with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'Vexel-dependency-provenance-audit/0.1'}),timeout=25) as r:
  raw=r.read(limit+1)
  if len(raw)>limit:raise ValueError('response exceeds evidence bound')
  return raw,r.geturl()
for name in names:
 try:
  mp=out/(name+'-registry.json');url='https://crates.io/api/v1/crates/'+name
  if mp.exists():raw=mp.read_bytes()
  else:raw,_=get(url,2*1024*1024);mp.write_bytes(raw)
  d=json.loads(raw);version='0.1.1' if name=='objc2-exception-helper' else ('0.3.2' if name=='objc2-image-io' else next(v['num'] for v in d['versions'] if v['num'].startswith('2.') and '-' not in v['num'] and not v['yanked']));v=next(v for v in d['versions'] if v['num']==version);assert not v['yanked']
  archive,actualurl=get('https://crates.io/api/v1/crates/'+name+'/'+version+'/download',8*1024*1024)
  checksum=hashlib.sha256(archive).hexdigest();assert checksum==v['checksum'],name
  target=out/(name+'-'+version);target.mkdir(exist_ok=True);(target/'published.crate').write_bytes(archive)
  files={};total=0
  with tarfile.open(fileobj=io.BytesIO(archive),mode='r:gz') as t:
   members=t.getmembers();assert len(members)<12000
   for m in members:
    rel=m.name.split('/',1)[1] if '/' in m.name else ''
    if not m.isfile():continue
    selected=rel in ['Cargo.toml','Cargo.toml.orig','README.md','src/lib.rs','build.rs','src/main_thread_marker.rs','src/exception.rs'] or rel.startswith('LICENSE') or rel.startswith('src/exception/')
    if not selected:continue
    assert m.size<=2*1024*1024 and total+m.size<=8*1024*1024 and '..' not in Path(rel).parts
    data=t.extractfile(m).read();assert len(data)==m.size;total+=len(data)
    dest=target/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(data);files[rel]=hashlib.sha256(data).hexdigest()
  records.append({'name':name,'version':version,'registry_url':url,'registry_sha256':hashlib.sha256(raw).hexdigest(),'archive_url':actualurl,'archive_sha256':checksum,'license_expression':v.get('license'),'rust_version':v.get('rust_version'),'created_at':v.get('created_at'),'selected_files':files,'retrieved_utc':datetime.now(timezone.utc).isoformat(),'status':'verified_download'})
  print(name,version,'checksum verified',flush=True)
 except Exception as e:
  records.append({'name':name,'status':'failed','error':type(e).__name__+': '+str(e)});(out/'index.json').write_text(json.dumps(records,indent=2)+'\n');raise
 (out/'index.json').write_text(json.dumps(records,indent=2)+'\n')
 time.sleep(0.3)
