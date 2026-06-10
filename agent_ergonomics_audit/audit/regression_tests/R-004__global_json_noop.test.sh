#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
output="$("$bin" --json --no-witness </dev/null)"
test -z "$output"
