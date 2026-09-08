# Tauri GraphQL Template

A copy-ready foundation for applications whose durable state is owned by Rust
and whose UI reads it through a generated GraphQL contract. This glossary fixes
the words the template uses about itself.

## Language

**Model**:
A persistent domain entity whose Rust representation and GraphQL CRUD are
generated from a reversible migration.
_Avoid_: entity, table, record, resource

**Migration**:
The single hand-authored, reversible source of truth for a Model's shape.
_Avoid_: schema change, DDL

**Generated Contract**:
The committed artifacts derived from migrations and authored operations, which
are reviewed as public API and never hand-edited.
_Avoid_: generated code, build output

**Caller Operation**:
A hand-authored GraphQL query, mutation, or subscription written for one
specific caller, from which a typed document is generated.
_Avoid_: query file, API call

**Cache Convergence**:
The declared rule by which a write brings the disposable client cache back into
agreement with Rust.
_Avoid_: cache invalidation, refresh, sync

**Module**:
A reusable vertical feature that contributes to both sides of the Generated
Contract — migrations and schema registrations on the Rust side, Caller
Operations, generated types, and UI on the frontend side — and is combined
with other Modules by a host application at compile time.
_Avoid_: plugin, package, extension, feature flag

**Primitive**:
One member of the closed set of things a Module may express beyond generated
CRUD, offered by the seam and consumed by Modules. Growing the set is a change
to the library; working around it is not an option a Module has.
_Avoid_: helper, utility, API, extension

**Escape Hatch**:
A declared operation admitting work no Primitive expresses. It is counted,
capped per Module, and requires a written exception, so reaching for one stays
a visible decision rather than a habit.
_Avoid_: workaround, custom resolver, raw operation

**Written Exception**:
The record under `docs/exceptions/` that a rule-breaking change owes: which
facility was insufficient, why, the smallest seam taken, and the test that
prevents drift. It is what a later reader needs in order to delete it.
_Avoid_: waiver, TODO, note

**Stability Boundary**:
The line between mechanisms the template declares proven by executable checks
and extension points that each require their own first proof.
_Avoid_: roadmap, supported features

## Targets and transport

**Target**:
One deployable form of the application. The template has exactly two: the
Desktop Target and the Web Target. Both present the same UI and the same
Generated Contract.
_Avoid_: platform, build, mode, version

**Desktop Target**:
The Tauri application. Its Store is a local file owned by the person running it,
and it works with no network.
_Avoid_: native app, tauri app, client

**Web Target**:
The browser application served against a Server Process. Its Store belongs to
the server, not to the visitor.
_Avoid_: web view, web app, hosted version, SPA

**Transport**:
The mechanism that carries a GraphQL request from the UI to the App Schema and
carries results back. Each Target has exactly one.
_Avoid_: link, channel, protocol, API layer

**App Schema**:
The one composed GraphQL schema both Targets execute against, built from the
generated Models.
_Avoid_: server, API, backend

**Store**:
The SQLite database a Target executes the App Schema against. Each Target has
its own; they hold unrelated data and never exchange it.
_Avoid_: database, storage, persistence layer

**Server Process**:
The headless Rust binary that hosts the App Schema over the network for the Web
Target. It is not a Tauri process and links no Tauri code.
_Avoid_: backend, API server, host
