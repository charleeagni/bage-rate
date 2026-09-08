# ADR 0001: Own hook composition locally; fork Seaography only on recorded need

Status: accepted, 2026-08-23.

## Context

Seaography `2.0.0-rc.9` (pinned; latest published) has a composition bug:
`MultiLifecycleHooks` does not forward `before_active_model_save`, so
composing hook sets silently disables save-time validation and mutation.
Full write-path parity with DRF additionally needs three small upstream
patches (save hook on update, `entity_filter` on create, per-row delete
hook). Upstream review latency is months. Details and sizing:
[seaography-drf-parity.md](../seaography-drf-parity.md).

## Decision

1. seaolim owns one composite hook implementing all five
   `LifecycleHooksInterface` methods explicitly, with **no default
   fallthroughs**. `MultiLifecycleHooks` is never used. A new upstream hook
   method must surface as a compile-time or review-time decision here, not a
   silent no-op.
2. The composite dispatches per-field overrides from a registry keyed by
   (entity, field), so one override touches one entry.
3. Selective mutation registration uses `register_generated_mutations`;
   entities register with
   `mutation: false` and publish only the generated writes they can honor.
4. The forwarding fix is submitted upstream regardless (small, unambiguous).
5. No fork of Seaography until a concrete entity needs an update/delete
   invariant that guards, `entity_filter`, and database constraints cannot
   express. That need must be captured in an override record first. Only then carry
   a `[patch.crates-io]` fork with the update/delete hook patches, with the
   explicit goal of retiring it at the next upstream release.

## Amendment (2026-08-23, same day)

Decision 5's "restricted custom seam" for update/delete invariants turned
out to be expressible once, generically, in-library instead of per entity:
`HookedUpdateMutationBuilder` and `HookedDeleteMutationBuilder` re-assemble
SDL-identical update/delete mutations from Seaography's public APIs, fetch
rows in a transaction, and invoke `before_active_model_save` per row with
`OperationType::Update`/`Delete` (action values upstream never sends to the
save hook). An SDL-parity test pins them against upstream drift. `Signals`
likewise pairs the save hook with `entity_watch` to deliver post-commit
events carrying the affected row on every path served by a hooked builder.
`HookedCreateOneMutationBuilder` additionally enforces
`entity_filter(Create)` — upstream issue #233 — by re-selecting the
inserted row through the filter inside the transaction and rolling back
when it is out of scope.

## Amendment: complete write sets

The row save hook cannot express sibling uniqueness, revision checks over
every matched row, or reorder validation. seaolim therefore owns
`WriteSetHook`, which the three hooked builders call once inside their open
transaction after all row save hooks allow the mutation and before any
persistence. `WriteSetRow` pairs old models with proposed ActiveModels for
update and delete. Create rows carry the proposed ActiveModel.

`ComposedWriteSetHooks` follows the same explicit composition rule as
`ComposedHooks`: hooks run in registration order, amendments flow to later
hooks, and the first `Block` stops dispatch. The trait's only method has no
default implementation. Applications attach the composer as async-GraphQL
schema data. This keeps all three `register_hooked_*` signatures unchanged
and avoids modifying Seaography's non-exhaustive `BuilderContext`.

## Consequences

- Stock crates.io dependency today; no fork maintenance tax. The remaining
  reason to fork upstream is gone; upstream PRs are now purely good
  citizenship plus eventual deletion of the local builders.
- Hooked update performs per-row updates instead of one `update_many`:
  hooks gain the old-row view and SeaORM `before_save`/`after_save` run,
  at the cost of N statements for N matched rows. Acceptable at this
  project's scale; revisit if a bulk write path is ever needed.
- Field-pathed validation errors are out of scope; entity-level rejection
  with clear messages is the accepted ergonomics for now.
- Write-set hooks are available only on mutations served by seaolim's three
  hooked builders. The generated create-batch mutation remains upstream and
  has no set-level hook.
