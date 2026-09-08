# CODING-1039 — Full-stack Modules composed into one App Schema

## Problem Statement

The template produces one application whose feature code is global to the
repository: migrations live in one crate-wide directory, schema registrations
in one entities module, Caller Operations and UI in one frontend tree. A team
building several applications from this template has no way to carry a
finished vertical feature — its Models, migrations, operations, generated
types, and UI together — from one application to the next except by copying
files and re-wiring them by hand.

The ecosystem offers no answer. Tauri plugins ship a Rust crate beside a
TypeScript package but know nothing of GraphQL contracts. The one adjacent
project (Mizuki) gives each plugin its own schema and endpoint, which forfeits
the single reviewable SDL, the single normalized Apollo cache, and the Cache
Convergence rules this template is built on. Nothing composes full-stack
modules into one generated contract.

There is also no safe registration surface. Feature code today makes raw
Seaography builder calls, so the repository's write-safety rules (identity-
scoped writes, no exposed auto-increment keys, filter discipline) are enforced
by review rather than by construction.

## Solution

A Module becomes the template's unit of reuse: one folder under `modules/`
holding a Rust half (migrations, registrations) and a frontend half (Caller
Operations, UI), per ADR-0005 and ADR-0006.

Discovery is automatic. Cargo workspace-member globs and npm workspace globs
make a dropped-in folder a build member; `generate` scans `modules/*/`, emits
the composition registry as part of the Generated Contract, and generates each
Module's TypeScript against a standalone mini-schema built from only that
Module's registrations.

Registration is safe by construction. A new thin crate — the SeaOLIM seam —
is the only Rust API a Module may call. It wraps today's Seaography builder
path and owns the safety conventions; Modules never touch the builder
directly. The seam performs registration for one Module and knows nothing
about discovery or composition (ADR-0005 boundary).

Ownership is enforced at generate time. Every Module's tables, types, and
root fields carry the Module's prefix; a Module's operations are validated
against its own mini-schema, so referencing another Module's types fails
generation. At runtime nothing changes shape: one App Schema, one Transport
per Target, one Apollo cache, one `schema.graphql`.

The existing demo Model moves into the first Module, so the mechanism ships
proven by the application's own features rather than beside them.

## User Stories

### Authoring a Module

1. As a module author, I want to create a new Module with one scaffold
   command, so that a working folder with both halves exists before I write
   any feature code.
2. As a module author, I want to author a reversible migration inside my
   Module's folder, so that my Model's shape travels with my feature.
3. As a module author, I want my Rust half to consist of migrations plus
   registration calls against one safe API, so that I cannot violate the
   repository's write-safety rules by construction.
4. As a module author, I want my Caller Operations and UI components to live
   in my Module's folder and be typed against my own generated types, so that
   my feature is self-contained front to back.
5. As a module author, I want to add a Rust or npm dependency by editing only
   my Module's own manifest, so that wiring never leaves my folder.
6. As a module author, I want shared foundational dependencies pinned once at
   the workspace root, so that my Module cannot skew sea-orm, async-graphql,
   React, or Apollo versions.
7. As a module author, I want `generate` to fail with a clear message when my
   registrations are missing my Module's prefix, so that namespace discipline
   is a build fact rather than a review comment.
8. As a module author, I want an operation that names another Module's types
   to fail at generation, so that staying in my lane is enforced, not asked.

### Hosting Modules

9. As a host developer, I want adding a Module to be: create or copy the
   folder, run `generate`, review the diff, so that composition requires no
   hand-wiring.
10. As a host developer, I want removing a Module to be the reverse — delete
    the folder, run `generate`, review the diff — so that no orphaned wiring
    survives.
11. As a host developer, I want the composed `schema.graphql` to remain the
    single reviewable public API, so that Modules change what the contract
    contains but never how it is reviewed.
12. As a host developer, I want cross-module views to be explicit host-level
    Caller Operations generated against the composed schema, so that
    combination is a visible decision with an owner.
13. As a host developer, I want migrations from all Modules composed into one
    deterministic sequence, so that both Targets' Stores are built the same
    way every time.

### Keeping the boundary honest

14. As a maintainer, I want the registry that links Modules into the host to
    be generated, committed, and drift-checked, so that composition is part of
    the Generated Contract.
15. As a maintainer, I want `verify` to prove the composed application still
    answers queries, mutations, and both Targets' checks with the demo Model
    now inside a Module, so that the module system is inside the Stability
    Boundary from its first commit.
16. As a maintainer, I want the file tree to show `modules/` as the feature
    surface of the application, so that a reader learns the architecture
    before opening code.
17. As a developer reading a Module, I want its Rust half to contain no raw
    Seaography builder calls, so that the SeaOLIM seam is demonstrably the
    only registration path.

## Implementation Decisions

### The SeaOLIM seam (new crate, prerequisite)

A new crate `crates/seaolim` exposes the registration API Modules call. Its
initial surface is deliberately small:

- a `ModuleDef` (name/prefix, migration list, registration function) that a
  Module's `rust/src/lib.rs` exports;
- a registration handle wrapping the Seaography `Builder`, exposing only safe
  registration (today's `register_entity_modules` behaviour moves behind it);
- the prefix check: registrations whose table or type names lack the Module's
  prefix are rejected with an error naming the offender.

The crate registers one Module's models and knows nothing about discovery,
folders, or composition. Growth of SeaOLIM (downstream reconciliation, richer
safety rules) happens behind this surface without touching Module code.

`app-schema`'s own entity registration is routed through the seam as part of
this ticket, so there is exactly one registration path in the workspace.

### Module folder anatomy and discovery

```
modules/<name>/
├── rust/                # crate <name>-module, workspace member via glob
│   ├── Cargo.toml       # deps only; shared deps use { workspace = true }
│   └── src/
│       ├── lib.rs       # exports the ModuleDef; calls only seaolim
│       └── migrations/  # hand-authored, reversible, unchanged rules
└── ui/                  # npm workspace member via glob
    ├── package.json     # deps only
    ├── operations/      # Caller Operations (.graphql)
    ├── generated/       # per-module generated types (committed)
    └── components/
```

Root `Cargo.toml` members gain `modules/*/rust`; root `package.json`
workspaces gain `modules/*/ui`. Dropping a folder in makes it a build member
with no edits elsewhere. A `new-module` script stamps out this skeleton with
the prefix wired through.

### The generated registry

`generate` scans `modules/*/` and emits a registry crate
(`crates/module-registry`, fully generated: its `Cargo.toml` carries a path
dependency per Module, its source composes every `ModuleDef` — migration
sequence first, registrations second — and hands `app-schema` one composed
input). The registry is committed, reviewed, never hand-edited, and covered by
the existing drift check. `app-schema` depends on the registry and stops
knowing individual features.

Migration ordering is deterministic: Modules sorted by name, each Module's
migrations in their declared order. Cross-module migration dependencies are
rejected at generation in this ticket (see Out of Scope).

### Per-module codegen against a mini-schema

For each Module, `generate` builds a standalone schema from only that
Module's registrations and runs GraphQL codegen for the Module's operations
against it, writing into the Module's `ui/generated/`. Because composition is
additive and prefix-namespaced, per-module types remain exactly valid against
the composed App Schema at runtime. The composed `schema.graphql` and the
host-level generated documents continue to be produced as today.

An operation in `modules/a/ui/operations/` that references `modules/b`'s
types fails this step — that is the ownership enforcement, delivered by the
existing codegen toolchain rather than a new checker.

### The first Module is the existing feature

The demo Model's migration, registration, Caller Operations, generated types,
and UI components move from their current global locations into
`modules/<demo>/`. The host keeps only the app shell, Transport selection, and
composed-schema wiring. This is the proof that the mechanism carries a real
feature: `verify` passing after the move is the acceptance test.

### What runtime never learns

No runtime discovery, no dynamic loading, no per-module endpoints, no
per-module caches. Both Targets keep exactly the shape ADR-0001..0004 fixed;
the module system exists entirely at authoring and generation time.

## Testing Decisions

### Seams

**The composed application (existing).** The Tauri composition test and the
composed Server Process tests keep passing unchanged with the demo feature
inside a Module — this proves composition end to end over real sockets and a
real temporary Store, and is the primary acceptance evidence.

**The generate pipeline (existing layer, new assertions).** Script-level
tests in the repository's existing `scripts/*.test.mjs` style:

- generate is idempotent: a second run after a clean run produces no diff
  (extends the drift check to the registry and per-module output);
- an unprefixed registration in a fixture Module fails generation with a
  message naming the Module and the offending name;
- a fixture operation referencing another Module's type fails per-module
  codegen;
- the composed SDL equals the union of the Modules' mini-SDLs plus root
  plumbing — asserting additive composition with no collisions.

**The seam crate (new, minimal).** One Rust test drives `seaolim` with a
fixture entity and asserts registration lands in a built schema; one asserts
the prefix rejection. No tests reach into builder internals.

### Rejected seams

Unit tests of registry file contents (implementation structure — the drift
check and composed-application tests already pin behaviour); a plugin-style
in-process composition simulator (duplicates the composed-application seam);
browser-driven tests (still rejected as in CODING-654).

### Continuous integration

No new jobs. Existing drift, TypeScript, Rust matrix, bundle, and both
package-manager paths must pass with `modules/` present. The dependency audit
already covers the whole workspace, which now includes Module crates via the
glob.

## Out of Scope

- **Publishing Modules externally.** Crate + npm distribution (the Tauri
  plugin shape) is a later tooling step; the authoring format is the folder.
- **Cross-module migration dependencies.** Rejected at generation for now; a
  Module needing another Module's tables is a design smell this ticket does
  not legitimise.
- **Cross-module type references or schema stitching.** Combination happens
  only in host-level operations against the composed schema.
- **Runtime plugins** in any form (ADR-0005 records why).
- **Per-module Stores, endpoints, or caches** (ADR-0006 records why).
- **Reconciling with the downstream SeaOLIM implementation.** The seam's
  surface is designed here; porting richer logic into it is separate work.
- **A second example Module.** One real Module proves the mechanism; a
  synthetic second module exists only as a test fixture.
- **Module-level authorization or visibility rules.**

## Further Notes

The Module term entered the project glossary during this design series; the
decisions are recorded as ADR-0005 (a separate compile-time module system)
and ADR-0006 (folder authoring, generate-time ownership, SeaOLIM as the sole
registration path).

Two tensions to carry forward:

The SeaOLIM seam is designed template-first while a richer implementation
exists downstream. The seam surface here is the contract; if the downstream
crate cannot adopt it, the reconciliation happens in the downstream port, not
by widening the template's surface.

Per-module codegen assumes additive, collision-free composition. The prefix
check is what makes that assumption safe; if the check is ever weakened, the
"module types are valid against the composed schema" property silently stops
being true. The generate-pipeline tests exist to keep that link visible.

Documentation to update alongside the code: the README's description of the
feature surface, the architecture document's tree diagram, AGENTS.md's Model
workflow (steps now happen inside a Module's folder), and the Stability
Boundary document, which gains the module composition mechanism on the proven
side and external Module distribution on the unproven side.
