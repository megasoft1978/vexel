# Exact cached module registration evidence

This supplements the declaration and feature proof in api-proof/native-constant-proof. Each record links to an actual cached module import/cfg and its SHA256; feature-name similarity alone is not evidence. Active dependency and cached Cargo feature declarations are in the inventory metadata table. No repository module/dependency registration was changed.

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:26`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0002, API-0003, API-0004, API-0005, API-0371, CONST-0089.

```rust
#[cfg(feature = "CFArray")]
#[path = "CFArray.rs"]
mod __CFArray;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:29`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0006, API-0007, API-0008.

```rust
#[cfg(feature = "CFAttributedString")]
#[path = "CFAttributedString.rs"]
mod __CFAttributedString;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:59`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0009, API-0010, API-0011, API-0012, API-0013, API-0014.

```rust
#[cfg(feature = "CFData")]
#[path = "CFData.rs"]
mod __CFData;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:68`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0015, API-0016, API-0017, API-0018, API-0372, API-0373, CONST-0090, CONST-0091.

```rust
#[cfg(feature = "CFDictionary")]
#[path = "CFDictionary.rs"]
mod __CFDictionary;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/lib.rs:40`

File SHA256 `83f4ca0f26d5d8e423f1cb80de4b9f1dd7f7f58b98da8b24832446c6e013f164`. Records: API-0019, API-0364, API-0365, CONST-0082, CONST-0083.

```rust
mod generated;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:92`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0020, API-0021, API-0022, API-0366, API-0367, API-0368, API-0369, CONST-0084, CONST-0085, CONST-0086, CONST-0087.

```rust
#[cfg(feature = "CFNumber")]
#[path = "CFNumber.rs"]
mod __CFNumber;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/lib.rs:24`

File SHA256 `83f4ca0f26d5d8e423f1cb80de4b9f1dd7f7f58b98da8b24832446c6e013f164`. Records: API-0023.

```rust
mod base;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/lib.rs:46`

File SHA256 `83f4ca0f26d5d8e423f1cb80de4b9f1dd7f7f58b98da8b24832446c6e013f164`. Records: API-0024, API-0036, API-0040, API-0047, API-0056, API-0069.

```rust
mod retained;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/lib.rs:47`

File SHA256 `83f4ca0f26d5d8e423f1cb80de4b9f1dd7f7f58b98da8b24832446c6e013f164`. Records: API-0025.

```rust
#[cfg(feature = "CFString")]
mod string;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:122`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0026, API-0027, API-0028, API-0029, API-0370, CONST-0088.

```rust
#[cfg(feature = "CFString")]
#[path = "CFString.rs"]
mod __CFString;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:23`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0030.

```rust
#[cfg(feature = "CGAffineTransform")]
#[path = "CGAffineTransform.rs"]
mod __CGAffineTransform;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/lib.rs:19`

File SHA256 `62a1d4bac29749ef0806aeb2b78e893dcb13dfbaa51157197f23d980187e426b`. Records: API-0031.

```rust
#[cfg(feature = "CGBitmapContext")]
mod bitmap_context;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:155`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0032, API-0212, API-0240, API-0250, API-0251, API-0260, API-0261, API-0313, API-0319, API-0323, API-0526, API-0531, API-0570, API-0624, API-0685, API-0704, CONST-0065.

```rust
#[cfg(feature = "NSColor")]
#[path = "NSColor.rs"]
mod __NSColor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:38`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0033, API-0034, API-0035, API-0376, API-0390, CONST-0093, CONST-0107.

```rust
#[cfg(feature = "CGColorSpace")]
#[path = "CGColorSpace.rs"]
mod __CGColorSpace;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:41`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0037, API-0038, API-0039, API-0041, API-0042, API-0043, API-0044, API-0045, API-0375, API-0471, API-0472, API-0474, CONST-0092.

```rust
#[cfg(feature = "CGContext")]
#[path = "CGContext.rs"]
mod __CGContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:50`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0046.

```rust
#[cfg(feature = "CGDataProvider")]
#[path = "CGDataProvider.rs"]
mod __CGDataProvider;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:53`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0048.

```rust
#[cfg(feature = "CGDirectDisplay")]
#[path = "CGDirectDisplay.rs"]
mod __CGDirectDisplay;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:101`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0049, API-0054, API-0055, API-0374, API-0377, API-0378, CONST-0094, CONST-0095.

```rust
#[cfg(feature = "CGImage")]
#[path = "CGImage.rs"]
mod __CGImage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-image-io-0.3.2/src/generated/mod.rs:26`

File SHA256 `fb60da415c1d94888060e0395aa0b1ef77d05f6f00b382be3efa0e6fc41e037f`. Records: API-0050, API-0051, API-0052, API-0053.

```rust
#[cfg(feature = "CGImageDestination")]
#[path = "CGImageDestination.rs"]
mod __CGImageDestination;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-image-io-0.3.2/src/generated/mod.rs:35`

File SHA256 `fb60da415c1d94888060e0395aa0b1ef77d05f6f00b382be3efa0e6fc41e037f`. Records: API-0057, API-0058, API-0059, API-0060, API-0061.

```rust
#[cfg(feature = "CGImageSource")]
#[path = "CGImageSource.rs"]
mod __CGImageSource;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:143`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0062, API-0063, API-0064, API-0065, API-0066, API-0067, API-0068, API-0385, API-0386, API-0387, API-0388, API-0389, CONST-0102, CONST-0103, CONST-0104, CONST-0105, CONST-0106.

```rust
#[cfg(feature = "CGPath")]
#[path = "CGPath.rs"]
mod __CGPath;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:92`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0070, API-0071, API-0072, API-0073.

```rust
#[cfg(feature = "CGGeometry")]
#[path = "CGGeometry.rs"]
mod __CGGeometry;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:26`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0074, API-0075, API-0078, API-0079, API-0080, API-0658.

```rust
#[cfg(feature = "CTFont")]
#[path = "CTFont.rs"]
mod __CTFont;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:32`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0076, API-0077, API-0392, API-0393, API-0395, API-0397, CONST-0109, CONST-0110, CONST-0111, CONST-0113.

```rust
#[cfg(feature = "CTFontDescriptor")]
#[path = "CTFontDescriptor.rs"]
mod __CTFontDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:53`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0081, API-0082, API-0083.

```rust
#[cfg(feature = "CTLine")]
#[path = "CTLine.rs"]
mod __CTLine;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:62`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0084, API-0085, API-0086, API-0087, API-0088.

```rust
#[cfg(feature = "CTRun")]
#[path = "CTRun.rs"]
mod __CTRun;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:182`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0089, API-0319, API-0323, API-0448, API-0451, API-0452, API-0453.

```rust
#[cfg(feature = "MTLDevice")]
#[path = "MTLDevice.rs"]
mod __MTLDevice;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:299`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0090, API-0091, API-0092.

```rust
#[cfg(feature = "MTLTypes")]
#[path = "MTLTypes.rs"]
mod __MTLTypes;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:38`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0093, API-0094, API-0095, API-0096, API-0098, API-0099, API-0100, API-0103, API-0104, API-0105, API-0106, API-0107, API-0108, API-0109, API-0110, API-0111, API-0112, API-0113, CONST-0008, CONST-0009, CONST-0010, CONST-0011, CONST-0012, CONST-0013, CONST-0014, CONST-0015, CONST-0016, CONST-0017, CONST-0018, CONST-0019, CONST-0020, CONST-0021, CONST-0022.

```rust
#[cfg(feature = "NSAccessibilityConstants")]
#[path = "NSAccessibilityConstants.rs"]
mod __NSAccessibilityConstants;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:50`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0097, API-0143, API-0144, API-0148, API-0149, API-0150, API-0151, API-0152, API-0153, API-0154, API-0155, API-0344, API-0345, API-0346, API-0347, API-0348, API-0535, API-0536.

```rust
#[cfg(feature = "NSAccessibilityProtocols")]
#[path = "NSAccessibilityProtocols.rs"]
mod __NSAccessibilityProtocols;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:32`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0101, API-0102, API-0145, API-0146, API-0147, CONST-0079.

```rust
#[cfg(feature = "NSAccessibility")]
#[path = "NSAccessibility.rs"]
mod __NSAccessibility;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:74`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0114, API-0115, API-0116, API-0209, API-0296, API-0319, API-0323, CONST-0027, CONST-0028.

```rust
#[cfg(feature = "NSAppearance")]
#[path = "NSAppearance.rs"]
mod __NSAppearance;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:365`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0117, API-0319, API-0323, CONST-0039.

```rust
#[cfg(feature = "NSRunLoop")]
#[path = "NSRunLoop.rs"]
mod __NSRunLoop;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:179`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0118, API-0119, API-0121, API-0122, API-0123, API-0124, API-0125, API-0126, API-0127, API-0128, API-0129, CONST-0080, CONST-0081.

```rust
#[cfg(feature = "NSGeometry")]
#[path = "NSGeometry.rs"]
mod __NSGeometry;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:311`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0120, API-0133, API-0134, API-0304, API-0305, API-0343, API-0408, API-0475, API-0666, API-0667, API-0668, API-0669, API-0672, API-0673, API-0674, API-0675, CONST-0032.

```rust
#[cfg(feature = "NSPathUtilities")]
#[path = "NSPathUtilities.rs"]
mod __NSPathUtilities;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:461`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0130, API-0131, API-0132, API-0204, API-0241, API-0274, API-0311, API-0319, API-0323, API-0484, API-0485, API-0559, API-0623, API-0676, CONST-0058, CONST-0059, CONST-0060, CONST-0061.

```rust
#[cfg(feature = "NSPasteboard")]
#[path = "NSPasteboard.rs"]
mod __NSPasteboard;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:413`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0135, API-0138, API-0139, API-0215, API-0275, API-0276, API-0319, API-0323, API-0324, API-0341, API-0349, API-0350, API-0351, API-0414, API-0421, API-0670, API-0671, API-0679, API-0717, CONST-0067.

```rust
#[cfg(feature = "NSString")]
#[path = "NSString.rs"]
mod __NSString;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:545`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0136, API-0137, API-0208, API-0319, API-0323, API-0326, API-0515, API-0517, API-0537, API-0542, API-0543, API-0554, API-0604, API-0718.

```rust
#[cfg(feature = "NSSavePanel")]
#[path = "NSSavePanel.rs"]
mod __NSSavePanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:476`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0140, API-0141, API-0319, API-0323.

```rust
#[cfg(feature = "NSUUID")]
#[path = "NSUUID.rs"]
mod __NSUUID;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:527`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0142, API-0319, API-0323, API-0402, API-0404, API-0424, API-0425, API-0438, API-0439, API-0440, API-0442, API-0443, API-0444, API-0445, API-0512, API-0513, API-0520, API-0521.

```rust
#[cfg(feature = "NSResponder")]
#[path = "NSResponder.rs"]
mod __NSResponder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:356`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0158, API-0217, API-0218, API-0227, API-0228, API-0229, API-0315, API-0319, API-0323, API-0411, API-0412, API-0636, API-0637, API-0638, API-0639, API-0640, API-0695, API-0696, API-0697, API-0698, API-0705, API-0706, CONST-0051, CONST-0052.

```rust
#[cfg(feature = "NSLayoutConstraint")]
#[path = "NSLayoutConstraint.rs"]
mod __NSLayoutConstraint;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:80`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0159, API-0194, API-0195, API-0196, API-0197, API-0295, API-0296, API-0302, API-0308, API-0319, API-0323, API-0405, API-0406, API-0427, API-0428, API-0430, API-0455, API-0507, API-0529, API-0538, API-0560, API-0580, API-0598, API-0617, API-0646, API-0664, API-0716, CONST-0030, CONST-0031.

```rust
#[cfg(feature = "NSApplication")]
#[path = "NSApplication.rs"]
mod __NSApplication;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:638`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0160, API-0161, API-0319, API-0323, API-0327, API-0330, API-0334, API-0540, API-0541, API-0564, API-0567, API-0609, API-0618, CONST-0064.

```rust
#[cfg(feature = "NSStackView")]
#[path = "NSStackView.rs"]
mod __NSStackView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:626`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0161, API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSSplitView")]
#[path = "NSSplitView.rs"]
mod __NSSplitView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:62`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0162, API-0207, API-0319, API-0321, API-0323, API-0514, API-0539, API-0582, API-0602, CONST-0023, CONST-0024, CONST-0025, CONST-0026.

```rust
#[cfg(feature = "NSAlert")]
#[path = "NSAlert.rs"]
mod __NSAlert;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:152`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0163, API-0213, API-0253, API-0319, API-0323, API-0482, API-0504, API-0663, CONST-0001.

```rust
#[cfg(feature = "MTLCommandBuffer")]
#[path = "MTLCommandBuffer.rs"]
mod __MTLCommandBuffer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:380`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0164, API-0319, API-0323, API-0339, API-0361, API-0362, API-0363, API-0435, API-0481, API-0495, API-0561, API-0571, API-0658.

```rust
#[cfg(feature = "NSMenu")]
#[path = "NSMenu.rs"]
mod __NSMenu;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:485`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0165, API-0317, API-0319, API-0323, API-0327, API-0334, API-0335, API-0362, API-0363, API-0527.

```rust
#[cfg(feature = "NSPopUpButton")]
#[path = "NSPopUpButton.rs"]
mod __NSPopUpButton;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:44`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0166, API-0167, API-0198, API-0199, API-0200, API-0270, API-0271, API-0309, API-0319, API-0323, API-0433.

```rust
#[cfg(feature = "NSArray")]
#[path = "NSArray.rs"]
mod __NSArray;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:224`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0168, API-0319, API-0323.

```rust
#[cfg(feature = "NSKeyValueSharedObservers")]
#[path = "NSKeyValueSharedObservers.rs"]
mod __NSKeyValueSharedObservers;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:221`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0168, API-0461, API-0462, API-0501.

```rust
#[cfg(feature = "NSKeyValueObserving")]
#[path = "NSKeyValueObserving.rs"]
mod __NSKeyValueObserving;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:275`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0169, API-0170, API-0278, API-0319, API-0323, API-0499, API-0500.

```rust
#[cfg(feature = "NSNotification")]
#[path = "NSNotification.rs"]
mod __NSNotification;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:860`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0171, API-0172, API-0173, API-0219, API-0220, API-0221, API-0262, API-0264, API-0265, API-0294, API-0316, API-0319, API-0323, API-0327, API-0332, API-0334, API-0354, API-0410, API-0494, API-0496, API-0545, API-0575, API-0576, API-0577, API-0581, API-0593, API-0605, API-0635, API-0645, API-0687, API-0702, API-0703, API-0707, CONST-0034.

```rust
#[cfg(feature = "NSView")]
#[path = "NSView.rs"]
mod __NSView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.5/src/lib.rs:178`

File SHA256 `1a1d7b121bfe025b9aab46d861a9007ac5508ee8773941b7f67f05a44bae6ac7`. Records: API-0174, API-0175, API-0176, API-0177, API-0178, API-0179, API-0180, API-0181, API-0182, API-0183, API-0184, API-0185, API-0186, API-0187, API-0188, API-0189, API-0190, API-0191, API-0232, API-0233, API-0234, API-0235, API-0236, API-0237, API-0238, API-0239.

```rust
mod top_level_traits;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:95`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0192, API-0193, API-0210, API-0211, API-0221, API-0245, API-0307, API-0319, API-0323, API-0415, API-0446, API-0595, API-0596, API-0680.

```rust
#[cfg(feature = "NSBezierPath")]
#[path = "NSBezierPath.rs"]
mod __NSBezierPath;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:197`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0201, API-0246, API-0319, API-0323, API-0336, API-0465, API-0509, API-0510, API-0533.

```rust
#[cfg(feature = "NSCursor")]
#[path = "NSCursor.rs"]
mod __NSCursor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:872`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0202, API-0205, API-0206, API-0226, API-0247, API-0258, API-0259, API-0263, API-0266, API-0319, API-0323, API-0325, API-0326, API-0360, API-0431, API-0432, API-0470, API-0494, API-0519, API-0534, API-0547, API-0555, API-0556, API-0557, API-0562, API-0612, API-0627, API-0628, API-0634, API-0661, API-0691, API-0708, API-0709, API-0710, API-0711, API-0712, API-0713, API-0714, API-0715, CONST-0055, CONST-0056, CONST-0076, CONST-0077, CONST-0078.

```rust
#[cfg(feature = "NSWindow")]
#[path = "NSWindow.rs"]
mod __NSWindow;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:161`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0203, API-0279, API-0303, API-0319, API-0323, API-0359, API-0497, CONST-0045, CONST-0046.

```rust
#[cfg(feature = "NSFileManager")]
#[path = "NSFileManager.rs"]
mod __NSFileManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:548`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0206, API-0280, API-0310, API-0319, API-0323.

```rust
#[cfg(feature = "NSScreen")]
#[path = "NSScreen.rs"]
mod __NSScreen;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:491`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0214, API-0319, API-0323, API-0338, API-0459, API-0460, API-0498, API-0502, API-0550, API-0587, API-0588, API-0607, API-0659, API-0660, API-0683, CONST-0068.

```rust
#[cfg(feature = "NSUserDefaults")]
#[path = "NSUserDefaults.rs"]
mod __NSUserDefaults;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:500`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0215, API-0216, API-0341, API-0701.

```rust
#[cfg(feature = "NSValue")]
#[path = "NSValue.rs"]
mod __NSValue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:359`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0217, API-0218, API-0227, API-0228, API-0229, API-0315, API-0319, API-0323, API-0411, API-0412, API-0695, API-0696, API-0697, API-0698, API-0705, API-0706.

```rust
#[cfg(feature = "NSLayoutGuide")]
#[path = "NSLayoutGuide.rs"]
mod __NSLayoutGuide;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:140`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0221, API-0226, API-0319, API-0323.

```rust
#[cfg(feature = "NSCollectionViewCompositionalLayout")]
#[path = "NSCollectionViewCompositionalLayout.rs"]
mod __NSCollectionViewCompositionalLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:434`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0221, API-0319, API-0323.

```rust
#[cfg(feature = "NSPDFImageRep")]
#[path = "NSPDFImageRep.rs"]
mod __NSPDFImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:710`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0221, API-0319, API-0323.

```rust
#[cfg(feature = "NSTextAttachment")]
#[path = "NSTextAttachment.rs"]
mod __NSTextAttachment;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:53`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0221, API-0316, API-0319, API-0323, API-0546, API-0547, API-0558, API-0574, API-0608.

```rust
#[cfg(feature = "CALayer")]
#[path = "CALayer.rs"]
mod __CALayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:77`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0221, API-0319, API-0323.

```rust
#[cfg(feature = "CARenderer")]
#[path = "CARenderer.rs"]
mod __CARenderer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:254`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0222, API-0230, API-0244, API-0319, API-0323, API-0401, API-0403, API-0418, API-0419, API-0422, API-0423, API-0436, API-0441, API-0522, API-0523, API-0524, API-0525, CONST-0040, CONST-0041, CONST-0042, CONST-0043, CONST-0044.

```rust
#[cfg(feature = "NSEvent")]
#[path = "NSEvent.rs"]
mod __NSEvent;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:110`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0223, API-0231, API-0319, API-0323, API-0327, API-0334, API-0548, API-0589, API-0590, API-0662.

```rust
#[cfg(feature = "NSButton")]
#[path = "NSButton.rs"]
mod __NSButton;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:92`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0224, API-0277, API-0319, API-0323, API-0413.

```rust
#[cfg(feature = "NSData")]
#[path = "NSData.rs"]
mod __NSData;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:590`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0225, API-0319, API-0323, API-0479, API-0625, API-0650, CONST-0063.

```rust
#[cfg(feature = "NSSharingService")]
#[path = "NSSharingService.rs"]
mod __NSSharingService;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:230`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0243, API-0319, API-0323, API-0449, API-0464.

```rust
#[cfg(feature = "NSDocumentController")]
#[path = "NSDocumentController.rs"]
mod __NSDocumentController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:263`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0248, API-0319, API-0323, API-0505, CONST-0002, CONST-0007.

```rust
#[cfg(feature = "MTLRenderPass")]
#[path = "MTLRenderPass.rs"]
mod __MTLRenderPass;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:266`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0249, API-0319, API-0320, API-0323, API-0572, API-0644.

```rust
#[cfg(feature = "MTLRenderPipeline")]
#[path = "MTLRenderPipeline.rs"]
mod __MTLRenderPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:158`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0252, API-0319, API-0323.

```rust
#[cfg(feature = "MTLCommandQueue")]
#[path = "MTLCommandQueue.rs"]
mod __MTLCommandQueue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:353`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0254, API-0255, API-0256, API-0257, API-0319, API-0323.

```rust
#[cfg(feature = "NSLayoutAnchor")]
#[path = "NSLayoutAnchor.rs"]
mod __NSLayoutAnchor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:92`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSBackgroundExtensionView")]
#[path = "NSBackgroundExtensionView.rs"]
mod __NSBackgroundExtensionView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:101`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSBox")]
#[path = "NSBox.rs"]
mod __NSBox;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:224`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "NSDockTile")]
#[path = "NSDockTile.rs"]
mod __NSDockTile;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:245`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323.

```rust
#[cfg(feature = "NSDrawer")]
#[path = "NSDrawer.rs"]
mod __NSDrawer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:293`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSGlassEffectView")]
#[path = "NSGlassEffectView.rs"]
mod __NSGlassEffectView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:311`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSGridView")]
#[path = "NSGridView.rs"]
mod __NSGridView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:551`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323, API-0327, API-0328, API-0334, API-0422, API-0521, API-0547, API-0551, API-0565, API-0579.

```rust
#[cfg(feature = "NSScrollView")]
#[path = "NSScrollView.rs"]
mod __NSScrollView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:740`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0259, API-0319, API-0323.

```rust
#[cfg(feature = "NSTextFinder")]
#[path = "NSTextFinder.rs"]
mod __NSTextFinder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:704`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0267, API-0319, API-0323, API-0327, API-0334, API-0541, API-0547, API-0571.

```rust
#[cfg(feature = "NSText")]
#[path = "NSText.rs"]
mod __NSText;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:113`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0267.

```rust
#[cfg(feature = "NSDecimal")]
#[path = "NSDecimal.rs"]
mod __NSDecimal;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:56`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0267.

```rust
#[cfg(feature = "CTParagraphStyle")]
#[path = "CTParagraphStyle.rs"]
mod __CTParagraphStyle;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:59`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0267.

```rust
#[cfg(feature = "CTRubyAnnotation")]
#[path = "CTRubyAnnotation.rs"]
mod __CTRubyAnnotation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:137`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0269.

```rust
#[cfg(feature = "MTLBlitCommandEncoder")]
#[path = "MTLBlitCommandEncoder.rs"]
mod __MTLBlitCommandEncoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:239`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0273, API-0416, API-0460.

```rust
#[cfg(feature = "NSLocale")]
#[path = "NSLocale.rs"]
mod __NSLocale;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:119`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0281, API-0282, API-0319, API-0323, API-0460.

```rust
#[cfg(feature = "NSDictionary")]
#[path = "NSDictionary.rs"]
mod __NSDictionary;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:95`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0288, API-0319, API-0323.

```rust
#[cfg(feature = "NSDate")]
#[path = "NSDate.rs"]
mod __NSDate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:236`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0289, API-0290, API-0291, API-0292, API-0476.

```rust
#[cfg(feature = "NSDragging")]
#[path = "NSDragging.rs"]
mod __NSDragging;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:260`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0293, API-0573, API-0613, CONST-0005.

```rust
#[cfg(feature = "MTLRenderCommandEncoder")]
#[path = "MTLRenderCommandEncoder.rs"]
mod __MTLRenderCommandEncoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:536`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0294, API-0319, API-0323.

```rust
#[cfg(feature = "NSRulerMarker")]
#[path = "NSRulerMarker.rs"]
mod __NSRulerMarker;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:491`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0296, API-0319, API-0323.

```rust
#[cfg(feature = "NSPopover")]
#[path = "NSPopover.rs"]
mod __NSPopover;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:383`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0299, API-0319, API-0323, API-0340, API-0355, API-0434, API-0532, API-0568, API-0569, API-0591, API-0614, API-0619, API-0626, API-0629, API-0630, API-0631, API-0633, API-0681, API-0682, API-0688.

```rust
#[cfg(feature = "NSMenuItem")]
#[path = "NSMenuItem.rs"]
mod __NSMenuItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:47`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0300, API-0301.

```rust
#[cfg(feature = "MTL4CommandEncoder")]
#[path = "MTL4CommandEncoder.rs"]
mod __MTL4CommandEncoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:155`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0300, API-0301.

```rust
#[cfg(feature = "MTLCommandEncoder")]
#[path = "MTLCommandEncoder.rs"]
mod __MTLCommandEncoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:434`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0304, API-0305, API-0306, API-0314, API-0319, API-0323, API-0352, API-0353, API-0408, API-0471, API-0472, API-0473, API-0474, API-0475, CONST-0066.

```rust
#[cfg(feature = "NSURL")]
#[path = "NSURL.rs"]
mod __NSURL;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:293`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0312, API-0319, API-0323, API-0506, API-0621, API-0622, API-0641, API-0642, API-0690.

```rust
#[cfg(feature = "MTLTexture")]
#[path = "MTLTexture.rs"]
mod __MTLTexture;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:56`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0318, API-0319, API-0323, API-0417, API-0426.

```rust
#[cfg(feature = "NSBundle")]
#[path = "NSBundle.rs"]
mod __NSBundle;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:29`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSATSTypesetter")]
#[path = "NSATSTypesetter.rs"]
mod __NSATSTypesetter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:41`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAccessibilityCustomAction")]
#[path = "NSAccessibilityCustomAction.rs"]
mod __NSAccessibilityCustomAction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:44`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAccessibilityCustomRotor")]
#[path = "NSAccessibilityCustomRotor.rs"]
mod __NSAccessibilityCustomRotor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:47`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAccessibilityElement")]
#[path = "NSAccessibilityElement.rs"]
mod __NSAccessibilityElement;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:53`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0629.

```rust
#[cfg(feature = "NSActionCell")]
#[path = "NSActionCell.rs"]
mod __NSActionCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:56`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAdaptiveImageGlyph")]
#[path = "NSAdaptiveImageGlyph.rs"]
mod __NSAdaptiveImageGlyph;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:65`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAlignmentFeedbackFilter")]
#[path = "NSAlignmentFeedbackFilter.rs"]
mod __NSAlignmentFeedbackFilter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:68`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAnimation")]
#[path = "NSAnimation.rs"]
mod __NSAnimation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:71`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAnimationContext")]
#[path = "NSAnimationContext.rs"]
mod __NSAnimationContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:86`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSArrayController")]
#[path = "NSArrayController.rs"]
mod __NSArrayController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:98`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSBitmapImageRep")]
#[path = "NSBitmapImageRep.rs"]
mod __NSBitmapImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:104`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0471, API-0472, API-0474, API-0547.

```rust
#[cfg(feature = "NSBrowser")]
#[path = "NSBrowser.rs"]
mod __NSBrowser;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:107`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSBrowserCell")]
#[path = "NSBrowserCell.rs"]
mod __NSBrowserCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:113`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547, CONST-0035, CONST-0036.

```rust
#[cfg(feature = "NSButtonCell")]
#[path = "NSButtonCell.rs"]
mod __NSButtonCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:116`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSButtonTouchBarItem")]
#[path = "NSButtonTouchBarItem.rs"]
mod __NSButtonTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:119`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCIImageRep")]
#[path = "NSCIImageRep.rs"]
mod __NSCIImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:122`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCachedImageRep")]
#[path = "NSCachedImageRep.rs"]
mod __NSCachedImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:125`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCandidateListTouchBarItem")]
#[path = "NSCandidateListTouchBarItem.rs"]
mod __NSCandidateListTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:128`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0341, API-0355, API-0541, API-0568, API-0571, API-0594, API-0629, CONST-0037, CONST-0038.

```rust
#[cfg(feature = "NSCell")]
#[path = "NSCell.rs"]
mod __NSCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:131`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSClickGestureRecognizer")]
#[path = "NSClickGestureRecognizer.rs"]
mod __NSClickGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:134`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSClipView")]
#[path = "NSClipView.rs"]
mod __NSClipView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:137`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSCollectionView")]
#[path = "NSCollectionView.rs"]
mod __NSCollectionView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:143`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCollectionViewFlowLayout")]
#[path = "NSCollectionViewFlowLayout.rs"]
mod __NSCollectionViewFlowLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:146`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCollectionViewGridLayout")]
#[path = "NSCollectionViewGridLayout.rs"]
mod __NSCollectionViewGridLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:149`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "NSCollectionViewLayout")]
#[path = "NSCollectionViewLayout.rs"]
mod __NSCollectionViewLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:152`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCollectionViewTransitionLayout")]
#[path = "NSCollectionViewTransitionLayout.rs"]
mod __NSCollectionViewTransitionLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:158`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSColorList")]
#[path = "NSColorList.rs"]
mod __NSColorList;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:161`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0326.

```rust
#[cfg(feature = "NSColorPanel")]
#[path = "NSColorPanel.rs"]
mod __NSColorPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:164`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSColorPicker")]
#[path = "NSColorPicker.rs"]
mod __NSColorPicker;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:167`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSColorPickerTouchBarItem")]
#[path = "NSColorPickerTouchBarItem.rs"]
mod __NSColorPickerTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:173`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSColorSampler")]
#[path = "NSColorSampler.rs"]
mod __NSColorSampler;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:176`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0516.

```rust
#[cfg(feature = "NSColorSpace")]
#[path = "NSColorSpace.rs"]
mod __NSColorSpace;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:179`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSColorWell")]
#[path = "NSColorWell.rs"]
mod __NSColorWell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:182`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSComboBox")]
#[path = "NSComboBox.rs"]
mod __NSComboBox;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:185`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSComboBoxCell")]
#[path = "NSComboBoxCell.rs"]
mod __NSComboBoxCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:188`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSComboButton")]
#[path = "NSComboButton.rs"]
mod __NSComboButton;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:191`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0341, API-0355, API-0541, API-0568, API-0571, API-0594, API-0629.

```rust
#[cfg(feature = "NSControl")]
#[path = "NSControl.rs"]
mod __NSControl;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:194`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSController")]
#[path = "NSController.rs"]
mod __NSController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:200`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCustomImageRep")]
#[path = "NSCustomImageRep.rs"]
mod __NSCustomImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:203`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCustomTouchBarItem")]
#[path = "NSCustomTouchBarItem.rs"]
mod __NSCustomTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:206`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDataAsset")]
#[path = "NSDataAsset.rs"]
mod __NSDataAsset;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:209`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSDatePicker")]
#[path = "NSDatePicker.rs"]
mod __NSDatePicker;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:212`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547.

```rust
#[cfg(feature = "NSDatePickerCell")]
#[path = "NSDatePickerCell.rs"]
mod __NSDatePickerCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:215`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDictionaryController")]
#[path = "NSDictionaryController.rs"]
mod __NSDictionaryController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:218`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDiffableDataSource")]
#[path = "NSDiffableDataSource.rs"]
mod __NSDiffableDataSource;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:227`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDocument")]
#[path = "NSDocument.rs"]
mod __NSDocument;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:239`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDraggingItem")]
#[path = "NSDraggingItem.rs"]
mod __NSDraggingItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:242`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDraggingSession")]
#[path = "NSDraggingSession.rs"]
mod __NSDraggingSession;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:248`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSEPSImageRep")]
#[path = "NSEPSImageRep.rs"]
mod __NSEPSImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:257`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFilePromiseProvider")]
#[path = "NSFilePromiseProvider.rs"]
mod __NSFilePromiseProvider;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:260`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFilePromiseReceiver")]
#[path = "NSFilePromiseReceiver.rs"]
mod __NSFilePromiseReceiver;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:266`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0437, API-0684.

```rust
#[cfg(feature = "NSFont")]
#[path = "NSFont.rs"]
mod __NSFont;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:269`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFontAssetRequest")]
#[path = "NSFontAssetRequest.rs"]
mod __NSFontAssetRequest;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:272`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFontCollection")]
#[path = "NSFontCollection.rs"]
mod __NSFontCollection;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:275`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0460, CONST-0047, CONST-0048, CONST-0049, CONST-0050.

```rust
#[cfg(feature = "NSFontDescriptor")]
#[path = "NSFontDescriptor.rs"]
mod __NSFontDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:278`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSFontManager")]
#[path = "NSFontManager.rs"]
mod __NSFontManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:281`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0326, API-0568.

```rust
#[cfg(feature = "NSFontPanel")]
#[path = "NSFontPanel.rs"]
mod __NSFontPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:284`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0577.

```rust
#[cfg(feature = "NSForm")]
#[path = "NSForm.rs"]
mod __NSForm;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:287`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFormCell")]
#[path = "NSFormCell.rs"]
mod __NSFormCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:290`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0425, API-0439, API-0445, API-0568.

```rust
#[cfg(feature = "NSGestureRecognizer")]
#[path = "NSGestureRecognizer.rs"]
mod __NSGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:296`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSGlyphGenerator")]
#[path = "NSGlyphGenerator.rs"]
mod __NSGlyphGenerator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:299`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSGlyphInfo")]
#[path = "NSGlyphInfo.rs"]
mod __NSGlyphInfo;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:302`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSGradient")]
#[path = "NSGradient.rs"]
mod __NSGradient;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:308`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0354.

```rust
#[cfg(feature = "NSGraphicsContext")]
#[path = "NSGraphicsContext.rs"]
mod __NSGraphicsContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:314`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSGroupTouchBarItem")]
#[path = "NSGroupTouchBarItem.rs"]
mod __NSGroupTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:317`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSHapticFeedback")]
#[path = "NSHapticFeedback.rs"]
mod __NSHapticFeedback;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:320`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0656.

```rust
#[cfg(feature = "NSHelpManager")]
#[path = "NSHelpManager.rs"]
mod __NSHelpManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:323`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0337, API-0354, API-0420, API-0547, API-0658, API-0700.

```rust
#[cfg(feature = "NSImage")]
#[path = "NSImage.rs"]
mod __NSImage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:326`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSImageCell")]
#[path = "NSImageCell.rs"]
mod __NSImageCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:329`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "NSImageRep")]
#[path = "NSImageRep.rs"]
mod __NSImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:332`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSImageView")]
#[path = "NSImageView.rs"]
mod __NSImageView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:335`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSInputManager")]
#[path = "NSInputManager.rs"]
mod __NSInputManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:338`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSInputServer")]
#[path = "NSInputServer.rs"]
mod __NSInputServer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:344`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSItemBadge")]
#[path = "NSItemBadge.rs"]
mod __NSItemBadge;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:350`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSKeyValueBinding")]
#[path = "NSKeyValueBinding.rs"]
mod __NSKeyValueBinding;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:362`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLayoutManager")]
#[path = "NSLayoutManager.rs"]
mod __NSLayoutManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:365`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSLevelIndicator")]
#[path = "NSLevelIndicator.rs"]
mod __NSLevelIndicator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:368`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLevelIndicatorCell")]
#[path = "NSLevelIndicatorCell.rs"]
mod __NSLevelIndicatorCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:371`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0422.

```rust
#[cfg(feature = "NSMagnificationGestureRecognizer")]
#[path = "NSMagnificationGestureRecognizer.rs"]
mod __NSMagnificationGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:374`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0439, API-0547.

```rust
#[cfg(feature = "NSMatrix")]
#[path = "NSMatrix.rs"]
mod __NSMatrix;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:377`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMediaLibraryBrowserController")]
#[path = "NSMediaLibraryBrowserController.rs"]
mod __NSMediaLibraryBrowserController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:386`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMenuItemBadge")]
#[path = "NSMenuItemBadge.rs"]
mod __NSMenuItemBadge;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:389`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0605, API-0629.

```rust
#[cfg(feature = "NSMenuItemCell")]
#[path = "NSMenuItemCell.rs"]
mod __NSMenuItemCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:392`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMenuToolbarItem")]
#[path = "NSMenuToolbarItem.rs"]
mod __NSMenuToolbarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:395`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMovie")]
#[path = "NSMovie.rs"]
mod __NSMovie;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:398`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNib")]
#[path = "NSNib.rs"]
mod __NSNib;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:401`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNibConnector")]
#[path = "NSNibConnector.rs"]
mod __NSNibConnector;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:404`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNibControlConnector")]
#[path = "NSNibControlConnector.rs"]
mod __NSNibControlConnector;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:413`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNibOutletConnector")]
#[path = "NSNibOutletConnector.rs"]
mod __NSNibOutletConnector;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:416`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSObjectController")]
#[path = "NSObjectController.rs"]
mod __NSObjectController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:419`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOpenGL")]
#[path = "NSOpenGL.rs"]
mod __NSOpenGL;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:422`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOpenGLLayer")]
#[path = "NSOpenGLLayer.rs"]
mod __NSOpenGLLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:425`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSOpenGLView")]
#[path = "NSOpenGLView.rs"]
mod __NSOpenGLView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:428`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0326, API-0466, API-0544, API-0552, API-0553, API-0615.

```rust
#[cfg(feature = "NSOpenPanel")]
#[path = "NSOpenPanel.rs"]
mod __NSOpenPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:431`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSOutlineView")]
#[path = "NSOutlineView.rs"]
mod __NSOutlineView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:437`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPDFInfo")]
#[path = "NSPDFInfo.rs"]
mod __NSPDFInfo;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:440`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPDFPanel")]
#[path = "NSPDFPanel.rs"]
mod __NSPDFPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:443`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPICTImageRep")]
#[path = "NSPICTImageRep.rs"]
mod __NSPICTImageRep;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:446`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPageController")]
#[path = "NSPageController.rs"]
mod __NSPageController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:449`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPageLayout")]
#[path = "NSPageLayout.rs"]
mod __NSPageLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:452`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPanGestureRecognizer")]
#[path = "NSPanGestureRecognizer.rs"]
mod __NSPanGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:455`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0326.

```rust
#[cfg(feature = "NSPanel")]
#[path = "NSPanel.rs"]
mod __NSPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:458`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0541, API-0594, CONST-0053, CONST-0054.

```rust
#[cfg(feature = "NSParagraphStyle")]
#[path = "NSParagraphStyle.rs"]
mod __NSParagraphStyle;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:464`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPasteboardItem")]
#[path = "NSPasteboardItem.rs"]
mod __NSPasteboardItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:467`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547.

```rust
#[cfg(feature = "NSPathCell")]
#[path = "NSPathCell.rs"]
mod __NSPathCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:470`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPathComponentCell")]
#[path = "NSPathComponentCell.rs"]
mod __NSPathComponentCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:473`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSPathControl")]
#[path = "NSPathControl.rs"]
mod __NSPathControl;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:476`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPathControlItem")]
#[path = "NSPathControlItem.rs"]
mod __NSPathControlItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:479`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPersistentDocument")]
#[path = "NSPersistentDocument.rs"]
mod __NSPersistentDocument;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:482`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSPickerTouchBarItem")]
#[path = "NSPickerTouchBarItem.rs"]
mod __NSPickerTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:488`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0362, API-0363.

```rust
#[cfg(feature = "NSPopUpButtonCell")]
#[path = "NSPopUpButtonCell.rs"]
mod __NSPopUpButtonCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:494`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPopoverTouchBarItem")]
#[path = "NSPopoverTouchBarItem.rs"]
mod __NSPopoverTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:497`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSPredicateEditor")]
#[path = "NSPredicateEditor.rs"]
mod __NSPredicateEditor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:500`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPredicateEditorRowTemplate")]
#[path = "NSPredicateEditorRowTemplate.rs"]
mod __NSPredicateEditorRowTemplate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:503`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPressGestureRecognizer")]
#[path = "NSPressGestureRecognizer.rs"]
mod __NSPressGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:506`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPressureConfiguration")]
#[path = "NSPressureConfiguration.rs"]
mod __NSPressureConfiguration;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:509`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPreviewRepresentingActivityItem")]
#[path = "NSPreviewRepresentingActivityItem.rs"]
mod __NSPreviewRepresentingActivityItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:512`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPrintInfo")]
#[path = "NSPrintInfo.rs"]
mod __NSPrintInfo;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:515`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPrintOperation")]
#[path = "NSPrintOperation.rs"]
mod __NSPrintOperation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:518`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPrintPanel")]
#[path = "NSPrintPanel.rs"]
mod __NSPrintPanel;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:521`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPrinter")]
#[path = "NSPrinter.rs"]
mod __NSPrinter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:524`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSProgressIndicator")]
#[path = "NSProgressIndicator.rs"]
mod __NSProgressIndicator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:530`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSRotationGestureRecognizer")]
#[path = "NSRotationGestureRecognizer.rs"]
mod __NSRotationGestureRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:533`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSRuleEditor")]
#[path = "NSRuleEditor.rs"]
mod __NSRuleEditor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:539`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0354.

```rust
#[cfg(feature = "NSRulerView")]
#[path = "NSRulerView.rs"]
mod __NSRulerView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:542`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, CONST-0029.

```rust
#[cfg(feature = "NSRunningApplication")]
#[path = "NSRunningApplication.rs"]
mod __NSRunningApplication;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:554`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSScroller")]
#[path = "NSScroller.rs"]
mod __NSScroller;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:557`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSScrubber")]
#[path = "NSScrubber.rs"]
mod __NSScrubber;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:560`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0355.

```rust
#[cfg(feature = "NSScrubberItemView")]
#[path = "NSScrubberItemView.rs"]
mod __NSScrubberItemView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:563`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScrubberLayout")]
#[path = "NSScrubberLayout.rs"]
mod __NSScrubberLayout;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:566`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSSearchField")]
#[path = "NSSearchField.rs"]
mod __NSSearchField;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:569`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSearchFieldCell")]
#[path = "NSSearchFieldCell.rs"]
mod __NSSearchFieldCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:572`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSearchToolbarItem")]
#[path = "NSSearchToolbarItem.rs"]
mod __NSSearchToolbarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:575`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSSecureTextField")]
#[path = "NSSecureTextField.rs"]
mod __NSSecureTextField;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:578`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0592, API-0616.

```rust
#[cfg(feature = "NSSegmentedCell")]
#[path = "NSSegmentedCell.rs"]
mod __NSSegmentedCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:581`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0329, API-0334, API-0592, API-0616, CONST-0062.

```rust
#[cfg(feature = "NSSegmentedControl")]
#[path = "NSSegmentedControl.rs"]
mod __NSSegmentedControl;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:584`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSShadow")]
#[path = "NSShadow.rs"]
mod __NSShadow;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:587`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSharingCollaborationModeRestriction")]
#[path = "NSSharingCollaborationModeRestriction.rs"]
mod __NSSharingCollaborationModeRestriction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:593`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSharingServicePickerToolbarItem")]
#[path = "NSSharingServicePickerToolbarItem.rs"]
mod __NSSharingServicePickerToolbarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:596`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSSharingServicePickerTouchBarItem")]
#[path = "NSSharingServicePickerTouchBarItem.rs"]
mod __NSSharingServicePickerTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:599`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSSlider")]
#[path = "NSSlider.rs"]
mod __NSSlider;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:602`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSSliderAccessory")]
#[path = "NSSliderAccessory.rs"]
mod __NSSliderAccessory;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:605`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSliderCell")]
#[path = "NSSliderCell.rs"]
mod __NSSliderCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:608`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSliderTouchBarItem")]
#[path = "NSSliderTouchBarItem.rs"]
mod __NSSliderTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:611`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSound")]
#[path = "NSSound.rs"]
mod __NSSound;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:614`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSpeechRecognizer")]
#[path = "NSSpeechRecognizer.rs"]
mod __NSSpeechRecognizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:617`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSpeechSynthesizer")]
#[path = "NSSpeechSynthesizer.rs"]
mod __NSSpeechSynthesizer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:620`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSpellChecker")]
#[path = "NSSpellChecker.rs"]
mod __NSSpellChecker;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:629`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0692.

```rust
#[cfg(feature = "NSSplitViewController")]
#[path = "NSSplitViewController.rs"]
mod __NSSplitViewController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:632`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSplitViewItem")]
#[path = "NSSplitViewItem.rs"]
mod __NSSplitViewItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:635`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSplitViewItemAccessoryViewController")]
#[path = "NSSplitViewItemAccessoryViewController.rs"]
mod __NSSplitViewItemAccessoryViewController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:641`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStatusBar")]
#[path = "NSStatusBar.rs"]
mod __NSStatusBar;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:644`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSStatusBarButton")]
#[path = "NSStatusBarButton.rs"]
mod __NSStatusBarButton;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:647`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "NSStatusItem")]
#[path = "NSStatusItem.rs"]
mod __NSStatusItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:650`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSStepper")]
#[path = "NSStepper.rs"]
mod __NSStepper;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:653`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStepperCell")]
#[path = "NSStepperCell.rs"]
mod __NSStepperCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:656`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStepperTouchBarItem")]
#[path = "NSStepperTouchBarItem.rs"]
mod __NSStepperTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:659`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStoryboard")]
#[path = "NSStoryboard.rs"]
mod __NSStoryboard;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:662`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStoryboardSegue")]
#[path = "NSStoryboardSegue.rs"]
mod __NSStoryboardSegue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:665`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "NSStringDrawing")]
#[path = "NSStringDrawing.rs"]
mod __NSStringDrawing;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:668`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSSwitch")]
#[path = "NSSwitch.rs"]
mod __NSSwitch;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:671`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0571.

```rust
#[cfg(feature = "NSTabView")]
#[path = "NSTabView.rs"]
mod __NSTabView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:674`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTabViewController")]
#[path = "NSTabViewController.rs"]
mod __NSTabViewController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:677`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTabViewItem")]
#[path = "NSTabViewItem.rs"]
mod __NSTabViewItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:680`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSTableCellView")]
#[path = "NSTableCellView.rs"]
mod __NSTableCellView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:683`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTableColumn")]
#[path = "NSTableColumn.rs"]
mod __NSTableColumn;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:686`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTableHeaderCell")]
#[path = "NSTableHeaderCell.rs"]
mod __NSTableHeaderCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:689`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSTableHeaderView")]
#[path = "NSTableHeaderView.rs"]
mod __NSTableHeaderView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:692`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSTableRowView")]
#[path = "NSTableRowView.rs"]
mod __NSTableRowView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:695`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0547.

```rust
#[cfg(feature = "NSTableView")]
#[path = "NSTableView.rs"]
mod __NSTableView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:698`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTableViewDiffableDataSource")]
#[path = "NSTableViewDiffableDataSource.rs"]
mod __NSTableViewDiffableDataSource;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:701`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547.

```rust
#[cfg(feature = "NSTableViewRowAction")]
#[path = "NSTableViewRowAction.rs"]
mod __NSTableViewRowAction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:707`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextAlternatives")]
#[path = "NSTextAlternatives.rs"]
mod __NSTextAlternatives;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:713`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextAttachmentCell")]
#[path = "NSTextAttachmentCell.rs"]
mod __NSTextAttachmentCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:719`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextCheckingController")]
#[path = "NSTextCheckingController.rs"]
mod __NSTextCheckingController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:722`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0594, API-0658.

```rust
#[cfg(feature = "NSTextContainer")]
#[path = "NSTextContainer.rs"]
mod __NSTextContainer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:728`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextContentManager")]
#[path = "NSTextContentManager.rs"]
mod __NSTextContentManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:731`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextElement")]
#[path = "NSTextElement.rs"]
mod __NSTextElement;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:734`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334, API-0407, API-0547, API-0601, API-0632.

```rust
#[cfg(feature = "NSTextField")]
#[path = "NSTextField.rs"]
mod __NSTextField;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:737`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547.

```rust
#[cfg(feature = "NSTextFieldCell")]
#[path = "NSTextFieldCell.rs"]
mod __NSTextFieldCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:746`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextInputContext")]
#[path = "NSTextInputContext.rs"]
mod __NSTextInputContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:749`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSTextInsertionIndicator")]
#[path = "NSTextInsertionIndicator.rs"]
mod __NSTextInsertionIndicator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:752`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextLayoutFragment")]
#[path = "NSTextLayoutFragment.rs"]
mod __NSTextLayoutFragment;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:755`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextLayoutManager")]
#[path = "NSTextLayoutManager.rs"]
mod __NSTextLayoutManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:758`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextLineFragment")]
#[path = "NSTextLineFragment.rs"]
mod __NSTextLineFragment;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:761`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextList")]
#[path = "NSTextList.rs"]
mod __NSTextList;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:764`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextListElement")]
#[path = "NSTextListElement.rs"]
mod __NSTextListElement;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:767`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextRange")]
#[path = "NSTextRange.rs"]
mod __NSTextRange;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:770`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextSelection")]
#[path = "NSTextSelection.rs"]
mod __NSTextSelection;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:773`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextSelectionNavigation")]
#[path = "NSTextSelectionNavigation.rs"]
mod __NSTextSelectionNavigation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:776`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextStorage")]
#[path = "NSTextStorage.rs"]
mod __NSTextStorage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:782`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0547.

```rust
#[cfg(feature = "NSTextTable")]
#[path = "NSTextTable.rs"]
mod __NSTextTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:785`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0331, API-0334, API-0547.

```rust
#[cfg(feature = "NSTextView")]
#[path = "NSTextView.rs"]
mod __NSTextView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:788`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextViewportLayoutController")]
#[path = "NSTextViewportLayoutController.rs"]
mod __NSTextViewportLayoutController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:791`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTintConfiguration")]
#[path = "NSTintConfiguration.rs"]
mod __NSTintConfiguration;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:797`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTitlebarAccessoryViewController")]
#[path = "NSTitlebarAccessoryViewController.rs"]
mod __NSTitlebarAccessoryViewController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:800`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0334.

```rust
#[cfg(feature = "NSTokenField")]
#[path = "NSTokenField.rs"]
mod __NSTokenField;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:803`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTokenFieldCell")]
#[path = "NSTokenFieldCell.rs"]
mod __NSTokenFieldCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:806`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSToolbar")]
#[path = "NSToolbar.rs"]
mod __NSToolbar;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:809`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0568, API-0629.

```rust
#[cfg(feature = "NSToolbarItem")]
#[path = "NSToolbarItem.rs"]
mod __NSToolbarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:812`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSToolbarItemGroup")]
#[path = "NSToolbarItemGroup.rs"]
mod __NSToolbarItemGroup;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:815`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTouch")]
#[path = "NSTouch.rs"]
mod __NSTouch;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:818`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTouchBar")]
#[path = "NSTouchBar.rs"]
mod __NSTouchBar;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:821`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTouchBarItem")]
#[path = "NSTouchBarItem.rs"]
mod __NSTouchBarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:824`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTrackingArea")]
#[path = "NSTrackingArea.rs"]
mod __NSTrackingArea;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:827`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTrackingSeparatorToolbarItem")]
#[path = "NSTrackingSeparatorToolbarItem.rs"]
mod __NSTrackingSeparatorToolbarItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:830`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTreeController")]
#[path = "NSTreeController.rs"]
mod __NSTreeController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:833`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTreeNode")]
#[path = "NSTreeNode.rs"]
mod __NSTreeNode;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:836`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTypesetter")]
#[path = "NSTypesetter.rs"]
mod __NSTypesetter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:842`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUserDefaultsController")]
#[path = "NSUserDefaultsController.rs"]
mod __NSUserDefaultsController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:845`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUserInterfaceCompression")]
#[path = "NSUserInterfaceCompression.rs"]
mod __NSUserInterfaceCompression;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:863`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSViewController")]
#[path = "NSViewController.rs"]
mod __NSViewController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:866`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSViewLayoutRegion")]
#[path = "NSViewLayoutRegion.rs"]
mod __NSViewLayoutRegion;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:869`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0327, API-0333, API-0334, API-0549, API-0599, API-0620, CONST-0071, CONST-0072, CONST-0073.

```rust
#[cfg(feature = "NSVisualEffectView")]
#[path = "NSVisualEffectView.rs"]
mod __NSVisualEffectView;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:875`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWindowController")]
#[path = "NSWindowController.rs"]
mod __NSWindowController;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:884`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWindowTab")]
#[path = "NSWindowTab.rs"]
mod __NSWindowTab;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:887`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWindowTabGroup")]
#[path = "NSWindowTabGroup.rs"]
mod __NSWindowTabGroup;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:890`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323, API-0456, API-0648, API-0649.

```rust
#[cfg(feature = "NSWorkspace")]
#[path = "NSWorkspace.rs"]
mod __NSWorkspace;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:893`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWritingToolsCoordinator")]
#[path = "NSWritingToolsCoordinator.rs"]
mod __NSWritingToolsCoordinator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:896`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWritingToolsCoordinatorAnimationParameters")]
#[path = "NSWritingToolsCoordinatorAnimationParameters.rs"]
mod __NSWritingToolsCoordinatorAnimationParameters;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:899`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSWritingToolsCoordinatorContext")]
#[path = "NSWritingToolsCoordinatorContext.rs"]
mod __NSWritingToolsCoordinatorContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:29`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAffineTransform")]
#[path = "NSAffineTransform.rs"]
mod __NSAffineTransform;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:32`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAppleEventDescriptor")]
#[path = "NSAppleEventDescriptor.rs"]
mod __NSAppleEventDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:35`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAppleEventManager")]
#[path = "NSAppleEventManager.rs"]
mod __NSAppleEventManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:38`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAppleScript")]
#[path = "NSAppleScript.rs"]
mod __NSAppleScript;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:41`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSArchiver")]
#[path = "NSArchiver.rs"]
mod __NSArchiver;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:47`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAttributedString")]
#[path = "NSAttributedString.rs"]
mod __NSAttributedString;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:50`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSAutoreleasePool")]
#[path = "NSAutoreleasePool.rs"]
mod __NSAutoreleasePool;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:53`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSBackgroundActivityScheduler")]
#[path = "NSBackgroundActivityScheduler.rs"]
mod __NSBackgroundActivityScheduler;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:59`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSByteCountFormatter")]
#[path = "NSByteCountFormatter.rs"]
mod __NSByteCountFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:65`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0460.

```rust
#[cfg(feature = "NSCache")]
#[path = "NSCache.rs"]
mod __NSCache;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:68`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCalendar")]
#[path = "NSCalendar.rs"]
mod __NSCalendar;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:71`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCalendarDate")]
#[path = "NSCalendarDate.rs"]
mod __NSCalendarDate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:74`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCharacterSet")]
#[path = "NSCharacterSet.rs"]
mod __NSCharacterSet;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:77`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSClassDescription")]
#[path = "NSClassDescription.rs"]
mod __NSClassDescription;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:80`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCoder")]
#[path = "NSCoder.rs"]
mod __NSCoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:83`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSComparisonPredicate")]
#[path = "NSComparisonPredicate.rs"]
mod __NSComparisonPredicate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:86`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSCompoundPredicate")]
#[path = "NSCompoundPredicate.rs"]
mod __NSCompoundPredicate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:89`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSConnection")]
#[path = "NSConnection.rs"]
mod __NSConnection;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:98`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDateComponentsFormatter")]
#[path = "NSDateComponentsFormatter.rs"]
mod __NSDateComponentsFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:101`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDateFormatter")]
#[path = "NSDateFormatter.rs"]
mod __NSDateFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:104`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDateInterval")]
#[path = "NSDateInterval.rs"]
mod __NSDateInterval;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:107`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDateIntervalFormatter")]
#[path = "NSDateIntervalFormatter.rs"]
mod __NSDateIntervalFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:116`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDecimalNumber")]
#[path = "NSDecimalNumber.rs"]
mod __NSDecimalNumber;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:125`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDistributedLock")]
#[path = "NSDistributedLock.rs"]
mod __NSDistributedLock;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:128`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSDistributedNotificationCenter")]
#[path = "NSDistributedNotificationCenter.rs"]
mod __NSDistributedNotificationCenter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:131`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSEnergyFormatter")]
#[path = "NSEnergyFormatter.rs"]
mod __NSEnergyFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:134`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSEnumerator")]
#[path = "NSEnumerator.rs"]
mod __NSEnumerator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:137`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSError")]
#[path = "NSError.rs"]
mod __NSError;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:140`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSException")]
#[path = "NSException.rs"]
mod __NSException;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:143`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSExpression")]
#[path = "NSExpression.rs"]
mod __NSExpression;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:146`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSExtensionContext")]
#[path = "NSExtensionContext.rs"]
mod __NSExtensionContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:149`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSExtensionItem")]
#[path = "NSExtensionItem.rs"]
mod __NSExtensionItem;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:155`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFileCoordinator")]
#[path = "NSFileCoordinator.rs"]
mod __NSFileCoordinator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:158`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFileHandle")]
#[path = "NSFileHandle.rs"]
mod __NSFileHandle;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:167`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFileVersion")]
#[path = "NSFileVersion.rs"]
mod __NSFileVersion;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:170`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFileWrapper")]
#[path = "NSFileWrapper.rs"]
mod __NSFileWrapper;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:173`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSFormatter")]
#[path = "NSFormatter.rs"]
mod __NSFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:176`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSGarbageCollector")]
#[path = "NSGarbageCollector.rs"]
mod __NSGarbageCollector;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:185`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0471, API-0472, API-0474.

```rust
#[cfg(feature = "NSHTTPCookie")]
#[path = "NSHTTPCookie.rs"]
mod __NSHTTPCookie;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:188`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSHTTPCookieStorage")]
#[path = "NSHTTPCookieStorage.rs"]
mod __NSHTTPCookieStorage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:191`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSHashTable")]
#[path = "NSHashTable.rs"]
mod __NSHashTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:194`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSHost")]
#[path = "NSHost.rs"]
mod __NSHost;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:197`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSISO8601DateFormatter")]
#[path = "NSISO8601DateFormatter.rs"]
mod __NSISO8601DateFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:200`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSIndexPath")]
#[path = "NSIndexPath.rs"]
mod __NSIndexPath;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:203`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSIndexSet")]
#[path = "NSIndexSet.rs"]
mod __NSIndexSet;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:206`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSInflectionRule")]
#[path = "NSInflectionRule.rs"]
mod __NSInflectionRule;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:209`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSInvocation")]
#[path = "NSInvocation.rs"]
mod __NSInvocation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:212`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSItemProvider")]
#[path = "NSItemProvider.rs"]
mod __NSItemProvider;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:215`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSJSONSerialization")]
#[path = "NSJSONSerialization.rs"]
mod __NSJSONSerialization;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:227`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSKeyedArchiver")]
#[path = "NSKeyedArchiver.rs"]
mod __NSKeyedArchiver;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:230`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLengthFormatter")]
#[path = "NSLengthFormatter.rs"]
mod __NSLengthFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:233`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLinguisticTagger")]
#[path = "NSLinguisticTagger.rs"]
mod __NSLinguisticTagger;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:236`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSListFormatter")]
#[path = "NSListFormatter.rs"]
mod __NSListFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:242`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLocalizedNumberFormatRule")]
#[path = "NSLocalizedNumberFormatRule.rs"]
mod __NSLocalizedNumberFormatRule;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:245`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSLock")]
#[path = "NSLock.rs"]
mod __NSLock;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:248`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0460.

```rust
#[cfg(feature = "NSMapTable")]
#[path = "NSMapTable.rs"]
mod __NSMapTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:251`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMassFormatter")]
#[path = "NSMassFormatter.rs"]
mod __NSMassFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:254`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMeasurement")]
#[path = "NSMeasurement.rs"]
mod __NSMeasurement;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:257`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMeasurementFormatter")]
#[path = "NSMeasurementFormatter.rs"]
mod __NSMeasurementFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:260`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMetadata")]
#[path = "NSMetadata.rs"]
mod __NSMetadata;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:266`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMethodSignature")]
#[path = "NSMethodSignature.rs"]
mod __NSMethodSignature;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:269`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSMorphology")]
#[path = "NSMorphology.rs"]
mod __NSMorphology;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:272`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNetServices")]
#[path = "NSNetServices.rs"]
mod __NSNetServices;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:278`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNotificationQueue")]
#[path = "NSNotificationQueue.rs"]
mod __NSNotificationQueue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:281`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSNull")]
#[path = "NSNull.rs"]
mod __NSNull;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:284`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0322, API-0323, API-0597, API-0600, API-0603, API-0606, API-0643, API-0677, CONST-0057.

```rust
#[cfg(feature = "NSNumberFormatter")]
#[path = "NSNumberFormatter.rs"]
mod __NSNumberFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:296`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0429.

```rust
#[cfg(feature = "NSOperation")]
#[path = "NSOperation.rs"]
mod __NSOperation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:299`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOrderedCollectionChange")]
#[path = "NSOrderedCollectionChange.rs"]
mod __NSOrderedCollectionChange;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:302`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOrderedCollectionDifference")]
#[path = "NSOrderedCollectionDifference.rs"]
mod __NSOrderedCollectionDifference;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:305`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOrderedSet")]
#[path = "NSOrderedSet.rs"]
mod __NSOrderedSet;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:308`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSOrthography")]
#[path = "NSOrthography.rs"]
mod __NSOrthography;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:314`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPersonNameComponents")]
#[path = "NSPersonNameComponents.rs"]
mod __NSPersonNameComponents;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:317`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPersonNameComponentsFormatter")]
#[path = "NSPersonNameComponentsFormatter.rs"]
mod __NSPersonNameComponentsFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:320`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPointerArray")]
#[path = "NSPointerArray.rs"]
mod __NSPointerArray;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:323`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPointerFunctions")]
#[path = "NSPointerFunctions.rs"]
mod __NSPointerFunctions;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:326`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPort")]
#[path = "NSPort.rs"]
mod __NSPort;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:329`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPortCoder")]
#[path = "NSPortCoder.rs"]
mod __NSPortCoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:332`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPortMessage")]
#[path = "NSPortMessage.rs"]
mod __NSPortMessage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:335`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPortNameServer")]
#[path = "NSPortNameServer.rs"]
mod __NSPortNameServer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:338`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPredicate")]
#[path = "NSPredicate.rs"]
mod __NSPredicate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:341`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0469, API-0483, API-0686.

```rust
#[cfg(feature = "NSProcessInfo")]
#[path = "NSProcessInfo.rs"]
mod __NSProcessInfo;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:344`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSProgress")]
#[path = "NSProgress.rs"]
mod __NSProgress;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:347`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSPropertyList")]
#[path = "NSPropertyList.rs"]
mod __NSPropertyList;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:359`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSRegularExpression")]
#[path = "NSRegularExpression.rs"]
mod __NSRegularExpression;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:362`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSRelativeDateTimeFormatter")]
#[path = "NSRelativeDateTimeFormatter.rs"]
mod __NSRelativeDateTimeFormatter;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:368`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScanner")]
#[path = "NSScanner.rs"]
mod __NSScanner;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:371`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptClassDescription")]
#[path = "NSScriptClassDescription.rs"]
mod __NSScriptClassDescription;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:374`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptCoercionHandler")]
#[path = "NSScriptCoercionHandler.rs"]
mod __NSScriptCoercionHandler;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:377`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptCommand")]
#[path = "NSScriptCommand.rs"]
mod __NSScriptCommand;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:380`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptCommandDescription")]
#[path = "NSScriptCommandDescription.rs"]
mod __NSScriptCommandDescription;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:383`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptExecutionContext")]
#[path = "NSScriptExecutionContext.rs"]
mod __NSScriptExecutionContext;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:389`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptObjectSpecifiers")]
#[path = "NSScriptObjectSpecifiers.rs"]
mod __NSScriptObjectSpecifiers;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:392`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptStandardSuiteCommands")]
#[path = "NSScriptStandardSuiteCommands.rs"]
mod __NSScriptStandardSuiteCommands;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:395`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptSuiteRegistry")]
#[path = "NSScriptSuiteRegistry.rs"]
mod __NSScriptSuiteRegistry;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:398`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSScriptWhoseTests")]
#[path = "NSScriptWhoseTests.rs"]
mod __NSScriptWhoseTests;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:401`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0433.

```rust
#[cfg(feature = "NSSet")]
#[path = "NSSet.rs"]
mod __NSSet;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:404`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSortDescriptor")]
#[path = "NSSortDescriptor.rs"]
mod __NSSortDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:407`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSSpellServer")]
#[path = "NSSpellServer.rs"]
mod __NSSpellServer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:410`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSStream")]
#[path = "NSStream.rs"]
mod __NSStream;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:416`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTask")]
#[path = "NSTask.rs"]
mod __NSTask;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:419`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTermOfAddress")]
#[path = "NSTermOfAddress.rs"]
mod __NSTermOfAddress;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:422`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTextCheckingResult")]
#[path = "NSTextCheckingResult.rs"]
mod __NSTextCheckingResult;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:425`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0358.

```rust
#[cfg(feature = "NSThread")]
#[path = "NSThread.rs"]
mod __NSThread;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:428`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSTimeZone")]
#[path = "NSTimeZone.rs"]
mod __NSTimeZone;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:431`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0342, API-0518.

```rust
#[cfg(feature = "NSTimer")]
#[path = "NSTimer.rs"]
mod __NSTimer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:437`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLAuthenticationChallenge")]
#[path = "NSURLAuthenticationChallenge.rs"]
mod __NSURLAuthenticationChallenge;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:440`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLCache")]
#[path = "NSURLCache.rs"]
mod __NSURLCache;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:443`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLConnection")]
#[path = "NSURLConnection.rs"]
mod __NSURLConnection;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:446`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLCredential")]
#[path = "NSURLCredential.rs"]
mod __NSURLCredential;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:449`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLCredentialStorage")]
#[path = "NSURLCredentialStorage.rs"]
mod __NSURLCredentialStorage;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:452`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLDownload")]
#[path = "NSURLDownload.rs"]
mod __NSURLDownload;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:458`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLHandle")]
#[path = "NSURLHandle.rs"]
mod __NSURLHandle;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:461`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLProtectionSpace")]
#[path = "NSURLProtectionSpace.rs"]
mod __NSURLProtectionSpace;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:464`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLProtocol")]
#[path = "NSURLProtocol.rs"]
mod __NSURLProtocol;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:467`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLRequest")]
#[path = "NSURLRequest.rs"]
mod __NSURLRequest;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:470`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLResponse")]
#[path = "NSURLResponse.rs"]
mod __NSURLResponse;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:473`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSURLSession")]
#[path = "NSURLSession.rs"]
mod __NSURLSession;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:479`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323, API-0460.

```rust
#[cfg(feature = "NSUbiquitousKeyValueStore")]
#[path = "NSUbiquitousKeyValueStore.rs"]
mod __NSUbiquitousKeyValueStore;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:482`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUndoManager")]
#[path = "NSUndoManager.rs"]
mod __NSUndoManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:485`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUnit")]
#[path = "NSUnit.rs"]
mod __NSUnit;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:488`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUserActivity")]
#[path = "NSUserActivity.rs"]
mod __NSUserActivity;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:494`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUserNotification")]
#[path = "NSUserNotification.rs"]
mod __NSUserNotification;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:497`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSUserScriptTask")]
#[path = "NSUserScriptTask.rs"]
mod __NSUserScriptTask;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:503`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSValueTransformer")]
#[path = "NSValueTransformer.rs"]
mod __NSValueTransformer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:506`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLDTD")]
#[path = "NSXMLDTD.rs"]
mod __NSXMLDTD;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:509`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLDTDNode")]
#[path = "NSXMLDTDNode.rs"]
mod __NSXMLDTDNode;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:512`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLDocument")]
#[path = "NSXMLDocument.rs"]
mod __NSXMLDocument;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:515`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLElement")]
#[path = "NSXMLElement.rs"]
mod __NSXMLElement;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:518`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLNode")]
#[path = "NSXMLNode.rs"]
mod __NSXMLNode;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:524`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXMLParser")]
#[path = "NSXMLParser.rs"]
mod __NSXMLParser;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/generated/mod.rs:527`

File SHA256 `b7239ddd878de2ae07aedf8bb2c68ed0def17d42da4ddfce418ab7f814d841bb`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "NSXPCConnection")]
#[path = "NSXPCConnection.rs"]
mod __NSXPCConnection;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:23`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4AccelerationStructure")]
#[path = "MTL4AccelerationStructure.rs"]
mod __MTL4AccelerationStructure;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:29`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4ArgumentTable")]
#[path = "MTL4ArgumentTable.rs"]
mod __MTL4ArgumentTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:35`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4BinaryFunctionDescriptor")]
#[path = "MTL4BinaryFunctionDescriptor.rs"]
mod __MTL4BinaryFunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:41`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4CommandAllocator")]
#[path = "MTL4CommandAllocator.rs"]
mod __MTL4CommandAllocator;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:44`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4CommandBuffer")]
#[path = "MTL4CommandBuffer.rs"]
mod __MTL4CommandBuffer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:50`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4CommandQueue")]
#[path = "MTL4CommandQueue.rs"]
mod __MTL4CommandQueue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:56`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4Compiler")]
#[path = "MTL4Compiler.rs"]
mod __MTL4Compiler;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:65`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4ComputePipeline")]
#[path = "MTL4ComputePipeline.rs"]
mod __MTL4ComputePipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:68`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4Counters")]
#[path = "MTL4Counters.rs"]
mod __MTL4Counters;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:71`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4FunctionDescriptor")]
#[path = "MTL4FunctionDescriptor.rs"]
mod __MTL4FunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:74`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4LibraryDescriptor")]
#[path = "MTL4LibraryDescriptor.rs"]
mod __MTL4LibraryDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:77`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4LibraryFunctionDescriptor")]
#[path = "MTL4LibraryFunctionDescriptor.rs"]
mod __MTL4LibraryFunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:80`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4LinkingDescriptor")]
#[path = "MTL4LinkingDescriptor.rs"]
mod __MTL4LinkingDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:86`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4MachineLearningPipeline")]
#[path = "MTL4MachineLearningPipeline.rs"]
mod __MTL4MachineLearningPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:89`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4MeshRenderPipeline")]
#[path = "MTL4MeshRenderPipeline.rs"]
mod __MTL4MeshRenderPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:92`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4PipelineDataSetSerializer")]
#[path = "MTL4PipelineDataSetSerializer.rs"]
mod __MTL4PipelineDataSetSerializer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:95`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4PipelineState")]
#[path = "MTL4PipelineState.rs"]
mod __MTL4PipelineState;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:101`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4RenderPass")]
#[path = "MTL4RenderPass.rs"]
mod __MTL4RenderPass;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:104`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4RenderPipeline")]
#[path = "MTL4RenderPipeline.rs"]
mod __MTL4RenderPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:107`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4SpecializedFunctionDescriptor")]
#[path = "MTL4SpecializedFunctionDescriptor.rs"]
mod __MTL4SpecializedFunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:110`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4StitchedFunctionDescriptor")]
#[path = "MTL4StitchedFunctionDescriptor.rs"]
mod __MTL4StitchedFunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:113`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTL4TileRenderPipeline")]
#[path = "MTL4TileRenderPipeline.rs"]
mod __MTL4TileRenderPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:116`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "MTLAccelerationStructure")]
#[path = "MTLAccelerationStructure.rs"]
mod __MTLAccelerationStructure;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:119`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLAccelerationStructureCommandEncoder")]
#[path = "MTLAccelerationStructureCommandEncoder.rs"]
mod __MTLAccelerationStructureCommandEncoder;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:128`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLArgument")]
#[path = "MTLArgument.rs"]
mod __MTLArgument;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:134`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLBinaryArchive")]
#[path = "MTLBinaryArchive.rs"]
mod __MTLBinaryArchive;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:140`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLBlitPass")]
#[path = "MTLBlitPass.rs"]
mod __MTLBlitPass;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:146`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLCaptureManager")]
#[path = "MTLCaptureManager.rs"]
mod __MTLCaptureManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:164`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLComputePass")]
#[path = "MTLComputePass.rs"]
mod __MTLComputePass;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:167`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLComputePipeline")]
#[path = "MTLComputePipeline.rs"]
mod __MTLComputePipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:170`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLCounters")]
#[path = "MTLCounters.rs"]
mod __MTLCounters;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:179`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLDepthStencil")]
#[path = "MTLDepthStencil.rs"]
mod __MTLDepthStencil;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:194`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLEvent")]
#[path = "MTLEvent.rs"]
mod __MTLEvent;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:200`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLFunctionConstantValues")]
#[path = "MTLFunctionConstantValues.rs"]
mod __MTLFunctionConstantValues;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:203`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLFunctionDescriptor")]
#[path = "MTLFunctionDescriptor.rs"]
mod __MTLFunctionDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:212`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLFunctionStitching")]
#[path = "MTLFunctionStitching.rs"]
mod __MTLFunctionStitching;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:218`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "MTLHeap")]
#[path = "MTLHeap.rs"]
mod __MTLHeap;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:224`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLIOCommandQueue")]
#[path = "MTLIOCommandQueue.rs"]
mod __MTLIOCommandQueue;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:230`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323, API-0658.

```rust
#[cfg(feature = "MTLIndirectCommandBuffer")]
#[path = "MTLIndirectCommandBuffer.rs"]
mod __MTLIndirectCommandBuffer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:236`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLIntersectionFunctionTable")]
#[path = "MTLIntersectionFunctionTable.rs"]
mod __MTLIntersectionFunctionTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:239`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323, API-0450.

```rust
#[cfg(feature = "MTLLibrary")]
#[path = "MTLLibrary.rs"]
mod __MTLLibrary;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:242`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLLinkedFunctions")]
#[path = "MTLLinkedFunctions.rs"]
mod __MTLLinkedFunctions;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:245`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLLogState")]
#[path = "MTLLogState.rs"]
mod __MTLLogState;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:251`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLPipeline")]
#[path = "MTLPipeline.rs"]
mod __MTLPipeline;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:257`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLRasterizationRate")]
#[path = "MTLRasterizationRate.rs"]
mod __MTLRasterizationRate;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:269`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLResidencySet")]
#[path = "MTLResidencySet.rs"]
mod __MTLResidencySet;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:278`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLResourceStatePass")]
#[path = "MTLResourceStatePass.rs"]
mod __MTLResourceStatePass;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:281`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLResourceViewPool")]
#[path = "MTLResourceViewPool.rs"]
mod __MTLResourceViewPool;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:284`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLSampler")]
#[path = "MTLSampler.rs"]
mod __MTLSampler;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:287`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLStageInputOutputDescriptor")]
#[path = "MTLStageInputOutputDescriptor.rs"]
mod __MTLStageInputOutputDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:290`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLTensor")]
#[path = "MTLTensor.rs"]
mod __MTLTensor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:302`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLVertexDescriptor")]
#[path = "MTLVertexDescriptor.rs"]
mod __MTLVertexDescriptor;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:305`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "MTLVisibleFunctionTable")]
#[path = "MTLVisibleFunctionTable.rs"]
mod __MTLVisibleFunctionTable;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:23`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323, API-0471, API-0472, API-0474.

```rust
#[cfg(feature = "CAAnimation")]
#[path = "CAAnimation.rs"]
mod __CAAnimation;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:29`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAConstraintLayoutManager")]
#[path = "CAConstraintLayoutManager.rs"]
mod __CAConstraintLayoutManager;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:32`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CADisplayLink")]
#[path = "CADisplayLink.rs"]
mod __CADisplayLink;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:38`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAEDRMetadata")]
#[path = "CAEDRMetadata.rs"]
mod __CAEDRMetadata;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:41`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323, API-0568.

```rust
#[cfg(feature = "CAEmitterCell")]
#[path = "CAEmitterCell.rs"]
mod __CAEmitterCell;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:44`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAEmitterLayer")]
#[path = "CAEmitterLayer.rs"]
mod __CAEmitterLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:50`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAGradientLayer")]
#[path = "CAGradientLayer.rs"]
mod __CAGradientLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:59`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAMediaTimingFunction")]
#[path = "CAMediaTimingFunction.rs"]
mod __CAMediaTimingFunction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:62`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAMetalDisplayLink")]
#[path = "CAMetalDisplayLink.rs"]
mod __CAMetalDisplayLink;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:65`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323, API-0409, API-0454, API-0480, API-0563, API-0566, API-0578, API-0611, API-0689.

```rust
#[cfg(feature = "CAMetalLayer")]
#[path = "CAMetalLayer.rs"]
mod __CAMetalLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:68`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAOpenGLLayer")]
#[path = "CAOpenGLLayer.rs"]
mod __CAOpenGLLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:71`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CARemoteLayerClient")]
#[path = "CARemoteLayerClient.rs"]
mod __CARemoteLayerClient;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:74`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CARemoteLayerServer")]
#[path = "CARemoteLayerServer.rs"]
mod __CARemoteLayerServer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:80`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAReplicatorLayer")]
#[path = "CAReplicatorLayer.rs"]
mod __CAReplicatorLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:83`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAScrollLayer")]
#[path = "CAScrollLayer.rs"]
mod __CAScrollLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:86`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323, API-0471, API-0472, API-0474.

```rust
#[cfg(feature = "CAShapeLayer")]
#[path = "CAShapeLayer.rs"]
mod __CAShapeLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:89`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323, API-0571.

```rust
#[cfg(feature = "CATextLayer")]
#[path = "CATextLayer.rs"]
mod __CATextLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:92`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CATiledLayer")]
#[path = "CATiledLayer.rs"]
mod __CATiledLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:95`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CATransaction")]
#[path = "CATransaction.rs"]
mod __CATransaction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:101`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CATransformLayer")]
#[path = "CATransformLayer.rs"]
mod __CATransformLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-quartz-core-0.3.2/src/generated/mod.rs:104`

File SHA256 `329804e12d78a3f1702da953266c9fe7710a1c7d037d9836ba75f1e03c044e0c`. Records: API-0319, API-0323.

```rust
#[cfg(feature = "CAValueFunction")]
#[path = "CAValueFunction.rs"]
mod __CAValueFunction;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-0.6.5/src/runtime/mod.rs:37`

File SHA256 `accb4533fec9dbfc9ca51090663def2308aac72c66ec9cbf85649c4d2729c4c6`. Records: API-0356, API-0357, API-0511.

```rust
mod nsobject;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-image-io-0.3.2/src/generated/mod.rs:32`

File SHA256 `fb60da415c1d94888060e0395aa0b1ef77d05f6f00b382be3efa0e6fc41e037f`. Records: API-0379, API-0380, API-0381, API-0382, API-0383, API-0384, CONST-0096, CONST-0097, CONST-0098, CONST-0099, CONST-0100, CONST-0101.

```rust
#[cfg(feature = "CGImageProperties")]
#[path = "CGImageProperties.rs"]
mod __CGImageProperties;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:68`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0391, API-0399, API-0400, CONST-0108, CONST-0115, CONST-0116.

```rust
#[cfg(feature = "CTStringAttributes")]
#[path = "CTStringAttributes.rs"]
mod __CTStringAttributes;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:41`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0394, API-0396, API-0398, CONST-0112, CONST-0114.

```rust
#[cfg(feature = "CTFontTraits")]
#[path = "CTFontTraits.rs"]
mod __CTFontTraits;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-foundation-0.3.2/src/lib.rs:135`

File SHA256 `960d1558eebcfbd7531a4db9c556a1bfea093373369031ce7497797762e3f97f`. Records: API-0447.

```rust
#[cfg(feature = "NSObject")]
mod copying;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-foundation-0.3.2/src/generated/mod.rs:137`

File SHA256 `af508283110a0fe8c87986bb89e8e54dae8e3d732a5aa95e0cba7c737718219d`. Records: API-0471, API-0472, API-0474.

```rust
#[cfg(feature = "CFURL")]
#[path = "CFURL.rs"]
mod __CFURL;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-text-0.3.2/src/generated/mod.rs:44`

File SHA256 `1e92ec4e4e4388e97c6bfb5e79bd8ab0343336c8a65508218d01d59ac9d25d9a`. Records: API-0471, API-0472, API-0474.

```rust
#[cfg(feature = "CTFrame")]
#[path = "CTFrame.rs"]
mod __CTFrame;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:779`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: API-0571.

```rust
#[cfg(feature = "NSTextStorageScripting")]
#[path = "NSTextStorageScripting.rs"]
mod __NSTextStorageScripting;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:104`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0658.

```rust
#[cfg(feature = "CGLayer")]
#[path = "CGLayer.rs"]
mod __CGLayer;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-core-graphics-0.3.2/src/generated/mod.rs:152`

File SHA256 `770140af1dddbb63fbba79532d32c51660b74a3158bcb5d2a6c3222d10ddd091`. Records: API-0658.

```rust
#[cfg(feature = "CGRenderingBufferProvider")]
#[path = "CGRenderingBufferProvider.rs"]
mod __CGRenderingBufferProvider;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:254`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: CONST-0003, CONST-0004.

```rust
#[cfg(feature = "MTLPixelFormat")]
#[path = "MTLPixelFormat.rs"]
mod __MTLPixelFormat;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-metal-0.3.2/src/generated/mod.rs:272`

File SHA256 `d6ce6925431c4b29df7339c4c2e326932cb89e15eb7021fdab6832985c70a743`. Records: CONST-0006.

```rust
#[cfg(feature = "MTLResource")]
#[path = "MTLResource.rs"]
mod __MTLResource;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:305`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: CONST-0033, CONST-0074, CONST-0075.

```rust
#[cfg(feature = "NSGraphics")]
#[path = "NSGraphics.rs"]
mod __NSGraphics;
```

## `.cache/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/objc2-app-kit-0.3.2/src/generated/mod.rs:854`

File SHA256 `ed2ee03a54db629e9c6226f64103ed24d80c54404c621f2fe9d2ebf81847637e`. Records: CONST-0069, CONST-0070.

```rust
#[cfg(feature = "NSUserInterfaceLayout")]
#[path = "NSUserInterfaceLayout.rs"]
mod __NSUserInterfaceLayout;
```

