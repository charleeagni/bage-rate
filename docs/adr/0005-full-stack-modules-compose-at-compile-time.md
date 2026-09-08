---
status: accepted
---

# Full-stack modules compose at compile time, in their own layer

Applications built from this template want to reuse whole vertical features
(migrations, schema registrations, Caller Operations, generated types, UI)
across Tauri applications. We decided to add a separate full-stack module
system — working name `tauri-graphql-modules` — rather than extend any
existing layer, and to compose modules at compile time: the host application
statically combines every module's contributions into one App Schema and one
frontend build.

## Boundaries this fixes

- There is still exactly one GraphQL TauRPC Endpoint per Target. GraphQL
  already dispatches every operation, so modules never add endpoints.
- `tauri-graphql-transport` stays unchanged. It accepts any finished App
  Schema and knows nothing about how that schema was assembled.
- The Seaography/SeaORM layer keeps its single job of safe mutation
  registration. Composition logic does not live there.
- A module contributes to both sides of the Generated Contract: Rust
  migrations and schema registrations on one side; Caller Operations,
  generated types, and UI on the other. The host application is the only place
  where modules meet.

## Considered Options

Runtime plugins would let modules be added without rebuilding the host, but
they bring unsolved problems this template has no proof for: migration
ordering across independently shipped modules, replacing a live schema,
subscription lifecycle across reloads, and regenerating the typed contract at
runtime. Compile-time composition keeps the existing generate-and-review
workflow intact; each of those problems would need its own first proof before
the Stability Boundary could move.

Extending `tauri-graphql-transport` or the mutation-registration layer to do
composition was rejected because it would give each of those layers a second
concern and couple every host application to one composition strategy.
