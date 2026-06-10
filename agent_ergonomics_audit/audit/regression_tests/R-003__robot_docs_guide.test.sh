#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
output="$("$bin" robot-docs guide)"
grep -F "hashbytes robot-docs guide" <<<"$output" >/dev/null
grep -F "hashbytes capabilities --json" <<<"$output" >/dev/null
