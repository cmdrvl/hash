#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
set +e
output="$(printf '{not-json}\n' | "$bin" --no-witness)"
code=$?
set -e
test "$code" -eq 2
jq -e '.refusal.next_command == "vacuum <ROOT> | hashbytes"' <<<"$output" >/dev/null
