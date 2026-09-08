# Writing a Module, by example

This document is the authoring experience end to end: every file a coding
agent writes, in order, to add one full-CRUD Model and its custom operations
as a Module. The example is real and it is committed — `modules/documents/`
translates ticketry-rust's design-documents feature (its `design_documents`
table and `save_design_document` mutation) into a Module — so every excerpt
below is a file you can open, and every claim about what fails is a check you
can run.

Read it once for the shape of the whole thing. When you are adding one
operation to a Module that already exists, [`operations/`](operations/README.md)
is the recipe: a table that picks the surface, then one end-to-end page each
for a [query](operations/query.md), [mutation](operations/mutation.md), and
[subscription](operations/subscription.md).

## 1. What the agent is given

| Surface | Command or path |
| --- | --- |
| Scaffold | `npm run new-module -- documents` (or `bun run new-module documents`) |
| Generate | `npm run generate` — regenerates registry, module index, entities, `schema.graphql`, TauRPC bindings, per-module documents |
| Prove | `npm run verify` — seal, drift, valve guard, typecheck, TS tests, bundle check, fmt, clippy, workspace tests, Server Process build and dependency check |
| May author | `modules/documents/rust/src/` except `entities/`, `rust/Cargo.toml`, `ui/operations/*.graphql`, `ui/src/` except `generated/`, `ui/package.json` |
| Never touches | `modules/*/rust/src/entities/`, `modules/*/ui/src/generated/`, `crates/module-registry/`, `schema.graphql`, `src/generated/modules.ts`, `src/generated/taurpc.ts` |
| Rust it may name | `crate` and `module_host`, and nothing else |
| UI it may name | React, `@apollo/client`, `graphql`, and the Module's own files |

Two rules shape everything below.

The Module's name is its namespace prefix. Every table, GraphQL type, and root
field it registers must carry it, compared case-insensitively with underscores
removed (`crates/module-host/src/prefix_check.rs`), and violations fail
`generate`, not review.

`module_host::CustomOps` is the complete list of what the Module can say
beyond generated CRUD. It is closed by decision: something outside it is a
written exception, and recurring exceptions are the signal to grow the
library.

## 2. Part one — a CRUD Model

Ticketry's registry of design documents: rows keyed by a caller-supplied id,
locating one document per `(root_dir, rel_path)`, scoped to a work item, with
a content digest recorded after each save.

### Scaffold

```console
$ npm run new-module -- documents
created modules/documents/
```

The scaffold stamps out `rust/` (crate `documents-module`) and `ui/` (npm
package `@tauri-graphql-template/documents-module`). Workspace globs make both
build members; nothing else is hand-wired. The Rust half is one macro call:

```rust
// modules/documents/rust/src/lib.rs — as scaffolded
module_host::module_def! {
    name: "documents",
    migrations: migrations::Migrator,
}
```

### The migration

Here the prefix rule first matters: ticketry's table is `design_documents`,
which normalises to `designdocuments` and does not start with `documents`, so
the Module cannot keep that name. The table is named `documents`, equal to the
Module name, which the prefix check accepts. One adaptation besides the
rename: `discovered_by_run_id` is dropped because it couples to ticketry's run
service.

```rust
// modules/documents/rust/src/migrations/m20260824_000001_create_documents.rs
use module_host::migration::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create().table(Documents::Table)
                .col(ColumnDef::new(Documents::Id).string().not_null().primary_key())
                // ... one column per field, content_digest nullable
                .to_owned(),
        ).await?;
        manager.create_index(
            Index::create().name("uq_documents_path").table(Documents::Table)
                .col(Documents::RootDir).col(Documents::RelPath).unique().to_owned(),
        ).await
    }
    // down() drops the table
}
```

Note `use module_host::migration::*;` rather than `use sea_orm_migration::…`.
The seam re-exports the migration prelude for the same reason it re-exports
everything else: a Module names one crate below its own, and
`scripts/check-module-imports.mjs` fails when it names another.

The agent registers the migration in `migrations/mod.rs` and adds
`pub mod entities;` plus `entities: entities::register_entity_modules,` to
`lib.rs`, as that file's comment describes.

### Run generate

`npm run generate` regenerates the registry, migrates a clean scratch Store
holding only this Module's tables, replaces the entity directory, exports the
composed SDL, and runs codegen per Module. Everything it writes is labelled
**generated, never edited**:

```rust
// modules/documents/rust/src/entities/documents.rs — generated
#[sea_orm(table_name = "documents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    // ... every column
    pub content_digest: Option<String>,
}
```

```rust
// crates/module-registry/src/lib.rs — generated (the diff)
-    vec![projects_module::module_def()]
+    vec![documents_module::module_def(), projects_module::module_def()]
```

The Module's mini-schema now carries the full generated bundle:
`documents`, `documentsCreateOne`, `documentsCreateBatch`, `documentsUpdate`,
`documentsDelete`. Section 3 removes two of those on purpose.

### Caller Operations and cache convergence

Operations go under `ui/operations/`, and the two identity-scoped writes bind
a non-null `$id` into a literal filter (AGENTS.md rule 6), so a caller cannot
pass the `{}` that matches all rows. `ui/src/cache.ts` declares the
convergence: the list is filtered by `taskId` and ordered by `relPath`, so
registering a document can change membership and refetches, while a delete
evicts the known identity.

The agent then runs `npm run verify`.

## 3. Part two — everything past CRUD

Ticketry's `save_design_document` is a write generated CRUD cannot express: a
compare-and-swap keyed on `expected_digest` whose conflict is data (`stale:
true` with the digest actually held), not an error.

Every decision about it lives in one file, which is the point — a reviewer
reads `custom.rs` and knows the Module's whole surface:

```rust
// modules/documents/rust/src/custom.rs
pub fn register(ops: &mut CustomOps) {
    ops.writes::<documents::Entity, documents::ActiveModel>(
        Writes::HOOKED.without(Write::Update),
    );
    ops.write_hook::<documents::Entity, _>(RelativePathRule);

    ops.output::<DocumentsSaveOutcome>();
    ops.mutation::<DocumentsSaveMutations>();

    ops.output::<DocumentsSaveCheck>();
    ops.query::<DocumentsSaveQueries>();

    ops.subscription::<DocumentsSavedEvent>("documentsSaved");

    ops.computed_field::<documents::Entity, bool, _>("neverSaved", |document| {
        document.content_digest.is_none()
    });
}
```

and `lib.rs` names it:

```rust
module_host::module_def! {
    name: "documents",
    migrations: migrations::Migrator,
    entities: entities::register_entity_modules,
    custom: custom::register,
}
```

### The write surface

`Writes::HOOKED.without(Write::Update)` says three things at once. Create-one
and delete run the Module's rules rather than Seaography's hook-free
generated resolvers. Create-batch is off, because there is no hooked batch
create and one write beside its siblings that quietly skips every rule is a
trap. And update is dropped outright, so `documentsSave` is the only path to a
document's digest — a caller who would rather not compare-and-swap has no
alternative to find.

The SDL is where that becomes a contract. After generate, the composed
`schema.graphql` has lost exactly `documentsUpdate`, `documentsCreateBatch`,
and the `DocumentsUpdateInput` that served the first, and kept everything the
remaining writes still need. `dropping_a_generated_mutation_removes_exactly_that_field`
asserts both halves.

Using `writes` for any Model replaces the whole Module's generated write
surface, so a Module that selects writes selects them for every Model it owns.
The SDL diff is where an omission shows up.

### The rule

```rust
// modules/documents/rust/src/rules.rs
impl WriteHook<documents::Entity> for RelativePathRule {
    fn check(&self, write: &mut WriteSet<'_, '_, documents::Entity>, _: &Txn) -> Decision {
        if write.action() == WriteAction::Delete { return Decision::Allow; }
        for index in 0..write.len() {
            // reject an absolute relPath, or one containing ".."
        }
        Decision::Allow
    }
}
```

A hook runs once per mutation over the mutation's complete write set, inside
the write's own transaction, after the rows are fetched and the proposed
values folded in and before anything is persisted. `current(index)` is the row
as the Store holds it and is absent on a create; `proposed(index)` is the row
as the mutation would leave it, and amending it there amends what is
persisted. `Decision::reject` rolls the transaction back and returns the
rule's own sentence to the caller.

That position is what makes one hook cover what Django splits between
`pre_save` signals and a serializer's `validate()`. It also means a rule
attached to a Model whose writes are all generated never fires, which is why
both calls sit together in `custom.rs`.

### The operation

```rust
// modules/documents/rust/src/save.rs
#[output]
pub struct DocumentsSaveOutcome {
    pub document_id: String,
    pub digest: String,
    pub saved: bool,
    pub stale: bool,
}

#[custom_fields]
impl DocumentsSaveMutations {
    async fn documents_save(
        ctx: &ModuleCtx<'_>,
        document_id: String,
        expected_digest: String,
        digest: String,
        saved_at: String,
    ) -> Result<DocumentsSaveOutcome> {
        let outcome = ctx.transaction(|transaction| Box::pin(async move {
            // read the row; a digest mismatch returns stale as data,
            // a match writes and returns saved
        })).await?;
        if outcome.saved { ctx.publish(DocumentsSavedEvent { .. })?; }
        Ok(outcome)
    }
}
```

Four things are worth reading closely.

`ctx` is a `ModuleCtx`, not the GraphQL context. It offers the Store, the
transaction primitive, and `publish`, and nothing else the host attached to the
schema is reachable from it. Any other effect a Module ever needs arrives the
same way, as a capability on this type.

The conflict is returned through `Ok`. An `Err` would roll the transaction
back, which is correct, but it would also throw away the digest the caller
needs in order to re-read and retry. "Unknown outcome" describes the
interaction, never the contract: `DocumentsSaveOutcome` is a concrete type
carrying the Module's prefix.

Names are camel-cased on the way into the schema, so `documents_save` becomes
`documentsSave` and `expected_digest` becomes `expectedDigest`, matching the
generated fields beside them. Ticketry's original `save_design_document` would
fail generate with the same `UnownedRootField` error as a mis-prefixed table.

The event is published after the transaction commits, so a subscriber never
sees a save that was rolled back — and a stale save publishes nothing, which
`a_conflicting_save_publishes_nothing` pins.

### The read half

`documentsSaveCheck` answers what a save *would* do, in
`modules/documents/rust/src/check.rs`. It is a custom query for the same
reason the save is a custom mutation: a caller could fetch the row and
compare digests itself, but then the rule for what counts as up to date —
an absent digest is not equal to anything, a document nobody registered is
not the same as one whose digest differs — lives in every caller instead of
beside the write it predicts. `a_custom_query_answers_from_the_store` pins
all three answers. [operations/query.md](operations/query.md) walks it
step by step.

### The subscription

`ops.subscription::<DocumentsSavedEvent>("documentsSaved")` puts one field on
the Subscription root, which the App Schema declares only because a Module
registered it. Delivery is live-only: an event raised while nobody is
subscribed is dropped, and a subscriber that falls far enough behind skips
ahead. It is a notification channel, not a second Store.

Both Transports carry it, proven end to end rather than by analogy:
`crates/tauri-graphql-transport/tests/app_schema_subscription.rs` for the
Desktop Target and `a_module_subscription_delivers_over_the_web_transport` for
the Web Target.

### Converging a result that is not an entity

`documentsSave` returns an outcome, so Apollo has nothing to normalize and the
usual "converges through the returned entity" rule cannot apply. The write
touches neither the list's filter nor its sort key, so refetching would be
waste. The declared convergence writes the two changed fields onto the cached
document by identity:

```ts
// modules/documents/ui/src/cache.ts
export function applySavedDigest(cache, documentId, digest, savedAt) {
  cache.modify({
    id: cache.identify({ __typename: "Documents", id: documentId }),
    fields: { contentDigest: () => digest, updatedAt: () => savedAt, neverSaved: () => false },
  });
}
```

## 4. When nothing fits

Some work fits no primitive: reading the Store repeatedly, branching on what
was read, and ending in a write whose shape is not known until the reads are
done. The primitives deliberately cannot express it — a hooked mutation has
one write, a custom operation has one declared output type — and pretending
otherwise would either bloat the set or push authors into inventing their own
seams.

`CustomOps::escape_hatch` admits it as a primitive, so the transaction, the
Store access, the output type, and the schema boundary stay framework-owned
and only the shape of the interaction is free. It is guarded:

- **Declared, not discovered.** A separate call from `mutation`, in the one
  file a reviewer already reads.
- **Written exception required.** `docs/exceptions/<module>--<operation>.md`,
  in the same shape AGENTS.md demands for replacement CRUD.
  `scripts/check-escape-hatches.mjs` fails `verify` without it.
- **Counted and capped.** More than two per Module fails `verify`.
- **Output still typed and prefixed.** Same prefix check as everything else.
- **Reviewed as a framework signal.** A second Module reaching for the same
  shape is the trigger to design a primitive and delete both exceptions.

No Module in this template declares one, which is the state to keep.

## 5. What the agent cannot do

Each of these is a check, not a convention.

- **Name a table outside its prefix.** A migration creating `design_documents`
  survives entity generation but fails the SDL export step of `generate`:
  `module "documents" registered type "DesignDocuments", which does not carry
  the module's name as its prefix`.
- **Reference another Module's types.** An operation in
  `modules/documents/ui/operations/` selecting `projects` fails per-module
  codegen: graphql-codegen validates against the documents mini-schema and
  exits with `Cannot query field "projects" on type "Query".`
  (`scripts/module-ownership.test.mjs` pins this behaviourally.)
- **Call the Seaography builder.** `use seaography::Builder;` — or a bare
  `seaography::Builder` with no import at all — fails
  `scripts/check-module-imports.mjs`. So does reaching
  `module_host::__private`, which exists only for the seam's own macros.
- **Add an unsanctioned dependency.** `check-module-dependencies.mjs` fails
  naming the crate or npm package. A Module's ui half declares peers and pins
  nothing, so two Modules cannot resolve two copies of React into one bundle.
- **Slip an escape hatch past review.** Declaring one without a written
  exception fails `verify`; so does an exception missing any of the
  discipline's four sections.

`scripts/module-seal.test.mjs` runs each of those checks against fixture
Modules built to fail, and against the same fixture built to pass, so a check
that quietly stopped checking is itself a failure.
