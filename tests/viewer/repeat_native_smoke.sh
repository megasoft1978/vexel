#!/bin/sh
set -u

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
runs=${1:-20}
if [ "$#" -gt 0 ]; then shift; fi
case "$runs" in
    ''|*[!0-9]*)
        printf '%s\n' 'usage: repeat_native_smoke.sh [runs [<binary> [<first.svg> [<second.svg>]]]]' >&2
        exit 2
        ;;
esac
if [ "$runs" -lt 1 ] || [ "$runs" -gt 100 ]; then
    printf '%s\n' 'runs must be between 1 and 100' >&2
    exit 2
fi

report=${VEXEL_VIEWER_SMOKE_REPEAT_LOG:-"${TMPDIR:-/tmp}/vexel-viewer-smoke-repeat-$$.log"}
: > "$report" || exit 2
passed=0
failed=0
iteration=1
while [ "$iteration" -le "$runs" ]; do
    run_log=$(mktemp "${TMPDIR:-/tmp}/vexel-viewer-smoke-run.XXXXXX") || exit 2
    if "$repo_root/dev" viewer-smoke "$@" >"$run_log" 2>&1; then
        result=PASS
        passed=$((passed + 1))
    else
        result=FAIL
        failed=$((failed + 1))
    fi
    cat "$run_log"
    cat "$run_log" >> "$report"
    rm -f "$run_log"
    line=$(printf 'iteration %02d: %s' "$iteration" "$result")
    printf '%s\n' "$line"
    printf '%s\n' "$line" >> "$report"
    iteration=$((iteration + 1))
done
summary=$(printf 'native viewer smoke repeat: %d passed, %d failed, report %s' \
    "$passed" "$failed" "$report")
printf '%s\n' "$summary"
printf '%s\n' "$summary" >> "$report"
[ "$failed" -eq 0 ]
