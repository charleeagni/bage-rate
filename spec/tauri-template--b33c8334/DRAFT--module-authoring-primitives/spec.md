# DRAFT (file as Story, rename folder to its T-number) — Module authoring primitives

## Problem Statement

The module system proves composition, ownership, and generated CRUD, but a
Module can express nothing beyond generated CRUD. The worked example
(docs/writing-a-module-by-example.md) shows the consequence: a routine
write-with-behaviour — ticketry's compare-and-swap document save — has no home.
There is no module-level surface for custom operations, no hook point for
validation or side effects, no transaction primitive spanning a Module's own
Models, no computed read fields, no per-operation selection of generated
mutations, and no subscription registration. Every one of these exists in the
substrate (sea-orm transactions; Rustry seaolim's hooked mutations,
ComposedHooks, WriteSetHook, per-operation mutation selection; seaography
custom fields and subscriptions) — but none is reachable from module code, so
each real application would reinvent them as raw Seaography exceptions, which
is exactly the custom code the architecture exists to prevent.

## Solution

The library owns the primitives; Modules only consume them. The seam crate
exposes a closed set of authoring primitives — the complete vocabulary a
module author (human or agent) has — and is sealed so nothing below it is
reachable. The set, with its Django/DRF equivalents fixed as the design
benchmark:

| Primitive | Django analog |
| --- | --- |
| Generated CRUD from a Migration | ModelViewSet defaults |
| `CustomOps`: custom queries, mutations, subscriptions — one file per operation, registered in `module_def()` | views.py + urls.py |
| Write hooks (before/after save, write-set validation) | pre_save/post_save signals, serializer validate() |
| Hooked mutations (generated CRUD with injected behaviour, SDL-identical) | overriding Model.save() |
| Per-operation mutation selection (drop `xUpdate` so a custom op is the only write path) | limiting ViewSet actions |
| `ModuleCtx::transaction(|txn| …)` across the Module's own Models | transaction.atomic |
| Computed read fields; per-field hiding | SerializerMethodField; excluded fields |
| Escape-hatch operation: many reads, branching, a write whose shape follows from them — declared, capped, exception-backed | a bare `@transaction.atomic` view bypassing serializers |

Everything registers through the sealed API, lands in the Module's
mini-schema, and is therefore prefix-checked and per-module codegen'd for
free. Atomicity is a server-side primitive only: a GraphQL request composing
multiple root mutations runs them sequentially, so anything atomic must be one
custom operation.

## Implementation Decisions

### The real seaolim goes underneath

Rustry's `crates/seaolim` (per-operation mutation selection, hooked mutations,
ComposedHooks, WriteSetHook, byte-identical-SDL discipline) becomes the
substance beneath the seam rather than a parallel invention. The template's
current `seaolim` crate is renamed (working name `module-host`) to end the
name collision; ADR-0005's layering stands — safe registration below,
composition above.

### The sealed surface

`ModuleDef` becomes opaque, constructed by one macro; no public signature
mentions `Builder`, `BuilderContext`, or `async_graphql`. Modules lose their
direct `seaography` dependency for hand-authored code (generated entities
retain what codegen requires). Two verify checks make the seal executable:
an import scan over hand-authored module Rust and a dependency allowlist per
module manifest — mirrored on the ui half (module UI sees React,
`@apollo/client` cache types, and its own generated types).

### CustomOps and ModuleCtx

`module_def()` gains a `custom` registration function receiving `&mut
CustomOps` (`.query::<T>()`, `.mutation::<T>()`, `.subscription::<T>()`,
`.hooks(...)`, `.select_generated(...)`, `.computed_field(...)`). Resolvers
receive `ModuleCtx`, which exposes the Store connection, `transaction()`, and
the Module's own entities — nothing else. Custom registrations land in the
mini-schema, so the existing prefix check and per-module codegen cover them
with no new machinery. Cross-module transactions and queries remain host-level
combining code.

### Views delegate; services compute; effects are borrowed

The framework fixes the boundary, not the middle. A resolver body is free
code: it delegates to module-private services (pure logic under the Module's
own source tree, built only from sanctioned dependencies) and the framework
does not constrain what they compute. What it does constrain is every effect:
the Store is reachable only through `ModuleCtx`'s borrowed Models, and any
other I/O a Module ever needs (filesystem, network, processes) arrives the
same way — as a capability handle added to the library, never as a free
import inside a service. Services are not an escape hatch; they are where the
unconstrained computation lives between constrained edges.

### The pressure valve, and its guard

Some work fits no primitive: reading the Store repeatedly, branching on what
was read, and ending in a write whose shape is not known until the reads are
done. The primitives deliberately cannot express this — a hooked mutation has
one write, a custom operation has one declared output type — and pretending
otherwise would either bloat the set or push authors into inventing their own
seams.

Such work is admitted through one named primitive, not through an absence of
rules: an escape-hatch operation that receives the borrowed transaction handle
and may issue arbitrary reads and writes within it, against its own Module's
Models only. It is a primitive so that it is still inside the framework — the
transaction, the Store access, and the schema boundary remain framework-owned;
only the shape of the interaction is free.

It is guarded, and the guard is executable rather than advisory:

- **Declared, not discovered.** The Module names each escape-hatch operation
  in `module_def()` through a distinct registration (not `.mutation()`), so
  its use is visible in the one place a reviewer already reads.
- **Written exception required.** The same discipline AGENTS.md applies to
  replacement CRUD: which primitive was insufficient and why, the smallest
  seam taken, and its drift-prevention test. Verify fails when a declared
  escape hatch has no exception record.
- **Counted and capped.** Verify reports the count per Module and fails above
  a low threshold, so the hatch cannot quietly become the normal way to write
  a Module.
- **Output still typed and prefixed.** The operation declares a concrete
  output type carrying the Module's prefix; "unknown shape" describes the
  interaction, never the contract.
- **Reviewed as a framework signal.** A second Module reaching for the same
  hatch shape is the trigger to design a primitive and delete both
  exceptions — the valve exists to keep work moving while that design
  happens, not to substitute for it.

### Proof

The worked example's document save (transactional compare-and-swap returning
conflict-as-data) is implemented in a real Module as the acceptance test,
alongside: a hook that rejects a write (validation), a subscription delivering
one event over both Transports, a Module that disables a generated mutation
and proves the SDL lost exactly that field, seal tests (imports and
dependencies that must fail verify), and valve-guard tests: a declared
escape hatch without an exception record fails verify, and exceeding the
per-Module cap fails verify. Stability Boundary moves "custom Model
operations" to the proven side when — and only when — these pass.

## Out of Scope

- Per-field resolver override of an existing column (the one Django affordance
  the substrate lacks; the sanctioned idiom is hide-plus-computed-field).
- Cross-module transactions, relations, or type references.
- Ticketry adoption (separate migration; T1001 covers its seaolim step).
- Runtime plugins, external Module distribution (unchanged from ADR-0005/0006).

## Further Notes

Sequencing: rename first (cheap now, expensive after a second consumer), real
seaolim underneath second, sealed CustomOps third, verify seals last. The
primitive list above is closed by decision: an application needing something
outside it writes a host-level exception with the written-exception
discipline, and recurring exceptions are the signal to grow the library — not
the Module.

## As built

Delivered on `agent/harden-template-contracts`. `verify` is green under npm and
Bun, and the composed Server Process was driven by hand to watch each primitive
behave. Everything above is implemented except the two deviations recorded at
the end of this section.

Where each thing landed, in the sequence Further Notes asked for:

| Decision | Where |
| --- | --- |
| Rename first | `crates/seaolim` → `crates/module-host` |
| Real seaolim underneath | `crates/seaolim`, vendored at `d4d19e7c7f107991bb4796e23ec9c3925fd2f435`; only its manifest adapted |
| Sealed `ModuleDef`, `CustomOps`, `ModuleCtx` | `crates/module-host/src/`, with `module_def!`, `#[custom_fields]`, `#[output]` in `crates/module-host-macros` |
| Verify seals last | `scripts/check-module-imports.mjs`, `scripts/check-module-dependencies.mjs`, `scripts/check-escape-hatches.mjs`, all in `verify` |
| The decision record | `docs/adr/0007-module-authoring-primitives.md`; ADR-0006 amended to point at it |

Proof, all against `modules/documents` through the composed schema rather than
a stub:

| Required proof | Test |
| --- | --- |
| Transactional compare-and-swap returning conflict-as-data | `the_compare_and_swap_writes_when_the_expected_digest_still_holds`, `a_conflicting_save_comes_back_as_data_and_writes_nothing` |
| A hook that rejects a write | `a_write_hook_rejects_the_write_and_leaves_the_store_untouched` |
| One subscription event over both Transports | `a_module_subscription_delivers_over_the_desktop_transport`, `a_module_subscription_delivers_over_the_web_transport` |
| A disabled generated mutation, and the SDL losing exactly that field | `dropping_a_generated_mutation_removes_exactly_that_field`, paired with `a_module_that_selects_nothing_keeps_every_generated_write` |
| Seal tests: imports and dependencies that must fail verify | `scripts/module-seal.test.mjs`, each rejection paired with the fixture that must pass |
| Valve-guard tests: a hatch with no exception, and the cap | `scripts/module-seal.test.mjs` |

The Stability Boundary moved accordingly: custom Model operations, lifecycle
hooks and side effects around generated mutations, and an application
subscription field are on the checked side.

### Deviation: per-field hiding is not reachable at this pin

Seaography 2.0.0-rc.9 hides a column only through an attribute on the entity,
and this template regenerates entities wholesale from a scratch Store; there is
no `BuilderContext` path to it. It stays an extension point, with the reason
and the upstream change it needs recorded in ADR-0007 and
`docs/stability-boundary.md`. Computed read fields — the other half of the
hide-plus-computed idiom this spec names as the sanctioned replacement for
per-field override — are implemented and proven.

### Deviation: "after save" is `ModuleCtx::publish`, not a second hook

The write hook runs before persistence inside the mutation's own transaction,
which is what makes rejection roll back. Post-commit work is a typed publish
that subscriptions read from, so a Module announces a save only after it
committed and announces nothing on a conflict
(`a_conflicting_save_publishes_nothing`).

### Still open: the folder rename

This folder keeps its `DRAFT--` prefix. The `T<number>--<slug>` names come from
a tracker outside this repository, and nothing here can assign one; inventing a
number would misfile the Story. Rename the folder once the Story is filed. No
code depends on the folder name.
