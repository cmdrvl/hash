#!/usr/bin/env bash
set -euo pipefail
grep -F "brew install cmdrvl/tap/cmdrvl-hash" README.md >/dev/null
if grep -F 'version "{bare}"' .github/workflows/release.yml >/dev/null; then
  echo "release workflow should let Homebrew infer formula version" >&2
  exit 1
fi
