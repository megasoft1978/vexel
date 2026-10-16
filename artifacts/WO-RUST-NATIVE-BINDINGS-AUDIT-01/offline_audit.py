from pathlib import Path
import json,hashlib,tarfile,tomllib
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01');P=A/'coordinator-primary'
def sha(b):return hashlib.sha256(b).hexdigest()
idx=json.loads((P/'index.json').read_text()); records=[]
for r in idx:
 n,v=r['name'],r['version'];p=P/f'{n}-{v}';meta=P/f'{n}-registry.json';assert sha(meta.read_bytes())==r['registry_sha256']
 m=json.loads(meta.read_text());version=next(x for x in m['versions'] if x['num']==v);assert not version['yanked'];assert version['checksum']==r['archive_sha256']
 arch=p/'published.crate';assert arch.stat().st_size<12000000;assert sha(arch.read_bytes())==r['archive_sha256']
 for rel,h in r['selected_files'].items():assert sha((p/rel).read_bytes())==h
 with tarfile.open(arch,'r:gz') as t:
  names=[x.name for x in t.getmembers() if x.isfile()];licenses=[x for x in names if '/LICENSE' in x or '/LICENSES/' in x or x.endswith('/COPYING')]
  print(n,'licenses',licenses)
  saved=[]
  for rel in licenses:
   info=t.getmember(rel);assert info.size<100000;data=t.extractfile(info).read(100001);out=A/'published-selective'/n/rel.split('/',1)[1];out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data);saved.append(dict(path=str(out),sha256=sha(data),archive_member=rel))
 d=tomllib.loads((p/'Cargo.toml').read_text());assert d['package']['version']==v;assert d['package']['license']==r['license_expression'];assert d['package']['rust-version']==r['rust_version']
 records.append(dict(**r,registry_path=str(meta),archive_path=str(arch),official_checksum=version['checksum'],yanked=version['yanked'],latest_stable=m['crate']['max_stable_version'],licenses=saved,verification='PASS; index + registry checksum + manifest + selected file hashes'))
 print('deps',n,json.dumps({k:d[k] for k in ['dependencies','build-dependencies','target'] if k in d}))
(A/'offline-primary-verification.json').write_text(json.dumps(records,indent=2)+'\n')
