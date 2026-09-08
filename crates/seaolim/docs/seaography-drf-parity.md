# Seaography vs DRF: parity assessment

Written 2026-08-23. Facts audited against Seaography `2.0.0-rc.9` (latest
published tag) and upstream `main` as of this date. Re-audit when the pinned
version changes.

Status update, same day: every write-path gap marked "Gap" below except
field-pathed validation errors is closed locally by this crate —
`ComposedHooks` (safe composition), `register_generated_mutations`
(selective bundle), `HookedCreateOneMutationBuilder` (enforces
`entity_filter` on create, issue #233), `HookedUpdateMutationBuilder` /
`HookedDeleteMutationBuilder` (per-row save hook with old-row view on
update/delete, SeaORM row behaviors restored), and `Signals` (post-commit
events carrying the affected row on all three write paths). SDL parity with
the upstream bundle is pinned by test. The table is kept as the record of
stock-Seaography behavior; see ADR 0001's amendment for what supersedes it.

## Use case

Match Django REST Framework's extension model in a Rust GraphQL backend:
override serialization, resolution, and validation **per field**, across all
generated queries and mutations, so each override's blast radius is one
field or one rule — never a whole hand-written endpoint.

Seaography is the chosen base. It generates the GraphQL schema from SeaORM
entities and exposes lifecycle hooks (`LifecycleHooksInterface`):
`entity_guard`, `field_guard`, `entity_filter`, `before_active_model_save`,
`entity_watch`.

## Parity: read path (queries)

| DRF capability | Seaography rc.9 equivalent | Status |
| --- | --- | --- |
| Object permissions | `entity_guard(Read)` | Parity |
| Per-field read permission | `field_guard(Read)`, can downcast parent row | Parity |
| Queryset scoping (`get_queryset`) | `entity_filter(Read)`, applied to all generated queries incl. relations | Parity |
| Custom field formatting | `column_options.output_conversion` per entity+column | Parity |
| `SerializerMethodField` | `#[CustomFields]` on the Model (async, ctx-aware) | Parity |
| Replace one field's resolver | Ignore the column + `#[CustomFields]` method of the same name | Parity (pattern) |
| Ctx-aware output conversion | `output_conversion` has no `ResolverContext`; use the ignore+CustomFields pattern | Partial |

## Parity: write path (mutations)

| DRF capability | Seaography rc.9 equivalent | Status |
| --- | --- | --- |
| Enable only some generated mutations | All-or-nothing upstream; `register_generated_mutations` fixes it locally | Parity (local wrapper) |
| Write permission per entity and field | `entity_guard` / `field_guard` with Create, Update, Delete | Parity |
| `perform_create` (mutate/reject before insert) | `before_active_model_save` — create only | Parity, **after the composition fix below** |
| Post-save signals | `entity_watch` (upstream fix pending for the update path) | Partial |
| `perform_update` / inspect old row | None; update goes straight to `update_many` | Gap (largest) |
| Scope what a user may create | `entity_filter` not applied on Create (upstream issue #233) | Gap |
| `validate_<field>` with field-pathed errors | None; entity-level reject only, unstructured error | Gap |
| Object-level `validate()` | Create only, via `before_active_model_save`; never on update | Partial |
| `perform_destroy` / per-row delete hook | `delete_many`; SeaORM delete callbacks skipped | Gap |
| Ctx-aware input transformation | `input_conversion` is value-only, no ctx | Partial |

## The composition bug

`MultiLifecycleHooks` (rc.9 and upstream `main`, checked 2026-08-23)
forwards only four of the five hooks. `before_active_model_save` falls
through to the trait default, which returns `GuardAction::Allow` and does
nothing. Composing two hook sets therefore silently disables every child's
save-time mutation and validation. No error is raised; inserts proceed
unchecked. Not reported upstream. Nearest upstream issues: #238 (closed;
motivated the hook's existence), #233 (open; `entity_filter` on create).

This bug blocks the per-field-override architecture directly: small blast
radius requires composing many small hooks, and composition is exactly what
is broken.

## Work sizing

| Item | Size | Where |
| --- | --- | --- |
| Copy wrapper + policy docs from prior art | Copy-paste | seaolim |
| Application-owned composite hook (all five methods, no default fallthroughs) + regression test | ~100 lines | seaolim |
| Forwarding fix for `MultiLifecycleHooks` | ~15 lines incl. test | Upstream PR |
| `before_active_model_save` on the update path | ~40–60 lines; update already fetches rows in a transaction | Upstream PR |
| `entity_filter` on create (#233); per-row delete hook | Same shape and size | Upstream PRs |
| Field-pathed validation errors | Design conversation, not a patch | Deferred |

Code volume is small. The real cost is dependency logistics: upstream
releases slowly (rc.9 latest since mid-2026; PRs sit unreviewed since late
2025), so using the update/delete patches before release means carrying a
`[patch.crates-io]` fork across sea-orm/seaography bumps.

Strategy decision: see ADR 0001.

## Upstream references

- Hooks source (rc.9): <https://github.com/SeaQL/seaography/blob/2.0.0-rc.9/src/builder_context/hooks.rs>
- Update mutation: <https://github.com/SeaQL/seaography/blob/2.0.0-rc.9/src/mutation/entity_update_mutation.rs>
- Issue #233 (`entity_filter` on create): <https://github.com/SeaQL/seaography/issues/233>
- Issue #238 (closed; save-time data access): <https://github.com/SeaQL/seaography/issues/238>
- Seaography 2.0 announcement: <https://www.sea-ql.org/blog/2025-10-08-seaography/>
