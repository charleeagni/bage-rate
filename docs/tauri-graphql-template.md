# Tauri GraphQL Template

A copy-ready application with SQLite, generated SeaORM entities, Seaography
CRUD, GraphQL Code Generator, and Apollo Client, deployable in two forms from
one codebase.

## Two Targets

Every copied application ships both. They share the UI, the App Schema, and the
generated contract, and both are proven by `verify`.

| | Desktop Target | Web Target |
| --- | --- | --- |
| What it is | the Tauri 2 application | the browser application served by a headless Server Process |
| Transport | GraphQL over TauRPC IPC, subscriptions over Tauri channels | `POST /graphql` and `WS /graphql/ws`, subscriptions over graphql-ws |
| Store | SQLite in the platform app-data directory, owned by the person running it | SQLite owned by the Server Process, at its `--store` path |
| Network | none required; works fully offline | one origin, the one that served the page |
| Run it | `dev` | `dev:web` |
| Ship it | `build:app` | `build:web` |

**The two Targets hold unrelated data.** Each Store is separate, and there is no
sync, export, import, or migration path between them. That is a decision, not a
gap: a project created on the Desktop Target does not appear in the Web Target
and never will. See
[ADR-0001](adr/0001-two-targets-separate-stores.md).

**The Server Process is unauthenticated.** It ships no authentication or
authorization, and generated CRUD filters are unscoped, so anything that reaches
its port can read and write every row. It binds to loopback by default and a
test holds that default. It is unsafe to expose publicly; authorization is
tracked as separate work that must bring its own proof. See
[`stability-boundary.md`](stability-boundary.md).

The template keeps application code declarative:

```text
SeaORM migration
  -> migrated SQLite schema
  -> generated SeaORM entity
  -> Seaography-generated GraphQL CRUD and SDL
  -> authored caller operation
  -> generated TypedDocumentNode
  -> Apollo over the Transport the Target was built with
```

Features live under `modules/`, one folder per Module: a vertical slice
carrying its migrations, generated registrations, Caller Operations, generated
types, and UI. `new-module` scaffolds the folder; dropping one in (or deleting
one) and running `generate` is the whole wiring step. The included `projects`
Module is the example.

The Cargo workspace root is the repository root. Its members are named for
what they are:

- `modules/*/rust`: each Module's Rust half — its migrations and generated
  registrations — a workspace member via glob.
- `crates/module-host`: the seam, and the closed set of primitives a Module
  authors against (ADR-0007). It composes Modules, rejects any table, GraphQL
  type, or root field that does not carry the Module's name as its prefix, and
  is the only crate a Module's hand-authored code may name.
- `crates/module-host-macros`: the two attribute macros a Module uses to
  declare custom operations, so that declaring one names nothing below the
  seam.
- `crates/seaolim`: vendored from the standalone library of that name. It
  closes Seaography's write-path gaps — per-operation mutation selection,
  hooked create/update/delete that run rules the generated writes have no hook
  point for, and set-level write hooks — and knows nothing about Modules.
- `crates/module-registry`: generated from the module tree; the one link
  between the discovered Modules and the host, committed and drift-checked.
- `crates/app-schema`: SQLite access and the composed App Schema. It composes
  whatever the registry hands it, knows no individual feature, links no Tauri
  code, and builds with no desktop or webview prerequisite.
- `crates/tauri-graphql-transport`: Rust GraphQL execution and subscription
  transport over TauRPC, for the Desktop Target.
- `crates/web-server`: the Server Process for the Web Target. It links no Tauri
  code, so it builds and deploys with no desktop or webview prerequisite.
- `src-tauri`: the Tauri application — composition and its entry point, and
  nothing else.

The UI has one Transport module per Target under `src/graphql/transport/`, and
one module that selects between them at build time. Everything downstream —
caller operations, generated documents, the Apollo cache and its convergence
rules — is shared unchanged.

The reusable Desktop Transport also has a JavaScript half:

- `packages/tauri-graphql-apollo`: Apollo Link for the generated TauRPC proxy.

The rest is intentionally app-owned and lives in Modules: migrations,
generated entities and types, Caller Operations, cache policies, and UI, one
folder per feature. The host keeps only the app shell (`src/App.tsx` maps over
the generated module index) and the Apollo client that merges every Module's
cache policies.

## Prerequisites

- Rust 1.95.0 (selected by the root `rust-toolchain.toml`)
- Bun 1.3 or newer, or Node.js 24 with npm 11 or newer
- Tauri 2 platform prerequisites for your operating system
- `sea-orm-cli` exactly 2.0.1:

```sh
cargo install --locked sea-orm-cli@2.0.1
```

## Create an application

Copy this repository, remove its `.git` directory, and initialize a new Git
repository. Then configure its visible identity:

With Bun:

```sh
bun install --frozen-lockfile
bun run configure -- \
  --name my-app \
  --title "My App" \
  --identifier com.example.my-app
bun run verify
bun run dev
```

With npm:

```sh
npm ci
npm run configure -- \
  --name my-app \
  --title "My App" \
  --identifier com.example.my-app
npm run verify
npm run dev
```

`configure` validates every identity surface before staging atomic replacements
for `package.json`, `package-lock.json`, `bun.lock`, `index.html`,
`src/app-config.ts`, and `src-tauri/tauri.conf.json`. A failed preflight leaves
all files unchanged. It does not rename the reusable Rust crates, so source
imports remain stable.

## Daily commands

Every script works as either `bun run <name>` or `npm run <name>`.

| Script | Purpose |
| --- | --- |
| `dev` | Desktop Target: run Vite and the Tauri application. |
| `dev:web` | Web Target: run the bundler and the Server Process together in a browser, with hot module replacement. |
| `new-module` | Scaffold a Module folder under `modules/` with both halves compiling before any feature code. |
| `generate` | Regenerate the module registry and index, entities, SDL, TauRPC bindings, and per-module GraphQL documents. |
| `generate:check` | Rebuild generated artifacts in scratch space and fail on drift. |
| `verify` | The canonical proof of both Targets: drift, TypeScript and template-configuration tests, both bundles and their separation, formatting, Clippy, and the Rust workspace tests that drive the Server Process. |
| `build:ui-assets` | Build the UI assets one Target embeds or serves; `APP_TARGET=web` selects the Web Target. |
| `check:bundles` | Build both Targets' UI assets and fail if either carries the other's Transport. |
| `build:web` | Web Target: build its UI assets and the Server Process binary. |
| `build:app` | Desktop Target: verify and produce a Tauri bundle. |
| `build:app:check` | Build the complete Tauri application without bundling. |

`dev:web` starts the Server Process on loopback and has the bundler proxy
`/graphql` and `/graphql/ws` to it, so development uses the same relative
endpoints and the same single origin a deployment does. It opens the same Store
file the Server Process opens when run by hand, so development data survives
restarts; set `WEB_TARGET_DEV_STORE` to put that file somewhere else. Ending the
command stops the Server Process with it. `build:web` writes the
Web Target UI assets to `dist-web` and the Server Process to
`target/release/web-server`; the Rust build never reads bundler output, and
running that binary from the repository root serves `dist-web` by default. Its
flags are `--bind` (loopback by default), `--store`, and `--web-root`, so an
operator chooses where the Server Store lives — the volume they intend to back
up, for instance.

Unlike `build:app`, `build:web` does not run `verify` first. The Web Target's
build takes minutes on the release profile and is the command a deployment
pipeline calls repeatedly, so the proof is run once by the pipeline rather than
folded into every build; CI runs `build:web` in the same job that has just run
`verify`. Run `verify` yourself before shipping a Web Target build from a
developer machine.

Both `verify` and CI run every check above under both supported package
managers, so neither installation path can break silently and neither Target can
rot while nobody is looking at it. CI additionally runs each Target's ship
command — `build:app:check` and `build:web` — so the two scripts that deliver
them are executed, not merely described.

All direct dependencies are exact pins. npm installations are captured by
`package-lock.json`, Bun installations by `bun.lock`, and Rust dependencies by
`Cargo.lock` at the repository root. CI installs both JavaScript lockfiles independently and
rejects drift in either package-manager path.

## Add a Model

Read [`adding-a-model.md`](adding-a-model.md). The short version is:
inside your Module's folder (scaffold one with `new-module` if the feature is
new), write one reversible migration, regenerate everything, review the
generated contract, author only caller operations, and add an integration
test.

The included `projects` Module is a replaceable example. Its migration proves
nullability, a database uniqueness constraint, generated create/read/update/
delete, ordering, pagination, rollback, typed callers, and cache convergence.

## Add an operation

Generated CRUD ends where the table does. For everything past it, read
[`operations/`](operations/README.md): its table picks the surface,
and one page each takes a [query](operations/query.md),
[mutation](operations/mutation.md), and
[subscription](operations/subscription.md) from the Rust file through the
schema, the caller operation, the cache, and the test. The `documents` Module
is the worked example behind all three.

## Stability boundary

The template includes only mechanisms backed by executable checks:

- migration-first generated Models;
- compile-time Module composition: prefix-checked registration through the
  module-host seam, a generated and drift-checked module registry and index, and
  per-module document generation against each Module's own mini-schema;
- a closed set of authoring primitives beyond generated CRUD — custom
  operations, write hooks, hooked and per-operation write selection,
  transactions, computed fields, subscriptions — proven against a real Module,
  and sealed by an import scan and a dependency allowlist over every Module;
- generated CRUD with unified query/mutation GraphQL entity names;
- valid SDL with or without an application subscription root, the root
  declared only when a Module registers a field on it;
- unary GraphQL operations and subscription streams over TauRPC;
- the Server Process over real sockets: queries and mutations reaching its
  Store, root assets, the deep-link fallback, the full graphql-ws lifecycle
  including cleanup, and the loopback-only default;
- build-time Transport selection, checked on both built bundles;
- operation-only TypeScript generation and Apollo type inference;
- persistent SQLite under the Tauri app-data directory;
- reproducible generated artifacts.

Authorization is not declared stable, and hosting the Web Target beyond loopback
is blocked on it. Field hiding, relations, lifecycle hooks, custom error
mapping, an application subscription field, sharing data between the two Stores,
distributing a Module outside this repository, and Model Deviations are not
declared stable either. Prove each pattern in an
isolated test before adding it to this template. See
[`stability-boundary.md`](stability-boundary.md).
