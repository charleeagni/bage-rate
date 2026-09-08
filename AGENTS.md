# Repository rules

This application is migration-first and generated-contract-first.

## Database-backed Models

Every Model belongs to a Module: one folder under `modules/<name>/` holding
the feature's Rust half and frontend half. Scaffold a new one with
`npm run new-module -- <name>`.

1. Author a reversible migration under `modules/<name>/rust/src/migrations/`.
2. Run `npm run generate`. Never hand-edit `modules/*/rust/src/entities/`,
   `schema.graphql`, `crates/module-registry/`, `src/generated/modules.ts`,
   `src/generated/taurpc.ts`, or `modules/*/ui/src/generated/`.
3. Review the complete SDL diff as public API.
4. Author caller-specific `.graphql` operations under
   `modules/<name>/ui/operations/`. Do not hand-write mirror TypeScript Model
   types or Apollo generics.
5. Declare cache convergence for every write. Updates converge through their
   returned entity only when list membership and ordering cannot change;
   otherwise update/refetch affected lists. Creates update/refetch affected
   lists, and deletes evict known identities or explicitly refetch. A custom
   operation returning its own outcome type has nothing to normalize, so it
   declares convergence explicitly.
6. Identity-scoped update/delete operations bind a non-null identity variable
   into a literal filter. Expose a generated filter variable only from an
   explicitly named bulk write; a non-null empty filter still matches all rows.
   Do not expose auto-increment primary keys through caller create/update data.
7. Run `npm run verify` before declaring the change complete.

## What a Module may say

Two rules keep Modules composable. Module code registers only through the
module-host seam — no raw Seaography builder calls in `modules/`. And a Module
never references another Module's types: its operations are generated against
its own mini-schema, so a cross-module reference fails generation. Combining
Modules is host-level code — explicit Caller Operations against the composed
`schema.graphql`.

Hand-authored files under `modules/*/rust/src/` name exactly two crate roots,
`crate` and `module_host`, and a Module's `ui/src/` sees React,
`@apollo/client`, `graphql`, and its own files. Both manifests are held to an
allowlist. `verify` checks all of this; do not work around it by widening a
manifest. A capability a Module needs — filesystem, network, a process — is
added to `module-host` as a handle on `ModuleCtx`, never imported inside the
Module.

Generated Seaography CRUD, filters, pagination, ordering, and batch mutations
stay enabled by default. Beyond them, `module_host::CustomOps` is the complete
list of what a Module can express: custom queries, mutations, and
subscriptions; write hooks; hooked generated writes; per-operation write
selection; transactions across the Module's own Models; and computed read
fields. Read `crates/module-host/src/lib.rs` before concluding something has no
home — most things that look like they need custom code are one call there, and
`docs/writing-a-module-by-example.md` shows each one against a real Module.
`docs/operations/` is the per-surface recipe: pick the surface from its table
first, then follow the page for a [query](docs/operations/query.md),
[mutation](docs/operations/mutation.md), or
[subscription](docs/operations/subscription.md) end to end.

## Written exceptions

Do not add replacement CRUD, repositories, mirrored DTOs, hand-written
resolvers where a generated mutation exists, `mutation: false`, or
generated-file patches without a written exception in `docs/exceptions/` that
identifies the missing behavior, the rejected framework facilities, the
smallest custom seam, and its drift-prevention test.

Work that fits no primitive — many reads, branching on what was read, and a
write whose shape follows from them — goes through `CustomOps::escape_hatch`
rather than through a seam you invent. It is declared, it needs the same
written exception, and `verify` fails above two per Module. A second Module
reaching for the same shape is the signal to design a primitive and delete both
exceptions.

## State

Rust owns persistent and live server state. Apollo owns a disposable cache.
Never add polling, focus-refetch, or persisted Apollo cache state. Live data
uses GraphQL subscriptions; user-action data converges through mutation
results or an explicit cache fallback.
