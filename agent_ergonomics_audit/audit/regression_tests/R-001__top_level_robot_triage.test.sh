#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
"$bin" --robot-triage | jq -e '."$schema" == "hashbytes.doctor.triage.v1" and (.recommended_next_commands | index("hashbytes --robot-triage"))' >/dev/null
