---
status: accepted
---

# A Module is one folder; ownership is enforced at generate time, not runtime

Building on ADR 0005, a Module is authored as a single folder containing a
Rust half (migrations, schema registrations) and a frontend half (Caller
Operations, UI). Discovery and wiring are automatic: workspace globs make the
folder a build member, and `generate` scans the module tree and emits the
host's registry as part of the Generated Contract. The author writes feature
code and per-module dependency manifests; everything that connects a Module to
the host is generated, reviewed, and drift-checked — never hand-wired.

A Module's Rust half performs its registrations exclusively through the seam
(`crates/module-host`); Modules do not make raw Seaography builder calls. The
boundary from ADR 0005 is unchanged in the other direction: the safe mutation
registration layer beneath the seam (`crates/seaolim`) registers one Module's
models safely and knows nothing about Modules, discovery, or composition — the
module system composes what it registered.

ADR-0007 amends this in one direction only: it fixes what a Module may say
through that seam as a closed set, and makes "registers only through the seam"
a check rather than a convention. Nothing about the folder format, discovery,
or generate-time ownership changes.

Ownership follows the per-endpoint intuition without per-endpoint costs. Every
Module's registrations are namespaced by its prefix, and each Module's
TypeScript is generated against a standalone mini-schema built from only its
own registrations. A Module therefore cannot name, query, or depend on another
Module's types — its operations fail generation, not review. At runtime all
Modules still compose into the one App Schema on the one Transport per Target,
preserving the single contract, single Apollo cache, and Cache Convergence
rules. Cross-module views are explicit host-level code with operations
generated against the composed schema.

## Considered Options

Physical separation (one schema and endpoint per Module, as Mizuki does) was
rejected: it forfeits cross-module cache normalization, makes one reviewable
SDL impossible, multiplies Web Target endpoints, and pretends isolation while
Modules still share one Store. Because additive namespaced composition cannot
collide, per-module generated types remain valid against the composed schema,
so generate-time enforcement yields the same authoring semantics at no runtime
cost.

## Consequences

Modules within a repository cannot version-skew: both halves live in one
folder and shared dependencies inherit the workspace's pinned versions.
Publishing a Module externally (crate + npm package, as Tauri plugins ship) is
a distribution step tooling can add later; it is not the authoring format.
