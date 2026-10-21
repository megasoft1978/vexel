#!/bin/bash
# Real-window UI click check for the vexel app (macOS, Apple Silicon).
# Drives the app through System Events (osascript) and screencapture only.
# It TAKES OVER mouse and keyboard for about 5 minutes: do not touch the
# machine while it runs. Refuses to run unless VEXEL_E2E_OK=1.
#
# usage: VEXEL_E2E_OK=1 scripts/e2e_ui.sh [fixture.svg]
#   VEXEL_BIN        binary (default target/debug/vexel, then build/rust/... debug)
#   VEXEL_OWNER      process / window owner name (default vexel)
#   VEXEL_WAIT       seconds to wait for the window (default 30)
#   VEXEL_E2E_KEEP   1 = keep the temp dir (screenshots, exports) afterwards
# Label overrides (AX name/title/description substrings) if the UI is renamed:
#   E2E_L_INSPECTOR_FIELD (default "Opacity"), E2E_L_ROTATE (default "Rotat"),
#   E2E_L_ANIMATE (default "Animate"), E2E_L_ADD_KEY (default "Add Key"),
#   E2E_L_EASING (default "Easing"), E2E_L_EXPORT_VIDEO (default "Export Video"),
#   E2E_L_EXPORT_GIF (default "Export GIF"), E2E_L_EXPORT_MP4 (default "Export MP4")
#
# Writes only inside one mktemp -d directory. Exit 0 = every step PASS,
# 1 = at least one FAIL, 2 = refused or setup error.
set -u

if [ "${VEXEL_E2E_OK:-}" != "1" ]; then
  echo "refusing: this script takes over mouse and keyboard (~5 min)." >&2
  echo "set VEXEL_E2E_OK=1 to run it on a desktop you are not using." >&2
  exit 2
fi
if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: macOS only" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HELPER="$ROOT/scripts/e2e_ui.applescript"
OWNER="${VEXEL_OWNER:-vexel}"
WAIT="${VEXEL_WAIT:-30}"
FIXTURE_SRC="${1:-$ROOT/rust/fixtures/v1/anim/loop.svg}"
L_FIELD="${E2E_L_INSPECTOR_FIELD:-Opacity}"
L_ROTATE="${E2E_L_ROTATE:-Rotat}"
L_ANIMATE="${E2E_L_ANIMATE:-Animate}"
L_ADDKEY="${E2E_L_ADD_KEY:-Add Key}"
L_EASING="${E2E_L_EASING:-Easing}"
L_EXPVIDEO="${E2E_L_EXPORT_VIDEO:-Export Video}"
L_EXPGIF="${E2E_L_EXPORT_GIF:-Export GIF}"
L_EXPMP4="${E2E_L_EXPORT_MP4:-Export MP4}"

BIN="${VEXEL_BIN:-}"
if [ -z "$BIN" ]; then
  for candidate in \
    "$ROOT/target/debug/vexel" \
    "${CARGO_TARGET_DIR:-$ROOT/build/rust}/aarch64-apple-darwin/debug/vexel" \
    "$ROOT/build/rust/aarch64-apple-darwin/debug/vexel"; do
    if [ -x "$candidate" ]; then BIN="$candidate"; break; fi
  done
fi
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
  echo "error: vexel binary not found; build it or set VEXEL_BIN" >&2
  exit 2
fi
if [ ! -f "$FIXTURE_SRC" ] || [ ! -f "$HELPER" ]; then
  echo "error: fixture or helper missing" >&2
  exit 2
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/vexel-e2e.XXXXXX")" || exit 2
case "$TMP" in /*vexel-e2e.*) ;; *) echo "error: unexpected temp dir" >&2; exit 2 ;; esac
SHOTS="$TMP/shots"
mkdir -p "$SHOTS" || exit 2
FIXTURE="$TMP/fixture.svg"
cp "$FIXTURE_SRC" "$FIXTURE" || exit 2

APP_PID=""
cleanup() {
  if [ -n "$APP_PID" ]; then
    kill "$APP_PID" 2>/dev/null
    sleep 1
    kill -9 "$APP_PID" 2>/dev/null
  fi
  if [ "${VEXEL_E2E_KEEP:-0}" = "1" ]; then
    echo "kept: $TMP"
  elif [ -n "$TMP" ] && [ -d "$TMP" ]; then
    rm -rf "$TMP"
  fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM

ax() { osascript "$HELPER" "$OWNER" "$@" 2>&1; }

RESULTS=()
FAILS=0
record() { # status name detail
  RESULTS+=("$1  $2${3:+  ($3)}")
  [ "$1" = "FAIL" ] && FAILS=$((FAILS + 1))
}

SHOT_N=0
shot() { # label -> path on stdout
  SHOT_N=$((SHOT_N + 1))
  local f
  f="$SHOTS/$(printf '%02d' "$SHOT_N")-$1.png"
  screencapture -x "$f" 2>/dev/null
  echo "$f"
}
sum() { cksum <"$1" 2>/dev/null | cut -d' ' -f1; }

alive() { kill -0 "$APP_PID" 2>/dev/null; }

step_ok() { # name, ax result, [detail]
  case "$2" in err:*|"") record FAIL "$1" "$2" ;; *) record PASS "$1" "${3:-}" ;; esac
}

# 1. launch
"$BIN" "$FIXTURE" >"$TMP/vexel.log" 2>&1 &
APP_PID=$!
ready=0
for _ in $(seq 1 "$((WAIT * 2))"); do
  alive || break
  if [ "$(ax windows)" -ge 1 ] 2>/dev/null; then ready=1; break; fi
  sleep 0.5
done
if [ "$ready" != 1 ]; then
  record FAIL "launch: window appears" "no window in ${WAIT}s or no Accessibility permission; see docs/e2e-ui.md"
else
  record PASS "launch: window appears"
  ax focus >/dev/null
  sleep 2
  BASE="$(shot launch)"
  [ -s "$BASE" ] && record PASS "screenshot capture" || record FAIL "screenshot capture" "Screen Recording permission?"

  # 2. inspector edit: select all, set a property field, expect pixels to change
  ax key a command >/dev/null; sleep 0.5
  before="$(sum "$(shot pre-inspector)")"
  r="$(ax setfield "$L_FIELD" 0.5)"; sleep 1
  after="$(sum "$(shot inspector-edit)")"
  if case "$r" in err:*) true ;; *) false ;; esac; then
    record FAIL "inspector edit ($L_FIELD)" "$r"
  elif [ "$before" = "$after" ]; then
    record FAIL "inspector edit ($L_FIELD)" "canvas unchanged"
  else
    record PASS "inspector edit ($L_FIELD)"
  fi

  # 3. rotate via inspector rotation field, canvas must change
  before="$(sum "$(shot pre-rotate)")"
  r="$(ax setfield "$L_ROTATE" 15)"; sleep 1
  after="$(sum "$(shot rotate)")"
  if case "$r" in err:*) true ;; *) false ;; esac; then
    record FAIL "canvas rotate" "$r"
  elif [ "$before" = "$after" ]; then
    record FAIL "canvas rotate" "canvas unchanged"
  else
    record PASS "canvas rotate"
  fi

  # 4. group / ungroup shortcuts (Cmd+G, Cmd+Shift+G); app must survive both
  ax key a command >/dev/null; sleep 0.3
  ax key g command >/dev/null; sleep 0.7
  shot grouped >/dev/null
  ax key g command,shift >/dev/null; sleep 0.7
  shot ungrouped >/dev/null
  alive && record PASS "canvas shortcuts group/ungroup (app alive)" || record FAIL "canvas shortcuts group/ungroup" "app exited"

  # 5. timeline: Animate mode, move playhead, add key, easing picker
  r="$(ax click "$L_ANIMATE")"; step_ok "timeline: enter Animate mode" "$r"; sleep 1
  ax code 124 >/dev/null; ax code 124 >/dev/null; sleep 0.3   # Right x2
  before="$(sum "$(shot pre-key)")"
  r="$(ax click "$L_ADDKEY")"; sleep 1
  after="$(sum "$(shot key-added)")"
  if case "$r" in err:*) true ;; *) false ;; esac; then
    record FAIL "timeline: add key" "$r"
  elif [ "$before" = "$after" ]; then
    record FAIL "timeline: add key" "timeline unchanged"
  else
    record PASS "timeline: add key"
  fi
  r="$(ax click "$L_EASING")"; sleep 0.8
  shot easing-picker >/dev/null
  step_ok "timeline: easing picker opens" "$r"
  ax code 53 >/dev/null; sleep 0.3  # Escape closes picker

  # 6. Export Video row
  r="$(ax has "$L_EXPVIDEO")"
  [ "$r" = "yes" ] && record PASS "Export Video... row present" || record FAIL "Export Video... row present" "$r"

  # 7. save panels: click row, type full path into the name field, confirm
  export_via_panel() { # label, row text, out file, header check
    local label="$1" row="$2" out="$3" kind="$4" r i
    r="$(ax click "$row")"
    case "$r" in err:*) record FAIL "$label: open row" "$r"; return ;; esac
    sleep 1.5
    if [ "$(ax windows)" -lt 2 ] 2>/dev/null && [ "$(ax has Save)" != "yes" ]; then
      record FAIL "$label: save panel appears" "no panel"
      return
    fi
    shot "$label-panel" >/dev/null
    ax key a command >/dev/null
    ax type "$out" >/dev/null
    sleep 0.5
    ax code 36 >/dev/null   # Return = Save
    for _ in $(seq 1 60); do
      [ -s "$out" ] && break
      sleep 1
    done
    shot "$label-done" >/dev/null
    if [ ! -s "$out" ]; then
      record FAIL "$label: file written" "missing or empty: $out"
      ax code 53 >/dev/null
      return
    fi
    local magic
    case "$kind" in
      gif) magic="$(head -c 4 "$out")"; [ "$magic" = "GIF8" ] ;;
      mp4) magic="$(dd if="$out" bs=1 skip=4 count=4 2>/dev/null)"; [ "$magic" = "ftyp" ] ;;
    esac
    if [ $? -eq 0 ]; then
      record PASS "$label: file written, header ok" "$(wc -c <"$out" | tr -d ' ') bytes"
    else
      record FAIL "$label: header" "unexpected magic '$magic'"
    fi
  }
  export_via_panel "export-gif" "$L_EXPGIF" "$TMP/out.gif" gif
  export_via_panel "export-mp4" "$L_EXPMP4" "$TMP/out.mp4" mp4

  alive && record PASS "app still running at end" || record FAIL "app still running at end" "see $TMP/vexel.log"
fi

echo
echo "== vexel e2e ui summary =="
for line in "${RESULTS[@]}"; do echo "$line"; done
echo "screenshots: $SHOTS $( [ "${VEXEL_E2E_KEEP:-0}" = 1 ] || echo '(deleted on exit; set VEXEL_E2E_KEEP=1)' )"
if [ "$FAILS" -gt 0 ]; then
  echo "RESULT: FAIL ($FAILS failed)"
  exit 1
fi
echo "RESULT: PASS"
exit 0
