#!/usr/bin/env bash
set -euo pipefail

# Builds both Targets' UI assets and proves the two bundles are separate.
#
# Build-time Transport selection is only a guarantee if something checks the
# artifacts. This is a property of a build output rather than of a running
# process, so it cannot be observed from a test binary and belongs here.
#
# Each marker is asserted twice: absent from the bundle that must not carry it,
# and present in the bundle that must. Without the positive half, renaming or
# minifying a marker away would turn this check into one that always passes.

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

desktop_bundle="dist"
web_bundle="dist-web"

# Desktop IPC, the TauRPC Transport, and the generated TauRPC bindings. The
# command names come from the generated bindings' argument map, which survives
# minification because it is string data.
desktop_only_markers=(
  __TAURI_INTERNALS__
  TauRPC
  graphql_execute
  graphql_subscribe
  graphql_unsubscribe
)

# The network Transport: the standard graphql-ws protocol name, its first
# client message, and the Server Process endpoint the Desktop Target never
# reaches for.
web_only_markers=(
  graphql-transport-ws
  connection_init
  /graphql/ws
)

./scripts/run-package-script.sh build:ui-assets
APP_TARGET=web ./scripts/run-package-script.sh build:ui-assets

for bundle in "${desktop_bundle}" "${web_bundle}"; do
  if [[ ! -f "${bundle}/index.html" ]]; then
    echo "ERROR: ${bundle} holds no entry document; the build did not run." >&2
    exit 1
  fi
done

failures=0

report() {
  echo "ERROR: $1" >&2
  failures=$((failures + 1))
}

contains() {
  grep -rqF -- "$2" "$1"
}

for marker in "${desktop_only_markers[@]}"; do
  if contains "${web_bundle}" "${marker}"; then
    report "the Web Target bundle contains '${marker}'; a browser must receive no desktop IPC, TauRPC Transport, or generated TauRPC binding code."
  fi
  if ! contains "${desktop_bundle}" "${marker}"; then
    report "the Desktop Target bundle no longer contains '${marker}', so checking the Web Target bundle for it proves nothing. Update the marker list to name what the Desktop Transport now emits."
  fi
done

for marker in "${web_only_markers[@]}"; do
  if contains "${desktop_bundle}" "${marker}"; then
    report "the Desktop Target bundle contains '${marker}'; the offline Target must carry no network Transport."
  fi
  if ! contains "${web_bundle}" "${marker}"; then
    report "the Web Target bundle no longer contains '${marker}', so checking the Desktop Target bundle for it proves nothing. Update the marker list to name what the Web Transport now emits."
  fi
done

if ((failures > 0)); then
  echo "ERROR: ${failures} Target bundle separation check(s) failed." >&2
  exit 1
fi

echo "Target bundles are separate: ${desktop_bundle} (Desktop), ${web_bundle} (Web)."
