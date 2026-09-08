#!/usr/bin/env bash
set -euo pipefail

PINNED_SEA_ORM_CLI_VERSION="2.0.1"
MINIMUM_NODE_MAJOR="24"
MINIMUM_BUN_MINOR="3"

for command_name in cargo sea-orm-cli; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "ERROR: ${command_name} is required." >&2
    exit 1
  fi
done

javascript_runtime=""
case "${npm_config_user_agent:-}" in
  bun/*) javascript_runtime="bun" ;;
  npm/*) javascript_runtime="node" ;;
esac
if [ -z "${javascript_runtime}" ] && command -v bun >/dev/null 2>&1; then
  javascript_runtime="bun"
elif [ -z "${javascript_runtime}" ] && command -v node >/dev/null 2>&1; then
  javascript_runtime="node"
fi

if [ "${javascript_runtime}" = "bun" ]; then
  bun_version="$(bun --version)"
  bun_major="$(printf '%s' "${bun_version}" | cut -d. -f1)"
  bun_minor="$(printf '%s' "${bun_version}" | cut -d. -f2)"
  if [ "${bun_major}" -lt 1 ] || { [ "${bun_major}" -eq 1 ] && [ "${bun_minor}" -lt "${MINIMUM_BUN_MINOR}" ]; }; then
    echo "ERROR: Bun 1.${MINIMUM_BUN_MINOR} or newer is required; found ${bun_version}." >&2
    exit 1
  fi
elif [ "${javascript_runtime}" = "node" ]; then
  node_major="$(node --version | sed -E 's/^v([0-9]+).*/\1/')"
  if [ "${node_major}" -lt "${MINIMUM_NODE_MAJOR}" ]; then
    echo "ERROR: Node.js ${MINIMUM_NODE_MAJOR} or newer is required; found $(node --version)." >&2
    exit 1
  fi
else
  echo "ERROR: Bun 1.${MINIMUM_BUN_MINOR}+ or Node.js ${MINIMUM_NODE_MAJOR}+ is required." >&2
  exit 1
fi

installed_cli_version="$(sea-orm-cli --version | awk '{print $NF}')"
if [ "${installed_cli_version}" != "${PINNED_SEA_ORM_CLI_VERSION}" ]; then
  echo "ERROR: sea-orm-cli ${installed_cli_version} is installed; ${PINNED_SEA_ORM_CLI_VERSION} is required." >&2
  echo "Install it with: cargo install --locked --force sea-orm-cli@${PINNED_SEA_ORM_CLI_VERSION}" >&2
  exit 1
fi

if [ ! -f node_modules/@graphql-codegen/cli/esm/bin.js ]; then
  echo "ERROR: JavaScript dependencies are missing." >&2
  echo "Run 'bun install --frozen-lockfile' or 'npm ci'." >&2
  exit 1
fi
