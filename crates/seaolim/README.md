# Seaolim

A small library on top of [Seaography](https://github.com/SeaQL/seaography) built for agents.

Most code in the projects that use it is written by coding agents, and base Seaography has traps that agents fall into without noticing. Each trap produces either a silent bug or a few hundred lines of hand-written CRUD that never needed to exist. seaolim turns those gaps into small registration calls, so the agent has less plumbing to reinvent and fewer chances to get it subtly wrong.

It depends on the stock crates.io release (`2.0.0-rc.9`, pinned), calls only public APIs, and adds no schema shapes of its own. A test asserts the generated SDL is byte-identical with and without it.

## The traps, and what changes

### 1. Composing hooks silently disables validation

Seaography's `LifecycleHooksInterface` has five methods. Its own composer, `MultiLifecycleHooks`, forwards four of them. The fifth, `before_active_model_save`, falls through to a default that allows every write untouched. So this refactor, which an agent will happily make, is a security bug:

```rust
// One hook: tenant stamping works.
context.hooks = LifecycleHooks::new(TenantStamp);

// "Just add audit logging." Stamping now silently stops.
// No compile error. No runtime error. Inserts proceed unstamped.
context.hooks = LifecycleHooks::new(
    MultiLifecycleHooks::default().add(TenantStamp).add(AuditLog),
);
```

With seaolim:

```rust
// Same shape, all five methods forwarded.
context.hooks = LifecycleHooks::new(
    ComposedHooks::default().add(TenantStamp).add(AuditLog),
);
```

`ComposedHooks` also has no default fallthroughs. If upstream adds a sixth hook method, adopting it here is a visible code change, not a silent no-op. `tests/hook_forwarding.rs` reproduces the upstream bug against the real crate and is written to fail the day upstream fixes it.

### 2. Mutations are all-or-nothing per entity

`register_entity!` publishes create-one, create-batch, update, and delete together or not at all. An agent asked for "create only" has two bad options in the base library: expose writes nobody reviewed, or hand-write a create resolver. A complete hand-written resolver needs guards, input parsing, and error mapping, so it is easy for mistakes to hide there.

With seaolim it is a registration decision:

```rust
seaography::register_entity!(builder, notes, mutation: false);
register_generated_mutations::<notes::Entity, notes::ActiveModel>(
    &mut builder,
    GeneratedMutations::CREATE_ONE,
);
```

Everything else about the entity, resolvers, guards, filters, codecs, stays generated.

### 3. Update and delete run no hooks at all

Base Seaography's generated update is one `update_many`. Delete is one `delete_many`. No per-row hook runs, and SeaORM's own `before_save`, `after_save`, `before_delete`, and `after_delete` behaviors are skipped. Any invariant like "status can only move forward" or "rows in use cannot be deleted" forces a hand-written mutation per entity.

seaolim ships SDL-identical replacements:

```rust
register_hooked_update::<notes::Entity, notes::ActiveModel>(&mut builder);
register_hooked_delete::<notes::Entity, notes::ActiveModel>(&mut builder);
```

The hooked update fetches the matching rows in a transaction, folds the patch into each row's `ActiveModel`, and calls `before_active_model_save` per row with `OperationType::Update`. The hook sees old values as `Unchanged` and patched values as `Set`, which is the whole picture DRF gives `perform_update`. A `Block` rolls the transaction back. Delete works the same way with `OperationType::Delete`, so one hook set can veto individual rows. SeaORM's row behaviors run again because each row goes through `ActiveModel::update` or `delete`.

The cost is real: N statements for N matched rows instead of one. At the scale these projects run, that trade is fine. It is recorded in ADR 0001 with a revisit condition.

### 4. Row scoping ignores creates

`entity_filter` scopes reads, updates, and deletes. On create, base Seaography never consults it (upstream issue #233). An agent who wrote a filter saying "callers only see their own workspace" will reasonably believe creates are scoped too. They are not. The insert lands wherever the input says.

seaolim's hooked create-one inserts inside a transaction, then re-selects the new row through the `entity_filter(Create)` condition. Row invisible to the caller means rollback and an error. Same shape as row-level security's `WITH CHECK`.

Restricted model mutations apply the same check whenever their prepared write
is an insert, including natural-key upserts. Their field declaration can also
project the affected-row result to `Boolean!`, which keeps existing flat delete
contracts while seaolim still owns persistence and hook timing.

### 5. Post-save events arrive without the row

`entity_watch` fires after a mutation with an entity name and an action. No row. Callers otherwise have to re-read the table after every write to recover the affected row.

seaolim's `Signals` pairs the save hook, which sees the row, with `entity_watch`, which fires after commit:

```rust
Signals::default().on("Notes", |action, row: Option<notes::ActiveModel>| async move {
    // Create, update, and delete all deliver the affected row here
    // when the hooked builders serve those mutations.
});
```

Handlers are typed per entity. The row travels through a per-request `SignalBuffer`, so concurrent requests never see each other's writes. Attach one with `Request::new(query).data(SignalBuffer::default())`.
Hooked builders refresh the captured row after write-set hooks run, so signal
payloads include set-level amendments.

### 6. A row hook cannot enforce a rule over siblings

`before_active_model_save` sees one folded ActiveModel. It cannot compare
all rows matched by an update, inspect each old revision beside its proposed
replacement, or reject a reorder as one unit. Applications otherwise replace
the whole generated mutation to enforce those rules.

seaolim's hooked builders call a `WriteSetHook` once after every row save hook
allows the mutation and before any persistence. The hook receives the open
transaction and the complete write set. Update and delete rows include both
the fetched model and mutable ActiveModel; create rows include the mutable
ActiveModel.

```rust
struct LeaseRules;

#[sea_orm::prelude::async_trait::async_trait]
impl WriteSetHook for LeaseRules {
    async fn before_write_set(
        &self,
        ctx: &ResolverContext,
        entity: &str,
        action: OperationType,
        transaction: &DatabaseTransaction,
        rows: &mut [WriteSetRow<'_>],
    ) -> GuardAction {
        // Query current siblings through `transaction`, then inspect or
        // amend every proposed ActiveModel in `rows`.
        GuardAction::Allow
    }
}

let schema = builder
    .schema_builder()
    .data(database)
    .data(ComposedWriteSetHooks::default().add_for::<notes::Entity>(LeaseRules))
    .finish()?;
```

The existing `register_hooked_create_one`, `register_hooked_update`, and
`register_hooked_delete` signatures do not change. A `Block` returns a
GraphQL error and rolls back the transaction. Composed hooks run in
registration order for their entity, carry amendments forward, and stop at
the first block.

### 7. A flattened field that writes a set has nowhere to go

`register_restricted_model_mutation` keeps an existing flat field while the
library owns its resolver, but it writes one row. A transactional reorder or
a delete that reassigns its referents writes several rows at once, so the
only remaining option is a hand-written resolver — and a hand-written
resolver runs no hooks.

`register_restricted_model_set_mutation` is its set-shaped sibling. The
domain rule selects and locks the rows; the library runs the per-row save
hooks, the entity-scoped `WriteSetHook` over the complete set, persistence,
the entity watch, and returns the rows as a non-null list.

```rust
#[sea_orm::prelude::async_trait::async_trait]
impl RestrictedModelSetMutation<notes::Entity, notes::ActiveModel> for ReorderNotes {
    async fn prepare(
        &self,
        ctx: &ResolverContext<'_>,
        transaction: &DatabaseTransaction,
    ) -> Result<PreparedModelSet<notes::ActiveModel, notes::Model>> {
        // Lock, read the rows this field writes, return them in write order.
        // The cross-row rule stays in a WriteSetHook.
    }
}

register_restricted_model_set_mutation::<notes::Entity, notes::ActiveModel, _>(
    &mut builder,
    RestrictedMutationField::new("reorder_notes", OperationType::Update)
        .argument(string_argument("parent_id"))
        .argument(InputValue::new("ordered_ids", TypeRef::named_nn_list_nn(TypeRef::STRING))),
    ReorderNotes,
);
```

Call `.returns_boolean()` on the field declaration when an existing set write,
such as a delete with reassignment, returns a success flag instead of rows.
The registrar still runs every hook and persists the full set in one
transaction, then returns whether the set affected at least one row.

`prepare` answers "which rows, in what order". Everything that reads or
amends siblings belongs in the write-set hook, so an application still finds
one rule per entity in one composed place. `tests/restricted_model_set_mutations.rs`
is the worked example.

## Why this makes agent code smaller and safer

An agent extending a project on base Seaography has to know six undocumented facts to avoid shipping a bug, and the failure mode for most of them is silence. An agent extending a project on seaolim follows four rules that fit in a CLAUDE.md:

1. Compose hooks with `ComposedHooks`. Never use `MultiLifecycleHooks`.
2. Register entities with `mutation: false`, then publish writes through the registrars. Pick generated or hooked per operation.
3. Write invariants live in hook sets, one rule per set. A custom mutation needs a recorded reason.
4. Cross-row invariants live in `WriteSetHook` implementations registered for one entity through one `ComposedWriteSetHooks` schema-data value.

Every rule violation is either impossible (the SDL parity test fails) or reviewable in one line of diff. Custom CRUD collapses to registrar calls plus hook sets that say only what the rule is.

## What stays upstream

Reads, filters, ordering, pagination, relations, DataLoaders, guards, column codecs, custom fields. seaolim replaces nothing on the read path and adds no types. If you delete every seaolim call, the schema shrinks but nothing generated changes shape.

## Not solved

Field-pathed validation errors. A rejected write returns one entity-level GraphQL error, not per-field paths like DRF serializers. We decided the ergonomics are acceptable for now; see ADR 0001.

## Versions

Pinned to `seaography =2.0.0-rc.9` and `sea-orm =2.0.1` on Rust 1.95.0. The hooked builders copy upstream resolver behavior, so a version bump requires re-running the SDL parity test and re-reading the diff of upstream's `src/mutation/`. The parity assessment in `docs/seaography-drf-parity.md` records what stock behavior was at pin time. Upstream fix drafts live in `docs/upstream/`; each one merged upstream lets a seaolim module be deleted.

## Docs

- `CONTEXT.md` defines the terms used everywhere (hook set, hooked builder, seam, signal, write surface).
- `docs/seaography-drf-parity.md` is the gap analysis against DRF that motivated all of this.
- `docs/adr/0001-hook-composition-strategy.md` records the no-fork decision and its amendments.
- `tests/` doubles as usage examples. `hook_forwarding.rs` is the upstream bug reproduction, `hooked_mutations.rs` shows every registrar in one schema, `signals.rs` shows typed signal handlers.
