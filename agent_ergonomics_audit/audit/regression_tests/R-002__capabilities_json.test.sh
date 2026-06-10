#!/usr/bin/env bash
set -euo pipefail
bin="${BINARY:-target/debug/hashbytes}"
"$bin" capabilities --json | jq -e '.agent_surfaces.capabilities.argv == ["hashbytes","capabilities","--json"] and .side_effects.reads_stdin == false' >/dev/null
