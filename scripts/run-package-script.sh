#!/usr/bin/env bash
set -euo pipefail

case "${npm_config_user_agent:-}" in
  bun/*)
    exec bun run "$@"
    ;;
  npm/*)
    exec npm run "$@"
    ;;
esac

if command -v bun >/dev/null 2>&1; then exec bun run "$@"; fi
if command -v npm >/dev/null 2>&1; then exec npm run "$@"; fi

echo "ERROR: Bun or npm is required." >&2
exit 1
