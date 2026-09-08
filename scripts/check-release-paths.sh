#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  echo "Usage: $0 <release-executable>" >&2
  exit 2
fi

artifact="$1"
if [[ ! -f "$artifact" ]]; then
  echo "ERROR: Release executable does not exist: $artifact" >&2
  exit 2
fi

# Source paths can be embedded in Rust debug metadata even in release builds.
# Do not print a matching string: it may itself be the local path this check
# is intended to prevent from being published.
if LC_ALL=C grep -a -qE '/(Users|home)/|[A-Za-z]:\\[Uu]sers\\' "$artifact"; then
  echo "ERROR: Release executable contains a local user-home path." >&2
  echo "Add or correct Rust --remap-path-prefix flags before packaging." >&2
  exit 1
fi

echo "Verified release executable has no local /Users or /home paths: $artifact"
