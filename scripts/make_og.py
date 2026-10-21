#!/usr/bin/env python3
"""Render website/assets/og.png (1200x630, <=300 KB). Needs Pillow; Geist from website/fonts."""
import os, sys
from PIL import Image, ImageDraw, ImageFont
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FONTS = os.path.join(ROOT, "website", "fonts")
TMP = "/private/tmp/vexel-fonts"

def static_font(name, wght, size):
    """PIL cannot read woff2 variable fonts: instantiate a static TTF into TMP."""
    os.makedirs(TMP, exist_ok=True)
    out = os.path.join(TMP, f"{name}-{wght}.ttf")
    if not os.path.exists(out):
        f = TTFont(os.path.join(FONTS, f"{name}-var-latin.woff2"))
        f.flavor = None
        instantiateVariableFont(f, {"wght": wght}, inplace=True)
        f.save(out)
    return ImageFont.truetype(out, size)

W, H = 1200, 630
img = Image.new("RGB", (W, H), "#0d0a1f")
# soft brand glow, deterministic
glow = Image.new("RGB", (W, H), "#0d0a1f")
g = ImageDraw.Draw(glow)
g.ellipse((700, -250, 1400, 450), fill="#2a1260")
g.ellipse((850, 250, 1450, 800), fill="#4a1448")
from PIL import ImageFilter
glow = glow.filter(ImageFilter.GaussianBlur(140))
img = Image.blend(img, glow, 0.8)

icon = Image.open(os.path.join(ROOT, "resources", "AppIcon-source.png")).convert("RGBA").resize((200, 200), Image.LANCZOS)
img.paste(icon, (80, 90), icon)
d = ImageDraw.Draw(img)
d.text((310, 100), "Vexel", font=static_font("geist", 700, 150), fill="#faf7f2")
tag = static_font("geist", 500, 54)
d.text((80, 340), "Vector animation for macOS", font=tag, fill="#faf7f2")
d.text((80, 410), "your agent can edit live.", font=tag, fill="#faf7f2")
d.text((80, 540), "Alpha · macOS 13+ · MPL-2.0", font=static_font("geist-mono", 500, 30), fill="#f4c46a")
# accent bar
for x in range(W):
    t = x / (W - 1)
    c = (0x5b, 0x2b, 0xd9); m = (0xd6, 0x38, 0x9f); e = (0xff, 0x7a, 0x1a)
    a, b, u = (c, m, t * 2) if t < .5 else (m, e, (t - .5) * 2)
    d.line([(x, 618), (x, 630)], fill=tuple(round(a[i] + (b[i] - a[i]) * u) for i in range(3)))
out = os.path.join(ROOT, "website", "assets", "og.png")
img.quantize(colors=256, method=Image.MEDIANCUT, dither=Image.NONE).save(out, optimize=True)
size = os.path.getsize(out)
print(f"og.png {size} bytes")
sys.exit(0 if size <= 300 * 1024 else 1)
