#!/usr/bin/env bash
set -euo pipefail

# Parses every script in this directory.
#
# Most scripts here are reached by `verify` or by CI, so a syntax error in them
# fails something. `dev-web.sh` is reached by neither — it starts a development
# session and waits, which no automated check can run to completion — so without
# this sweep a broken edit to it would ship green and first surface on a
# developer's machine. The same holds for any script added later that only a
# human ever invokes.
#
# This proves the scripts parse, not that they behave. Behaviour is proven by
# the checks `verify` runs and by the tests that read these scripts' text.

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

shopt -s nullglob
scripts=(scripts/*.sh)

# A glob that matched nothing would make this check pass while parsing nothing.
if ((${#scripts[@]} == 0)); then
  echo "ERROR: no scripts found to parse; this check would pass vacuously." >&2
  exit 1
fi

for script in "${scripts[@]}"; do
  if ! bash -n "${script}"; then
    echo "ERROR: ${script} does not parse." >&2
    exit 1
  fi
done

echo "Parsed ${#scripts[@]} script(s) in scripts/."
