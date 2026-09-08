# Architecture

## Ownership

| Layer | Application authors | Framework or generated output |
| --- | --- | --- |
| Database | Reversible migrations and constraints | SQLite schema and enforcement |
| Rust Model | Nothing by hand | Complete SeaORM entity directory |
| GraphQL server | A Module's registrations, through module-host only | Seaography CRUD, filters, ordering, pagination, inputs, outputs, and the composition registry |
| Beyond CRUD | Which primitives the Module declares, and what its resolvers and rules compute | The primitives themselves: custom operations, write hooks, hooked writes, write selection, transactions, computed fields, subscriptions |
| Contract | Review and commit | Exported `schema.graphql` |
| Transport | Nothing by hand | One Transport per Target: TauRPC for Desktop, HTTP and graphql-ws for Web |
| Client | Caller-specific operations and cache convergence | Typed documents, variables, and results |
| UI | Product behavior and presentation | Apollo hook inference |

## Two Targets

One App Schema and one UI are deployed in exactly two forms. Both execute the
same Generated Contract; each has its own Transport and its own Store, and the
two Stores hold unrelated data.

| | Desktop Target | Web Target |
| --- | --- | --- |
| Process | the Tauri application | the Server Process, `crates/web-server` |
| Transport | GraphQL over TauRPC IPC | `POST /graphql` and `WS /graphql/ws` |
| Store | SQLite under the platform app-data directory | SQLite at the Server Process's `--store` path |
| Network | none required | one origin, the one that served the page |

The Transport is chosen by the bundler from the `APP_TARGET` flag, so each
bundle contains only its own Transport rather than detecting its host at
runtime. `scripts/check-target-bundles.sh` builds both and fails if either
carries the other's Transport.

## Desktop Target runtime

The Tauri setup callback creates the application-data directory, opens one
SQLite database, applies every migration, builds the Seaography schema, and
installs it into the transport. Calls made before installation receive a
structured `service_unavailable` GraphQL error rather than a panic.

Queries and mutations use `graphql_execute`. Subscriptions use
`graphql_subscribe`, one unique subscription ID, and one Tauri IPC channel.
Unsubscription aborts the Rust task and removes it from the registry.

The Desktop Target opens no port and needs no browser network permission. Its
bundle contains no network Transport, so it keeps working with no connection.

## Server Process runtime

The Server Process is a headless binary that links no Tauri code and needs no
webview toolchain. It opens its Store, applies every migration, builds the same
App Schema, and serves four routes on one origin:

| Route | Purpose |
| --- | --- |
| `POST /graphql` | queries and mutations |
| `WS /graphql/ws` | subscriptions, standard graphql-ws protocol |
| `GET /` | the Web Target entry document and its assets |
| any unmatched path | the entry document, so client-side routes deep-link |

Assets are read from the `--web-root` directory at runtime rather than embedded,
which keeps the Rust and bundler build graphs independent and makes a stale
embedded bundle impossible.

Because the UI and its endpoints share an origin, the client uses relative
paths. There is no API base URL to configure and no CORS configuration: no
cross-origin request is ever made, so there is no CORS surface to review.

### It is unauthenticated, and that constrains where it may run

`--bind` defaults to loopback, and a test asserts that default so a refactor
cannot quietly widen it. The default is load-bearing rather than a convenience:
the Server Process ships no authentication and no authorization, so anything
that can reach the port has complete read and write access to the Store.

Generated CRUD makes this sharper than "unaudited". Seaography filters are
unscoped by construction — a caller chooses the filter, and no ownership
predicate is applied — so a single query can read or a single mutation can
change every row of every Model. There is no per-user scoping to defeat.

Authorization is therefore not a missing feature but an extension point outside
the Stability Boundary, tracked as separate work that must supply its own
executable proof — including per-user scoping over generated CRUD — before the
Web Target may be exposed beyond loopback. See
[`docs/stability-boundary.md`](stability-boundary.md) and
[ADR-0004](adr/0004-server-process-is-unauthenticated-and-same-origin.md).

## Modules

Features live under `modules/`, one folder per Module — a reusable vertical
slice contributing to both sides of the Generated Contract (ADR-0005,
ADR-0006). `scripts/new-module.sh` stamps the folder out; workspace globs
(`modules/*/rust` in `Cargo.toml`, `modules/*/ui` in `package.json`) make it a
build member with no edits elsewhere.

```text
modules/<name>/
├── rust/                # crate <name>-module, workspace member via glob
│   ├── Cargo.toml       # this Module's own dependencies
│   └── src/
│       ├── lib.rs       # one module_def! call; names crate and module_host only
│       ├── custom.rs    # the Module's whole surface beyond generated CRUD
│       ├── migrations/  # hand-authored, reversible
│       └── entities/    # generated SeaORM entities; never hand-edited
└── ui/                  # npm workspace member via glob
    ├── package.json     # this Module's own dependencies
    ├── operations/      # Caller Operations (.graphql)
    └── src/
        ├── index.ts     # exports moduleDef { name, typePolicies, Component }
        └── generated/   # typed documents against the Module's mini-schema
```

Composition is compile-time and generated, and every step of it is reviewable:

1. Each Module's `rust/src/lib.rs` calls `module_def!` — its name, its
   migrations, its generated registration function, and optionally the one
   function where everything past generated CRUD is declared.
2. `crates/module-host` is the only registration path, and the only crate a
   Module's hand-authored code may name. It rejects any table, GraphQL type, or
   root field that does not carry the Module's name as its prefix, naming the
   offender, and it holds the closed set of primitives a Module has beyond
   generated CRUD (ADR-0007). Beneath it, `crates/seaolim` closes Seaography's
   write-path gaps and knows nothing about Modules.
3. `generate` scans the module tree and emits `crates/module-registry`, which
   lists every Module in name order and flattens their migrations into one
   deterministic sequence for both Targets' Stores.
4. `crates/app-schema` composes whatever the registry hands it into the one
   App Schema (`app_schema()`); it knows no individual feature.

The frontend composes the same way: the generated `src/generated/modules.ts`
imports each Module's `moduleDef`, `src/App.tsx` maps over that list, and
`src/graphql/client.ts` merges the Modules' type policies into the one Apollo
cache. At runtime nothing changes shape: one App Schema, one Transport per
Target, one normalized cache, one `schema.graphql`.

The seal is a check rather than a compile error, because the generated entity
directory resolves `seaography`, `sea-orm`, and `tokio` by crate name and a
manifest is crate-wide. `scripts/check-module-imports.mjs` holds each Module's
hand-authored files to naming `crate` and `module_host`;
`scripts/check-module-dependencies.mjs` holds both its manifests to an
allowlist; `scripts/check-escape-hatches.mjs` counts the declared escape
hatches and demands a written exception for each. All three run in `verify`,
and `scripts/module-seal.test.mjs` proves each one rejects.

Per-module codegen is the ownership check. Each Module's operations are
generated against a standalone mini-schema built from only its own
registrations (`export_schema --module <name>`), so an operation naming
another Module's types fails generation rather than review. Cross-module
views are explicit host-level Caller Operations generated against the
composed `schema.graphql`.

## Generated boundaries

Six committed artifact groups are generated:

1. `crates/module-registry` from the module tree: the composition registry
   linking every Module into the host;
2. `modules/*/rust/src/entities/` from a clean migration-created scratch
   database holding only that Module's tables;
3. `schema.graphql` from the real composed schema;
4. `src/generated/taurpc.ts` from the Rust transport trait;
5. `src/generated/modules.ts` from the module tree: the frontend module index;
6. `modules/*/ui/src/generated/graphql.ts` from each Module's mini-schema and
   its authored operations.

`scripts/check-generated-drift.sh` creates both its migration database and each
generated artifact under temporary directories, then byte-compares them with
the committed artifact. It does not trust Git state, leave a shared generation
database, or rewrite the working tree.

The JavaScript workspace has two independently frozen installation paths:
`bun.lock` for Bun and `package-lock.json` for npm. Direct dependencies are
exactly pinned, and CI verifies both package managers against their own lock.

## State convergence

Rust is authoritative. Apollo is disposable and is never persisted.

- Updates return the changed entity. Seaography is configured so query and
  mutation results share one GraphQL typename, allowing normalization by ID.
  When an update can change list membership or ordering, callers also update or
  refetch each affected list.
- Creates explicitly refetch or update each affected list.
- Deletes evict known IDs and run cache garbage collection, or explicitly
  refetch when the affected identities are unknown.
- Live server state uses GraphQL subscriptions: over Tauri channels on the
  Desktop Target, over graphql-ws on the Web Target. Caller Operations,
  generated documents, and these convergence rules are identical on both.

Polling and focus-refetch are defects on the Desktop Target, whose Rust process
is local. On the Web Target they are equally unnecessary, because the Server
Process is reached over one origin with a live subscription socket.
