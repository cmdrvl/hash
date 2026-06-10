#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
set +e
stderr="$("$bin" doctor --fix 2>&1 >/dev/null)"
code=$?
set -e
test "$code" -eq 2
grep -F "doctor --fix is unavailable" <<<"$stderr" >/dev/null
grep -F "hashbytes --robot-triage" <<<"$stderr" >/dev/null
