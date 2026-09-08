#!/usr/bin/env bash
set -euo pipefail

case "${npm_config_user_agent:-}" in
  bun/*)
    exec bun "$@"
    ;;
  npm/*)
    exec node "$@"
    ;;
esac

if command -v bun >/dev/null 2>&1; then exec bun "$@"; fi
if command -v node >/dev/null 2>&1; then exec node "$@"; fi

echo "ERROR: Bun or Node.js is required." >&2
exit 1
