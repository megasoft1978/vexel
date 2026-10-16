"""Local-only evidence checkpoint after first HTTPS request failed; no retries."""
from pathlib import Path
import json,hashlib,subprocess,datetime
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01')
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(name,obj):(A/name).write_text(json.dumps(obj,indent=2)+'\n')
sdk=Path('/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk')
# Candidate symbol families, not verified Rust exports or Cargo feature names.
specs=[
('lifecycle','AppKit','NSApplication.h',['sharedApplication','setActivationPolicy','finishLaunching','nextEventMatchingMask','sendEvent','stop:'],'objc2-app-kit'),
('window/delegates/teardown','AppKit','NSWindow.h',['initWithContentRect','windowShouldClose','windowDidResize','windowDidChangeBackingProperties','delegate','close;'],'objc2-app-kit'),
('input/responder','AppKit','NSResponder.h',['keyDown','keyUp','flagsChanged','scrollWheel','magnifyWithEvent','interpretKeyEvents'],'objc2-app-kit'),
('IME/committed/composition/candidate rect','AppKit','NSTextInputClient.h',['insertText','setMarkedText','unmarkText','selectedRange','markedRange','firstRectForCharacterRange','characterIndexForPoint'],'objc2-app-kit'),
('geometry/backing/display','AppKit','NSView.h',['convertPoint','convertRectToBacking','convertRectFromBacking','viewDidChangeEffectiveAppearance'],'objc2-app-kit'),
('display bounds','CoreGraphics','CGDirectDisplay.h',['CGDisplayBounds'],'objc2-core-graphics'),
('cursor','AppKit','NSCursor.h',['arrowCursor','openHandCursor','closedHandCursor','resizeUpDownCursor'],'objc2-app-kit'),
('drag/drop','AppKit','NSDragging.h',['draggingEntered','performDragOperation','draggingPasteboard'],'objc2-app-kit'),
('menus/commands/recent/dock','AppKit','NSMenu.h',['popUpContextMenu','menuNeedsUpdate','addItem'],'objc2-app-kit'),
('AX trees/actions','AppKit','NSAccessibilityElement.h',['NSAccessibilityElement','accessibilityElementWithRole'],'objc2-app-kit'),
('AX notifications','AppKit','NSAccessibility.h',['NSAccessibilityPostNotification','NSAccessibilityPostNotificationWithUserInfo'],'objc2-app-kit'),
('panels/sheets','AppKit','NSSavePanel.h',['beginSheetModalForWindow','runModal','allowedContentTypes','allowedFileTypes'],'objc2-app-kit'),
('open dialogs','AppKit','NSOpenPanel.h',['openPanel','URLs'],'objc2-app-kit'),
('alerts/recovery choices','AppKit','NSAlert.h',['addButtonWithTitle','runModal','beginSheetModalForWindow'],'objc2-app-kit'),
('clipboard','AppKit','NSPasteboard.h',['generalPasteboard','setString','setData','dataForType','readObjectsForClasses'],'objc2-app-kit'),
('preferences/accessibility flags','AppKit','NSWorkspace.h',['accessibilityDisplayShouldReduceMotion','accessibilityDisplayShouldIncreaseContrast','accessibilityDisplayShouldReduceTransparency','NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification'],'objc2-app-kit'),
('appearance/material','AppKit','NSGlassEffectView.h',['NSGlassEffectView','API_AVAILABLE'],'objc2-app-kit'),
('preferences persistence','Foundation','NSUserDefaults.h',['standardUserDefaults','setInteger','NSUserDefaultsDidChangeNotification'],'objc2-foundation'),
('observer lifetime/blocks','Foundation','NSNotification.h',['addObserverForName','usingBlock','removeObserver'],'objc2-foundation'),
('main thread','Foundation','NSThread.h',['isMainThread'],'objc2-foundation'),
('wake/runloop/ticks','Foundation','NSTimer.h',['scheduledTimerWithTimeInterval','block:','invalidate'],'objc2-foundation'),
('run loop','Foundation','NSRunLoop.h',['runMode','performBlock'],'objc2-foundation'),
('font/outline','CoreText','CTFont.h',['CTFontCreateWithName','CTFontCreateCopyWithSymbolicTraits','CTFontCreatePathForGlyph'],'objc2-core-text'),
('layout','CoreText','CTLine.h',['CTLineCreateWithAttributedString','CTLineGetGlyphRuns','CTLineGetTypographicBounds'],'objc2-core-text'),
('glyphs/metrics','CoreText','CTRun.h',['CTRunGetGlyphs','CTRunGetPositions','CTRunGetAdvances','CTRunGetAttributes'],'objc2-core-text'),
('path callback','CoreGraphics','CGPath.h',['CGPathApply','CGPathApplyWithBlock'],'objc2-core-graphics'),
('label bitmap','CoreGraphics','CGBitmapContext.h',['CGBitmapContextCreate'],'objc2-core-graphics'),
('CF ownership','CoreFoundation','CFBase.h',['CFRetain','CFRelease'],'objc2-core-foundation'),
('GPU device/shader/pipeline','Metal','MTLDevice.h',['MTLCreateSystemDefaultDevice','newLibraryWithSource','newRenderPipelineStateWithDescriptor','newTextureWithDescriptor','newCommandQueue'],'objc2-metal'),
('GPU submission/completion','Metal','MTLCommandBuffer.h',['addCompletedHandler','presentDrawable','commit','waitUntilCompleted'],'objc2-metal'),
('GPU upload/readback','Metal','MTLTexture.h',['replaceRegion','getBytes'],'objc2-metal'),
('presenter layer','QuartzCore','CAMetalLayer.h',['CAMetalLayer','nextDrawable','drawableSize','maximumDrawableCount'],'objc2-quartz-core'),
]
provenance=json.loads((A/'provenance.json').read_text()); matrix=[]
for scope,framework,header,terms,crate in specs:
 p=sdk/'System/Library/Frameworks'/f'{framework}.framework/Headers'/header
 row=dict(scope=scope,candidate_crate=crate,version=None,features=None,binding_status='unverified: registry unreachable',sdk_header=str(p),symbols=[])
 if not p.exists():row['sdk_error']='header absent'
 else:
  lines=p.read_text(errors='replace').splitlines(); excerpts=[]
  for term in terms:
   hits=[i for i,l in enumerate(lines) if term in l]
   row['symbols'].append(dict(symbol=term,lines=[i+1 for i in hits],availability_status='declaration excerpt only; enclosing availability not exhaustively verified'))
   for i in hits[:3]:
    lo=max(0,i-2);hi=min(len(lines),i+6);excerpts.append('\n'.join(f'{j+1}: {lines[j]}' for j in range(lo,hi)))
  out=A/'primary'/f'sdk-{framework}-{header}.txt';out.write_text('\n\n'.join(excerpts)+'\n')
  provenance.append(dict(path=str(out),source_path=str(p),source_sha256=sha(p),sha256=sha(out),bytes=out.stat().st_size,date=now,kind='installed Apple SDK header excerpt',license_status='local SDK; no crate license inference'))
 matrix.append(row)
save('symbols.json',matrix)
crates=['objc2','objc2-encode','objc2-foundation','objc2-app-kit','objc2-core-foundation','objc2-core-graphics','objc2-core-text','objc2-metal','objc2-quartz-core','block2']
save('pins-features-licenses.json',dict(status='BLOCKED',reason='first crates.io request failed DNS; no further HTTPS attempted',pins=[dict(name=n,exact_pin=None,requested_candidate='0.3.2' if n.startswith('objc2-') and n!='objc2-encode' else 'current metadata',default_features=False,features=None,checksum=None,license_expression=None,license_text_sha256=None,rust_version=None,status='unverified') for n in crates],transitive_closure=None,resolution_rules=['Obtain current metadata and exact 0.3.2 framework versions; reject yanked/unpublished candidates.','Read normalized published Cargo.toml and Cargo.toml.orig, checksums and license files; bound and verify archives before reading.','For Apple Silicon macOS only, expand requested features to fixed point including dependency feature edges, weak dep?/feature edges, optional dep: edges, target predicates and default-feature activation.','Constrain every activated package/version including build dependencies; verify semver ranges, resolver3 Rust1.99 compatibility and registry checksums.','Record all resolved licenses and exact pins; published feature lists and closure remain unknown here.','No Cargo fetch/compile in this audit; future approved adoption must confirm lock resolution and API type checking.'],block2_justification='Proposal requires Metal completion and asynchronous sheets/observers; Apple headers show block parameters. Rust block API/export/features/licenses remain unverified; no adoption.',exception_boundary='Must verify objc2 exception handling against published implementation and helper/build dependency closure; Rust catch_unwind alone is not an Objective-C exception fence. No C shim approved.'))
# Read/hash bounded original excerpts needed for checkpoint; preserve source exactly.
paths=['OWNER-DIRECTIVES.md','AGENTS.md','artifacts/WO-RUST-NATIVE-PLAN-01/result.md']
paths += [str(p) for p in Path('docs/adr').glob('002[2345]*.md')]
paths += ['src/platform/platform_window.h','src/platform/platform_text_ui.h','src/platform/platform_metal.h','src/platform/media/platform_text.h','src/platform/macos/platform_window_internal.h','src/platform/macos/platform_metal_internal.h','src/platform/macos/platform_accessibility.h','src/platform/macos/platform_clipboard.h','src/platform/macos/platform_dialog.h']
paths += ['src/platform/macos/platform_window.m','src/platform/macos/platform_window_native_shell.m','src/platform/macos/platform_window_geometry.m','src/platform/macos/platform_window_animation.m','src/platform/macos/platform_menus.m','src/platform/macos/platform_accessibility.m','src/platform/macos/platform_dialog.m','src/platform/macos/platform_clipboard.m','src/platform/macos/platform_metal.m','src/platform/platform_text_ui.m','src/platform/media/platform_text.m']
source=[]
for name in paths:
 p=Path(name);source.append(dict(path=name,sha256=sha(p),bytes=p.stat().st_size))
save('local-source-provenance.json',source)
save('sdk.json',dict(path=str(sdk),version='27.0',settings_sha256=sha(sdk/'SDKSettings.json'),target='arm64-apple-darwin; macOS13+',runtime_verified=False,binding_generator_sdk_provenance=None,notes=['xcrun emitted DVTFilePathFSEvents and DARWIN_USER_CACHE_DIR warnings but returned path/version successfully.','Installed SDK declaration checks do not prove macOS13 runtime behavior.']))
save('provenance.json',provenance)
for baseline in ['artifacts/WO-RUST-PLAN-01/source-baseline.json',str(A/'source-baseline.json')]:
 b=json.loads(Path(baseline).read_text());changes=[]
 for name,h in b['files'].items():
  p=Path(name);actual=sha(p) if p.is_file() else None
  if actual!=h:changes.append(dict(path=name,expected=h,actual=actual))
 head=subprocess.run(['git','rev-parse','HEAD'],capture_output=True,text=True).stdout.strip()
 idx=subprocess.run(['git','rev-parse','--git-path','index'],capture_output=True,text=True).stdout.strip()
 save('preservation-'+('legacy' if 'WO-RUST-PLAN-01/' in baseline else 'rust')+'.json',dict(baseline=baseline,baseline_sha256=sha(Path(baseline)),files_checked=len(b['files']),changes=changes,head=head,expected_head=b['head'],index_sha256=sha(Path(idx)),expected_index_sha256=b['index_sha256']))
print('Local checkpoint collected:',len(matrix),'symbol groups;',len(provenance),'SDK evidence records.')
