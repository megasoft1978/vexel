from pathlib import Path
import json,tomllib,tarfile,hashlib
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01');P=A/'coordinator-primary';O=A/'published-selective'
roots={
'objc2':['std','catch-all'],
'objc2-encode':['std'],
'block2':['std'],
'objc2-app-kit':['std','block2','objc2-quartz-core','objc2-core-graphics','NSApplication','NSWindow','NSScreen','NSRunningApplication','NSPanel','NSView','NSResponder','NSEvent','NSTextInputClient','NSTextInputContext','NSTrackingArea','NSDragging','NSCursor','NSGraphics','NSMenu','NSMenuItem','NSAccessibility','NSAccessibilityConstants','NSAccessibilityProtocols','NSAccessibilityElement','NSSavePanel','NSOpenPanel','NSAlert','NSButton','NSControl','NSPasteboard','NSWorkspace','NSAppearance','NSColor','NSColorSpace'],
'objc2-foundation':['std','block2','NSObject','NSString','NSArray','NSDictionary','NSData','NSValue','NSGeometry','NSRange','NSDate','NSError','NSNotification','NSRunLoop','NSTimer','NSThread','NSOperation','NSUserDefaults','NSKeyValueObserving','NSAttributedString','NSSet'],
'objc2-core-foundation':['std','objc2','CFString','CFNumber','CFAttributedString','CFDictionary','CFArray','CFData','CFRunLoop'],
'objc2-core-graphics':['std','objc2','CGBitmapContext','CGContext','CGColorSpace','CGColor','CGPath','CGImage','CGDataProvider','CGFont','CGDirectDisplay'],
'objc2-core-text':['std','objc2','objc2-core-graphics','CTFont','CTFontDescriptor','CTLine','CTRun','CTStringAttributes'],
'objc2-metal':['std','block2','MTLDevice','MTLCommandQueue','MTLCommandBuffer','MTLCommandEncoder','MTLTexture','MTLResource','MTLPixelFormat','MTLLibrary','MTLRenderPipeline','MTLRenderPass','MTLRenderCommandEncoder','MTLBlitCommandEncoder','MTLDrawable','MTLTypes'],
'objc2-quartz-core':['std','objc2-core-foundation','objc2-core-graphics','objc2-metal','CALayer','CAMediaTiming','CAMetalLayer']}
roots['objc2-image-io']=['std','objc2','objc2-core-graphics','CGImageSource','CGImageDestination','CGImageProperties']
index={r['name']:r for r in json.loads((P/'index.json').read_text())};man={n:tomllib.loads((P/f"{n}-{r['version']}"/'Cargo.toml').read_text()) for n,r in index.items()}
for n in list(roots):
 bad=[f for f in roots[n] if f not in man[n]['features']]
 if bad:print('INVALID FEATURES',n,bad);raise SystemExit(1)
features={n:set(fs) for n,fs in roots.items()};active=set(roots);enabled={n:set() for n in roots};edges={};missing={};unknown=[]
# These are ALL target dependency predicates in the supplied manifests; exact target evaluation below.
def applies(expr):
 rules={'cfg(unix)':True,'cfg(target_vendor = "apple")':True,'cfg(not(target_os = "watchos"))':True,'cfg(target_os = "macos")':True,'cfg(all(target_vendor = "apple", not(target_os = "macos")))':False}
 if expr not in rules:raise ValueError(expr)
 return rules[expr]
def deps(n):
 d=dict(man[n].get('dependencies',{}));d.update(man[n].get('build-dependencies',{}))
 for pred,t in man[n].get('target',{}).items():
  if applies(pred):d.update(t.get('dependencies',{}));d.update(t.get('build-dependencies',{}))
 return d
changed=True
while changed:
 before=(str(sorted(active)),str({n:sorted(v) for n,v in features.items()}),str({n:sorted(v) for n,v in enabled.items()}));changed=False
 for n in list(active):
  if n not in man:continue
  features.setdefault(n,set());enabled.setdefault(n,set());d=deps(n);fm=man[n].get('features',{})
  enabled[n].update(k for k,v in d.items() if not v.get('optional',False))
  deferred=[]
  for f in list(features[n]):
   if f in fm:terms=fm[f]
   elif f in d and not any('dep:'+f in xs for xs in fm.values()):terms=['dep:'+f]
   else:unknown.append([n,f]);continue
   for term in terms:
    if term.startswith('dep:'):enabled[n].add(term[4:])
    elif '/' in term:
     dep,df=term.split('/',1)
     if dep.endswith('?'):deferred.append((dep[:-1],df))
     else:enabled[n].add(dep);features.setdefault(dep,set()).add(df)
    else:features[n].add(term)
  for dep,df in deferred:
   if dep in enabled[n]:features.setdefault(dep,set()).add(df)
  for dep in enabled[n]:
   if dep not in d:raise ValueError((n,dep))
   v=d[dep];active.add(dep);features.setdefault(dep,set()).update(v.get('features',[]));enabled.setdefault(dep,set())
   if v.get('default-features',True) and (dep not in man or 'default' in man[dep].get('features',{})):features[dep].add('default')
   edges[(n,dep)]=dict(source=n,dependency=dep,range=v['version'],default_features=v.get('default-features',True),requested_features=v.get('features',[]),kind='build' if dep in man[n].get('build-dependencies',{}) else 'normal',optional=v.get('optional',False))
 after=(str(sorted(active)),str({n:sorted(v) for n,v in features.items()}),str({n:sorted(v) for n,v in enabled.items()}));changed=before!=after
missing=[dict(name=n,exact_version=None,required_ranges=sorted({e['range'] for e in edges.values() if e['dependency']==n}),features=sorted(features[n]),requested_inputs=['registry metadata','official-checksum-matching published archive','normalized/original Cargo manifests','license texts','build.rs and native/helper source if present'],reason='activated normal dependency; inputs absent') for n in sorted(active) if n not in man]
licenses=[]
for n,r in index.items():
 with tarfile.open(P/f"{n}-{r['version']}"/'published.crate') as t:
  rel=f"{n}-{r['version']}/.cargo_vcs_info.json";data=t.extractfile(rel).read(10000);vcs=json.loads(data);out=O/n/'.cargo_vcs_info.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
  if any('LICENSE' in m.name for m in t.getmembers()):continue
  licenses.append(dict(name=n,revision=vcs['git']['sha1'],repository=man[n]['package']['repository'],path='LICENSE.md (repository root, referenced by published README)',reason='published archive does not contain license text; manifest expression verified only'))
(A/'needed-primary-inputs.json').write_text(json.dumps(dict(status='BLOCKED_PRIMARY_INPUTS' if missing else 'PACKAGE_INPUTS_COMPLETE_LICENSE_TEXTS_PENDING',packages=missing,license_texts=licenses,exception_helper_policy='objc2 catch-all -> exception -> objc2-exception-helper ^0.1.1. Requires exact helper/build closure and native source provenance; no first-party C shim permitted.'),indent=2)+'\n')
(A/'feature-resolution.json').write_text(json.dumps(dict(target=dict(arch='aarch64',os='macos',vendor='apple',minimum_os='13.0'),resolver=3,roots={n:dict(version='='+index[n]['version'],default_features=False,features=sorted(fs)) for n,fs in roots.items()},expanded_features={n:sorted(fs) for n,fs in features.items()},activated_edges=list(edges.values()),activated_packages=sorted(active),missing_packages=missing,unknown_features=unknown,closure_status='complete published-manifest closure; Cargo lock/type/link checks remain later' if not missing else 'partial; missing package manifests prevent transitive completion',excluded=['dev-dependencies','examples/tests','unsupported OS runtime features','dispatch2','GNUstep','static selector/class proc macros']),indent=2)+'\n')
# Select only native API feature modules and specific ownership/callback source; no generic extraction.
selection={n:['src/generated/'+f+'.rs' for f in roots[n] if f[0].isupper()] for n in roots}
selection['objc2']=['src/rc/retained.rs','src/rc/autorelease.rs','src/macros/define_class.rs','src/__macro_helpers/msg_send.rs','src/runtime/protocol_object.rs','src/runtime/mod.rs']
selection['objc2-core-graphics'] += ['src/bitmap_context.rs','src/image.rs','src/thread_safety.rs']
selection['objc2-core-foundation'] += ['src/retained.rs','src/type_traits.rs','src/thread_safety.rs','src/array.rs','src/dictionary.rs','src/data.rs','src/string.rs']
selection['objc2-exception-helper']=['src/try_catch.m']
selection['block2']=['src/rc_block.rs','src/block.rs','src/stack.rs','src/traits.rs','src/global.rs']
records=[];total=0
for n,rels in selection.items():
 r=index[n];prefix=f"{n}-{r['version']}/"
 with tarfile.open(P/prefix/'published.crate') as t:
  members={m.name:m for m in t.getmembers() if m.isfile()}
  rels += ['translation-config.toml','src/generated/mod.rs'] if n.startswith('objc2-') and n!='objc2-encode' else []
  for rel in rels:
   if prefix+rel not in members:continue
   m=members[prefix+rel];assert m.size<600000;total+=m.size;assert total<15000000
   data=t.extractfile(m).read(600001);out=O/n/rel;out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data)
   records.append(dict(path=str(out),sha256=hashlib.sha256(data).hexdigest(),bytes=len(data),archive_member=m.name,archive_sha256=r['archive_sha256'],url=r['archive_url'],retrieved_utc=r['retrieved_utc']))
(A/'published-api-provenance.json').write_text(json.dumps(records,indent=2)+'\n');print('Activated:',sorted(active));print('Missing:',[(x['name'],x['required_ranges']) for x in missing]);print('Selected API files',len(records),'bytes',total)
