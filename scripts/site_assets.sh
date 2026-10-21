#!/usr/bin/env bash
# Regenerates committed website/README media. Idempotent: outputs are deterministic.
# Needs: ffmpeg, cwebp, sips, magick (optional), pyftsubset (fonttools), python3+PIL, curl, tar, npm|curl.
# Inputs live outside the repo (/private/tmp/vexel-*); missing inputs are skipped with a warning.
set -euo pipefail
cd "$(dirname "$0")/.."
SRC_ICON=resources/AppIcon-source.png
ACC=${VEXEL_ACCEPT:-/private/tmp/vexel-accept}
DEMO=${VEXEL_DEMO:-/private/tmp/vexel-demo-video}
FONTS_TMP=/private/tmp/vexel-fonts
A=website/assets; M=website/media; D=docs/media
need() { command -v "$1" >/dev/null 2>&1 || { echo "MISSING tool: $1" >&2; return 1; }; }
have_in() { [ -f "$1" ] || { echo "WARN missing input: $1 (skipped)" >&2; return 1; }; }
FF=(ffmpeg -v error -y)
OB=(-map_metadata -1 -fflags +bitexact -flags:v +bitexact -flags:a +bitexact)
mkdir -p "$A" "$M" "$D" website/fonts

# --- icons ---
resize_png() { # in out size
  if need sips 2>/dev/null; then sips -z "$3" "$3" "$1" --out "$2" >/dev/null
  else magick "$1" -resize "${3}x${3}" -strip "$2"; fi
}
if have_in "$SRC_ICON"; then
  resize_png "$SRC_ICON" "$A/favicon-32.png" 32
  resize_png "$SRC_ICON" "$A/apple-touch-icon.png" 180
  for s in 512 128; do
    resize_png "$SRC_ICON" "$FONTS_TMP/icon-$s.png" "$s" 2>/dev/null || { mkdir -p "$FONTS_TMP"; resize_png "$SRC_ICON" "$FONTS_TMP/icon-$s.png" "$s"; }
    cwebp -quiet -q 85 -alpha_q 90 -m 6 "$FONTS_TMP/icon-$s.png" -o "$A/icon-$s.webp"
  done
fi

# --- fonts (Geist + Geist Mono variable, OFL; fontsource npm tarballs, then latin subset) ---
mkdir -p "$FONTS_TMP"
UNI=U+0020-007E,U+00A0-00FF,U+2013,U+2014,U+2018-201D,U+2022,U+2026,U+2192
for p in geist geist-mono; do
  if [ ! -f "$FONTS_TMP/$p/package/files/$p-latin-wght-normal.woff2" ]; then
    mkdir -p "$FONTS_TMP/$p"
    url=$(npm view "@fontsource-variable/$p" dist.tarball)
    curl -sL -o "$FONTS_TMP/$p.tgz" "$url"; tar xzf "$FONTS_TMP/$p.tgz" -C "$FONTS_TMP/$p"
  fi
  pyftsubset "$FONTS_TMP/$p/package/files/$p-latin-wght-normal.woff2" --flavor=woff2 \
    --unicodes="$UNI" --layout-features='*' --output-file="website/fonts/$p-var-latin.woff2"
done
cp "$FONTS_TMP/geist/package/LICENSE" website/fonts/OFL.txt

# --- hero loop (walk_home.mp4 -> mp4/webm <=800KB, muted, poster) ---
if have_in "$ACC/walk_home.mp4"; then
  "${FF[@]}" -i "$ACC/walk_home.mp4" -an -vf "scale=512:512:flags=lanczos,format=yuv420p" \
    "${OB[@]}" -c:v libx264 -preset veryslow -crf 20 -movflags +faststart -threads 1 "$M/hero-loop.mp4"
  "${FF[@]}" -i "$ACC/walk_home.mp4" -an -vf "scale=512:512:flags=lanczos" \
    "${OB[@]}" -c:v libvpx-vp9 -crf 30 -b:v 0 -row-mt 0 -threads 1 -deadline good -cpu-used 1 "$M/hero-loop.webm"
  cp "$M/hero-loop.mp4" "$M/walk-home.mp4"   # gallery video (same encode, budget permitting)
fi
have_in "$ACC/walk_home_a.png" && cwebp -quiet -q 80 -m 6 "$ACC/walk_home_a.png" -o "$M/hero-poster.webp"

# --- GIFs via ffmpeg palettegen (gifsicle not required) ---
mkgif() { # input out width fps colors dither [extra ffmpeg input opts via GIF_IN]
  local in=$1 out=$2 w=$3 fps=$4 colors=${5:-128} dither=${6:-bayer:bayer_scale=4}
  local vf="fps=$fps,scale=$w:-1:flags=lanczos"
  "${FF[@]}" ${GIF_IN:-} -i "$in" -vf "$vf,split[a][b];[a]palettegen=max_colors=$colors:stats_mode=diff[p];[b][p]paletteuse=dither=$dither:diff_mode=rectangle" "${OB[@]}" -loop 0 "$out"
}
have_in "$ACC/walk_home.mp4" && mkgif "$ACC/walk_home.mp4" "$D/gallery-walk-home.gif" 320 8 32 none  # noisy frames: tight palette to stay <=400KB
have_in "$DEMO/demo.gif" && mkgif "$DEMO/demo.gif" "$D/gallery-night-boat.gif" 400 12
have_in "$ACC/show8.gif" && mkgif "$ACC/show8.gif" "$D/gallery-show.gif" 400 12
# hero.gif: PLACEHOLDER 6 s cut (t=3..9) of the Codex+MCP demo; PRIVACY: only the first 37 s are Vexel-only,
# never cut past 37 s. Replace with the final video cut when it exists.
if have_in "$DEMO/vexel_codex_mcp_demo.mp4"; then
  GIF_IN="-ss 3 -t 6" mkgif "$DEMO/vexel_codex_mcp_demo.mp4" "$D/hero.gif" 1100 20 256
fi

# --- og.png ---
python3 -I scripts/make_og.py
echo done
