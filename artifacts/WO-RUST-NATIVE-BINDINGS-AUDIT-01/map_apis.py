from pathlib import Path
import json,re,hashlib
A=Path('artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01');O=A/'published-selective';sdk=Path(json.loads((A/'sdk.json').read_text())['path'])
# Per-call prerequisites include defining modules, signature types, ancestors and optional bridges.
S=[
('shell','objc2-app-kit','AppKit','NSApplication',['sharedApplication','setActivationPolicy','activateIgnoringOtherApps','finishLaunching','nextEventMatchingMask_untilDate_inMode_dequeue','sendEvent','stop','setDelegate'],['NSApplication','NSResponder','NSEvent','NSRunningApplication']),
('window','objc2-app-kit','AppKit','NSWindow',['initWithContentRect_styleMask_backing_defer','setContentView','setDelegate','backingScaleFactor','screen','convertRectToScreen','close'],['NSWindow','NSResponder','NSView','NSGraphics','NSScreen']),
('view/layer/backing','objc2-app-kit','AppKit','NSView',['convertPoint_fromView','convertRectToBacking','setWantsLayer','setLayer','registerForDraggedTypes','updateTrackingAreas'],['NSView','NSResponder','NSTrackingArea','objc2-quartz-core']),
('input','objc2-app-kit','AppKit','NSResponder',['keyDown','keyUp','flagsChanged','mouseDown','mouseUp','mouseMoved','scrollWheel','magnifyWithEvent','interpretKeyEvents'],['NSResponder','NSEvent']),
('IME','objc2-app-kit','AppKit','NSTextInputClient',['insertText_replacementRange','setMarkedText_selectedRange_replacementRange','unmarkText','selectedRange','markedRange','hasMarkedText','attributedSubstringForProposedRange_actualRange','validAttributesForMarkedText','firstRectForCharacterRange_actualRange','characterIndexForPoint','doCommandBySelector'],['NSTextInputClient']),
('input context','objc2-app-kit','AppKit','NSTextInputContext',['handleEvent','discardMarkedText','invalidateCharacterCoordinates'],['NSTextInputContext','NSEvent']),
('event payload','objc2-app-kit','AppKit','NSEvent',['characters','charactersIgnoringModifiers','modifierFlags','scrollingDeltaX','scrollingDeltaY','magnification','timestamp','keyCode'],['NSEvent']),
('cursor','objc2-app-kit','AppKit','NSCursor',['arrowCursor','openHandCursor','closedHandCursor','resizeUpDownCursor','resizeLeftRightCursor','set'],['NSCursor']),
('drag/drop','objc2-app-kit','AppKit','NSDragging',['draggingEntered','draggingExited','performDragOperation','draggingPasteboard'],['NSDragging']),
('menus','objc2-app-kit','AppKit','NSMenu',['addItem','popUpContextMenu_withEvent_forView','menuNeedsUpdate'],['NSMenu','NSMenuItem','NSEvent','NSView','NSResponder']),
('command targets','objc2-app-kit','AppKit','NSMenuItem',['initWithTitle_action_keyEquivalent','setTarget','setEnabled','setState'],['NSMenuItem']),
('AX','objc2-app-kit','AppKit','NSAccessibilityElement',['accessibilityElementWithRole_frame_label_parent'],['NSAccessibilityElement','NSAccessibility','NSAccessibilityProtocols']),
('AX basic notification','objc2-app-kit','AppKit','NSAccessibility',['NSAccessibilityPostNotification'],['NSAccessibility','NSAccessibilityConstants']),
('AX notifications','objc2-app-kit','AppKit','NSAccessibilityConstants',['NSAccessibilityPostNotificationWithUserInfo'],['NSAccessibilityConstants']),
('AX actions','objc2-app-kit','AppKit','NSAccessibilityProtocols',['setAccessibilityParent','accessibilityPerformPress','accessibilityPerformIncrement','accessibilityPerformDecrement','accessibilityFrame','accessibilityChildren','accessibilityValue'],['NSAccessibilityProtocols']),
('save/sheets','objc2-app-kit','AppKit','NSSavePanel',['savePanel','beginSheetModalForWindow_completionHandler','setAllowedFileTypes','URL','cancel'],['NSSavePanel','NSPanel','NSWindow','NSResponder','NSApplication','block2']),
('open','objc2-app-kit','AppKit','NSOpenPanel',['openPanel','URLs','setAllowsMultipleSelection'],['NSOpenPanel','NSSavePanel','NSPanel','NSWindow','NSResponder']),
('alerts','objc2-app-kit','AppKit','NSAlert',['new','addButtonWithTitle','beginSheetModalForWindow_completionHandler','runModal'],['NSAlert','NSButton','NSControl','NSView','NSResponder','NSWindow','NSApplication','block2']),
('clipboard','objc2-app-kit','AppKit','NSPasteboard',['generalPasteboard','setString_forType','setData_forType','dataForType','readObjectsForClasses_options'],['NSPasteboard']),
('preferences flags','objc2-app-kit','AppKit','NSAccessibility',['accessibilityDisplayShouldIncreaseContrast','accessibilityDisplayShouldReduceMotion','accessibilityDisplayShouldReduceTransparency'],['NSAccessibility','NSWorkspace']),
('appearance','objc2-app-kit','AppKit','NSAppearance',['bestMatchFromAppearancesWithNames'],['NSAppearance']),
('accent','objc2-app-kit','AppKit','NSColor',['controlAccentColor','colorUsingColorSpace','getRed_green_blue_alpha'],['NSColor','NSColorSpace']),
('defaults','objc2-foundation','Foundation','NSUserDefaults',['standardUserDefaults','setInteger_forKey','objectForKey'],['NSUserDefaults']),
('observers','objc2-foundation','Foundation','NSNotification',['addObserverForName_object_queue_usingBlock','removeObserver'],['NSNotification','block2']),
('observer queue','objc2-foundation','Foundation','NSOperation',['mainQueue'],['NSOperation']),
('KVO','objc2-foundation','Foundation','NSKeyValueObserving',['addObserver_forKeyPath_options_context','removeObserver_forKeyPath_context'],['NSKeyValueObserving']),
('ticks','objc2-foundation','Foundation','NSTimer',['scheduledTimerWithTimeInterval_repeats_block','invalidate'],['NSTimer','block2']),
('wake','objc2-core-foundation','CoreFoundation','CFRunLoop',['CFRunLoopGetMain','CFRunLoopWakeUp'],['CFRunLoop']),
('font','objc2-core-text','CoreText','CTFont',['CTFontCreateWithName','CTFontCreateCopyWithSymbolicTraits','CTFontCreatePathForGlyph','CTFontGetAscent','CTFontGetDescent','CTFontGetLeading'],['CTFont','objc2-core-graphics']),
('shaping','objc2-core-text','CoreText','CTLine',['CTLineCreateWithAttributedString','CTLineGetGlyphRuns','CTLineGetTypographicBounds'],['CTLine']),
('runs','objc2-core-text','CoreText','CTRun',['CTRunGetGlyphCount','CTRunGetGlyphs','CTRunGetPositions','CTRunGetAdvances','CTRunGetAttributes'],['CTRun']),
('outline callback','objc2-core-graphics','CoreGraphics','CGPath',['CGPathApply','CGPathCreateMutable','CGPathMoveToPoint','CGPathAddLineToPoint','CGPathAddCurveToPoint','CGPathCloseSubpath'],['CGPath']),
('label pixels','objc2-core-graphics','CoreGraphics','CGBitmapContext',['CGBitmapContextCreate'],['CGBitmapContext','CGColorSpace','CGContext','CGImage']),
('draw/image','objc2-core-graphics','CoreGraphics','CGContext',['CGContextDrawImage','CGContextAddPath','CGContextFillPath','CGContextSetBlendMode'],['CGContext']),
('image properties/decode','objc2-image-io','ImageIO','CGImageSource',['CGImageSourceCreateWithData','CGImageSourceGetType','CGImageSourceGetCount','CGImageSourceCopyPropertiesAtIndex','CGImageSourceCreateImageAtIndex'],['CGImageSource','CGImageProperties','objc2-core-graphics']),
('image export','objc2-image-io','ImageIO','CGImageDestination',['CGImageDestinationCreateWithData','CGImageDestinationAddImage','CGImageDestinationFinalize'],['CGImageDestination','objc2-core-graphics']),
('device','objc2-metal','Metal','MTLDevice',['MTLCreateSystemDefaultDevice','newCommandQueue','newTextureWithDescriptor','newLibraryWithSource_options_error','newRenderPipelineStateWithDescriptor_error'],['MTLDevice','MTLTexture','MTLCommandQueue','MTLLibrary','MTLRenderPipeline']),
('texture','objc2-metal','Metal','MTLTexture',['texture2DDescriptorWithPixelFormat_width_height_mipmapped','replaceRegion_mipmapLevel_withBytes_bytesPerRow','getBytes_bytesPerRow_fromRegion_mipmapLevel'],['MTLTexture','MTLTypes','MTLPixelFormat']),
('GPU submit','objc2-metal','Metal','MTLCommandBuffer',['addCompletedHandler','presentDrawable','commit','status','renderCommandEncoderWithDescriptor','blitCommandEncoder'],['MTLCommandBuffer','MTLDrawable','MTLRenderCommandEncoder','MTLRenderPass','MTLBlitCommandEncoder','block2']),
('GPU render','objc2-metal','Metal','MTLRenderCommandEncoder',['setRenderPipelineState','setFragmentTexture_atIndex','drawPrimitives_vertexStart_vertexCount'],['MTLRenderCommandEncoder','MTLRenderPipeline','MTLTexture']),
('GPU blit','objc2-metal','Metal','MTLBlitCommandEncoder',['copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin'],['MTLBlitCommandEncoder','MTLTexture','MTLTypes']),
('present layer','objc2-quartz-core','QuartzCore','CAMetalLayer',['new','setDevice','setPixelFormat','setDrawableSize','nextDrawable','texture'],['CAMetalLayer','CALayer','objc2-core-foundation','objc2-metal']),
('display coordinates','objc2-core-graphics','CoreGraphics','CGDirectDisplay',['CGDisplayBounds'],['CGDirectDisplay']),
]
resolution=json.loads((A/'feature-resolution.json').read_text());features=resolution['expanded_features'];records=[];provenance=[]
for scope,crate,fw,module,symbols,required in S:
 p=O/crate/'src/generated'/f'{module}.rs'
 if module=='CGBitmapContext':p=O/crate/'src/bitmap_context.rs'
 lines=p.read_text().splitlines();h=sdk/'System/Library/Frameworks'/f'{fw}.framework/Headers'/f'{module}.h';hlines=h.read_text(errors='replace').splitlines() if h.exists() else []
 entries=[];excerpts=[]
 for symbol in symbols:
  # Generated C functions can also have associated method aliases; declarations are authoritative.
  pat=re.compile(r'\bfn\s+'+re.escape(symbol)+r'\b');hits=[i for i,l in enumerate(lines) if pat.search(l)]
  apple=symbol.split('_')[0];shits=[i for i,l in enumerate(hlines) if apple in l]
  entries.append(dict(rust_symbol=symbol,binding_lines=[i+1 for i in hits],sdk_search_term=apple,sdk_lines=[i+1 for i in shits],status='declaration found' if hits else 'inherited/generated alias requires separate verification',availability='exact excerpts retained; annotations/enclosing declarations require per-symbol review',required_features=required,required_features_enabled=all(f in features[crate] for f in required)))
  for i in hits:
   lo=max(0,i-13);hi=min(len(lines),i+15);excerpts.append('BINDING '+symbol+'\n'+'\n'.join(f'{j+1}: {lines[j]}' for j in range(lo,hi)))
  for i in shits[:3]:
   lo=max(0,i-2);hi=min(len(hlines),i+8);excerpts.append('SDK '+symbol+'\n'+'\n'.join(f'{j+1}: {hlines[j]}' for j in range(lo,hi)))
 out=A/'primary'/f'mapped-{crate}-{module}-{re.sub(r"[^a-zA-Z0-9]+","-",scope)}.txt';out.write_text('\n\n'.join(excerpts)+'\n');provenance.append(dict(path=str(out),sha256=hashlib.sha256(out.read_bytes()).hexdigest(),binding_path=str(p),binding_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),sdk_path=str(h),sdk_sha256=hashlib.sha256(h.read_bytes()).hexdigest() if h.exists() else None))
 records.append(dict(scope=scope,crate=crate,version=resolution['roots'][crate]['version'],module=module,defining_feature=module,binding_path=str(p),sdk_path=str(h),symbols=entries))
(A/'symbols.json').write_text(json.dumps(records,indent=2)+'\n');(A/'symbol-excerpt-provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
print('mapped groups',len(records),'symbols',sum(len(r['symbols']) for r in records))
for r in records:
 missing=[s['rust_symbol'] for s in r['symbols'] if not s['binding_lines']];disabled=[s['rust_symbol'] for s in r['symbols'] if not s['required_features_enabled']]
 if missing or disabled:print(r['scope'],'missing',missing,'disabled',disabled)
