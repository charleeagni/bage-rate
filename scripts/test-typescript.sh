#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

run_with_bun() {
  exec bun test \
    packages/tauri-graphql-apollo/tests/link.test.ts \
    modules/documents/ui/src/cache.test.ts \
    modules/projects/ui/src/cache.test.ts \
    modules/workspaces/ui/src/cache.test.ts \
    src/graphql/transport/targetSelection.test.ts \
    src/lockin/babyPet.test.ts \
    src/lockin/customAnimation.test.ts \
    src/lockin/generatePrompt.test.ts \
    src/pets/psychPet.test.ts \
    packages/tauri-overlay/tests/clickable.test.ts \
    scripts/configure-template.test.mjs \
    scripts/module-ownership.test.mjs \
    scripts/module-seal.test.mjs \
    scripts/cross-language-constants.test.mjs \
    scripts/stability-boundary.test.mjs \
    scripts/web-target-commands.test.mjs
}

run_with_node() {
  exec node --test \
    packages/tauri-graphql-apollo/tests/link.test.ts \
    modules/documents/ui/src/cache.test.ts \
    modules/projects/ui/src/cache.test.ts \
    modules/workspaces/ui/src/cache.test.ts \
    src/graphql/transport/targetSelection.test.ts \
    src/lockin/babyPet.test.ts \
    src/lockin/customAnimation.test.ts \
    src/lockin/generatePrompt.test.ts \
    src/pets/psychPet.test.ts \
    packages/tauri-overlay/tests/clickable.test.ts \
    scripts/configure-template.test.mjs \
    scripts/module-ownership.test.mjs \
    scripts/module-seal.test.mjs \
    scripts/cross-language-constants.test.mjs \
    scripts/stability-boundary.test.mjs \
    scripts/web-target-commands.test.mjs
}

case "${npm_config_user_agent:-}" in
  bun/*) run_with_bun ;;
  npm/*) run_with_node ;;
esac

if command -v bun >/dev/null 2>&1; then run_with_bun; fi
if command -v node >/dev/null 2>&1; then run_with_node; fi

echo "ERROR: Bun or Node.js is required." >&2
exit 1
