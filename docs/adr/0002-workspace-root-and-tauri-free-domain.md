---
status: accepted
---

# The Cargo workspace root moves to the repository root

Migrations, generated entities, the App Schema, and Store access move out of
`src-tauri` into a crate that depends on no Tauri code, so that both the Tauri
application and the headless Server Process can depend on it. The Cargo
workspace root moves from `src-tauri` to the repository root, and `src-tauri`
is reduced to composition and the binary entry point.

## Considered Options

Keeping the workspace nested under `src-tauri` was cheaper — no script or CI
path would have changed — but it would have placed a crate that links no Tauri
code inside a directory named for Tauri, so the file tree would no longer
describe the project. Having the Server Process depend on the existing
application crate was cheaper still, but it would have made a headless binary
pull in Tauri and its platform build prerequisites.

## Consequences

Every `--manifest-path` flag, the generation and drift scripts, and the CI
dependency-audit working directory change together. There is no partial
migration: the move lands in one change or not at all.
