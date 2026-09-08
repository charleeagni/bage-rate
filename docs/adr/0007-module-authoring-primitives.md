---
status: accepted
---

# A Module's authoring surface is a closed set of primitives, and it is sealed

Building on ADR-0005 and ADR-0006, a Module could express nothing beyond the
CRUD its migration generates. Everything past that — a write with behaviour, a
validation rule, a transaction, a derived field, a subscription — had no home
inside a Module, so each application would have reinvented it as a raw
Seaography exception. That is exactly the custom code this architecture exists
to prevent.

We decided that the library owns the primitives and Modules only consume them.
`crates/module-host` exposes one closed set — the complete vocabulary a module
author, human or agent, has — and hand-authored Module code may name no layer
beneath it.

## The set

Django and DRF are the benchmark, because the question a primitive answers is
"what does a framework already give an application author, so that they do not
write it again".

| Primitive | Django analog |
| --- | --- |
| generated CRUD from a migration | `ModelViewSet` defaults |
| `CustomOps::query` / `mutation` / `subscription`, one file per operation, registered in `module_def!` | `views.py` plus `urls.py` |
| `CustomOps::write_hook` over a mutation's complete write set | `pre_save` signals, serializer `validate()` |
| `Writes::HOOKED`, SDL-identical to the generated write | overriding `Model.save()` |
| `Writes::without`, so a custom operation is the only write path | limiting ViewSet actions |
| `ModuleCtx::transaction` across the Module's own Models | `transaction.atomic` |
| `CustomOps::computed_field` | `SerializerMethodField` |
| `CustomOps::escape_hatch` | a bare `@transaction.atomic` view bypassing serializers |

Everything registers through the seam, lands in the Module's mini-schema, and
is therefore prefix-checked and per-module codegen'd for free. Atomicity is a
server-side primitive only: a GraphQL request composing several root mutations
runs them one after another, so anything atomic is one operation.

## The substrate is the real seaolim, not a parallel invention

`crates/seaolim` is vendored from the standalone library of that name:
per-operation mutation selection, hooked create/update/delete that run rules
the generated writes have no hook point for, `ComposedHooks`, `WriteSetHook`,
and the SDL-parity test that pins the hooked writes byte-identical to the
generated ones. The template's own seam is renamed `module-host` to end the
name collision.

The layering from ADR-0005 stands, with one more floor: `seaolim` closes
Seaography's write-path gaps and knows nothing about Modules; `module-host`
composes Modules, enforces namespaces, and decides what a Module may say;
neither knows how Modules are discovered.

## The seal is a check, not a compile error

`ModuleDef` is opaque and built by one macro. No documented signature mentions
`Builder`, `BuilderContext`, or `async_graphql`; the two attribute macros
expand to absolute paths so a Module never has to import what they reach.

It could not be made a compile error. The generated entity directory resolves
`seaography`, `sea-orm`, and `tokio` by crate name, and a manifest is
crate-wide, so those crates have to remain dependencies of every Module. Two
checks close the gap instead: `check-module-imports.mjs` holds hand-authored
files — `entities/` and `generated/` excluded — to naming `crate` and
`module_host` and nothing else, and `check-module-dependencies.mjs` holds both
manifests to an allowlist. `scripts/module-seal.test.mjs` proves both reject.

## Views delegate, services compute, effects are borrowed

The framework fixes the boundary, not the middle. A resolver body is free code
that delegates to module-private services, and nothing constrains what they
compute. What is constrained is every effect: the Store is reachable only
through `ModuleCtx`, and any other I/O a Module ever needs — filesystem,
network, a process — arrives the same way, as a capability added to
`module-host`, never as a free import inside a service.

## Considered Options

**Letting Modules call Seaography directly for anything the set omits** was
rejected for the reason the set exists: every application would grow its own
private conventions for hooks, transactions, and custom writes, and the
reviewable surface would be the whole of Seaography rather than one file.

**Growing the set until nothing needs an exception** was rejected because the
work that fits no primitive — many reads, branching on what was read, and a
write whose shape follows from them — is not one shape. Admitting it as
`escape_hatch` keeps it inside the framework: the transaction, the Store
access, the output type, and the schema boundary stay framework-owned, and
only the shape of the interaction is free. It is declared rather than
discovered, capped per Module, and requires a written exception, so the count
is a number CI reports rather than something review has to notice.

**Per-field hiding** was left out, not deferred by preference. Seaography
2.0.0-rc.9 hides a column only through an attribute on the entity, and entities
here are regenerated wholesale from a scratch Store. Reaching it needs an
upstream change, so it stays on the unproven side of the Stability Boundary
with hide-plus-computed-field as the intended idiom once it lands.

## Consequences

A Module now declares a subscription, so the App Schema declares a Subscription
root and both Transports carry an application field rather than only a stub.

`Writes` replaces a Module's whole generated write surface when it is used at
all, because registration is all-or-nothing per Model: publishing three of four
mutations means dropping what registration added and re-registering the
selection. A Module that selects writes selects them for every Model it owns,
and the reviewed SDL diff is where an omission shows up.
