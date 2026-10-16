from pathlib import Path
import json,hashlib
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01');P=A/'coordinator-primary'
def save(n,d):(A/n).write_text(json.dumps(d,indent=2)+'\n')
ver=json.loads((A/'offline-primary-verification.json').read_text());res=json.loads((A/'feature-resolution.json').read_text());need=json.loads((A/'needed-primary-inputs.json').read_text());lic=[]
for r in json.loads((P/'upstream-licenses/index.json').read_text()):
 if r['status'].startswith('fetched'):
  f=P/'upstream-licenses'/r['revision']/r['path'];b=f.read_bytes();assert hashlib.sha256(b).hexdigest()==r['sha256']
  if 'git_blob_sha1' in r:assert hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==r['git_blob_sha1']
  lic.append({**r,'local_path':str(f),'verification':'SHA256 verified; Git blob identity verified where provided; license overview/SDK derivation statement, not full grant text'})
save('upstream-license-verification.json',lic)
need['referenced_license_texts']=[dict(license='MIT',url='https://opensource.org/license/MIT',reason='Referenced by exact-revision upstream LICENSE.md; complete standard grant text missing from supplied binding archives'),dict(license='Zlib',url='https://zlib.net/zlib_license.html',reason='Referenced by upstream; optional license choice, text not supplied'),dict(license='Apache-2.0',url='https://www.apache.org/licenses/LICENSE-2.0',reason='Referenced by upstream; optional license choice, text not supplied')]
need['minimum_license_choice']='MIT suffices for all known packages if exact grant and required attribution/notice evidence is accepted; no adoption authorization implied.'
need['license_evidence_note']='Exact published SPDX expressions verified. Four exact-revision LICENSE.md overviews verified, same Git blob/text, including package provenance revisions. Overviews link to canonical grants but omit full grant texts. Bitflags, cc, find-msvc-tools and shlex license texts included in published archives. Retained LICENSE-MIT.txt 404; no worker network request.'
need['resolved_package_requests']=[dict(name='bitflags',version='2.13.2'),dict(name='objc2-exception-helper',version='0.1.1'),dict(name='objc2-image-io',version='0.3.2'),dict(name='cc',version='1.8.0'),dict(name='find-msvc-tools',version='0.1.14'),dict(name='shlex',version='2.0.1')]
save('needed-primary-inputs.json',need)
pins=[]
for r in ver:
 if r['name'] not in res['activated_packages']:continue
 pins.append(dict(name=r['name'],version='='+r['version'],default_features=False if r['name'] in res['roots'] else 'see activated edges',features=res['roots'].get(r['name'],{}).get('features',res['expanded_features'][r['name']]),expanded_features=res['expanded_features'][r['name']],checksum=r['official_checksum'],license_expression=r['license_expression'],license_texts=r['licenses'],license_status='grant texts verified' if r['licenses'] else 'expression + exact-revision upstream overview verified; referenced full grant/notice verification remains',rust_version=r['rust_version'],source=r['archive_url']))
save('pins-features-licenses.json',dict(status='PUBLISHED_CLOSURE_RESOLVED_LICENSE_AND_POLICY_GATES',pins=pins,transitive_closure_status=res['closure_status'],missing_package_inputs=need['packages'],missing_license_inputs=need['referenced_license_texts'],exception_policy='catch-all -> third-party Objective-C helper -> cc. No first-party C shim; zero foreign unwinding anywhere in Rust is not established by helper. Separate native boundary/adoption decision required.',adopted=False))
scopes=[]
for row in json.loads((A/'symbols.json').read_text()):
 fs={row['defining_feature']}
 for s in row['symbols']:fs.update(s['required_features'])
 scopes.append(dict(scope=row['scope'],crate=row['crate'],exact_pin=res['roots'][row['crate']]['version'],default_features=False,features=sorted(fs),supporting_features='signature/ancestor Foundation and bridge features expanded in feature-resolution.json',status='published names and requested feature prerequisites verified; compile validation remains later'))
save('features-by-scope.json',scopes)
errors=json.loads((A/'errors.json').read_text());event=dict(phase='coordinator supplemental offline inputs',status='successful; 16 packages and complete manifest closure verified',network='no worker network calls');
if event not in errors:errors.append(event)
errors += [dict(phase='local metadata assembly',error='duplicate status keyword when composing license record',status='fixed by preserving upstream status and using separate verification field'),dict(phase='symbol excerpt evidence',error='two scopes shared one NSAccessibility excerpt path',status='fixed with scope-specific paths; no source changes')]
save('errors.json',errors)
save('unresolved.json',dict(package_closure='resolved from 16 published manifests; Cargo resolution not run',licenses='referenced full upstream grant text/notice and SDK distribution implications remain',SDK_generator='exact generator SDK/Xcode version absent; installed SDK27 independently established',exception_boundary='helper requires third-party Objective-C build and permits foreign unwinding inside runtime trampoline; absolute zero-foreign-unwind requirement unresolved',compiler_runtime='MSRVs known, compile/link/macOS13/IME/AX/GPU/teardown runtime checks deferred to separate orders',protected_source='603 legacy/HEAD/index matched; rust/crates/vexel-svg-animation/src/lib.rs differs from supplied Rust baseline, without any audit edits; coordinator reconciliation required',independent_review='pending coordinator, no recursive agents'))
print('Final evidenced pins',[(x['name'],x['version'],x['rust_version']) for x in pins])
