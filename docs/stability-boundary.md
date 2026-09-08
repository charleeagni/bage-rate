# Stability boundary

## Included and checked

Shared by both Targets:

- persistent SQLite and automatic forward migrations
- reversible, migration-first SeaORM Model generation
- compile-time Module composition, held by four executable checks: the
  module-host tests that prove registration lands in a built schema, that a
  freshly scaffolded Module composes before its first migration, and that an
  unprefixed registration is rejected naming its Module; the drift groups
  covering the generated module registry, module index, per-module entities,
  and per-module documents; the module-ownership tests that prove codegen
  rejects an operation naming another Module's types; and the composed
  Tauri and Server Process tests, which drive the demo Models from inside
  their Modules
- Seaography query, create, batch create, update, delete, filters, ordering,
  and pagination
- the closed set of authoring primitives a Module has beyond generated CRUD
  (ADR-0007), each proven against the `documents` Module through the composed
  schema rather than a stub:
  - custom queries, mutations, and subscriptions registered through the seam,
    landing in the Module's mini-schema and so covered by the same prefix check
    and per-module codegen as generated fields — including a custom query whose
    declared signature is asserted against the exported SDL and whose three
    answers, one of them nullable, are driven through the composed schema
  - lifecycle rules over a mutation's complete write set, including a rejected
    write that rolls its transaction back and returns the rule's own reason
  - hooked generated writes, SDL-identical to the generated ones by a pinned
    test in the substrate
  - per-operation write selection, including a Module that drops a generated
    mutation and an assertion that the SDL lost exactly that field and the
    input that served it
  - a transaction across one Module's own Models, proven by a
    compare-and-swap whose conflict comes back as data and whose losing write
    is confirmed absent from the Store
  - computed read fields on a generated Model
- the seal on that surface: an import scan over each Module's hand-authored
  Rust and TypeScript, and a dependency allowlist over both manifests, each
  proven to reject by `scripts/module-seal.test.mjs` against fixture Modules
- the escape-hatch guard: a declared hatch without a written exception fails
  `verify`, an exception missing the discipline's sections fails `verify`, and
  a Module declaring more than two fails `verify`
- one application subscription field in the App Schema, its Module-owned
  source adapter, and its delivery over both Transports
- valid schema export, with the Subscription root declared only because a
  Module registered a field on it
- operation-only GraphQL Code Generator output
- tested Apollo normalization, create/update/delete list convergence, and
  convergence for a custom write whose result is not the entity
- generated-artifact drift detection
- build-time Transport selection, and a check on both built bundles proving
  neither Target carries the other's Transport
- the template configuration script and its own tests
- Rust, TypeScript, transport, migration, and contract tests

Desktop Target:

- Tauri 2 composition and capability configuration
- app-data Store owned by the person running the application, with no network
- one GraphQL-over-TauRPC unary command path
- GraphQL subscription transport over one Tauri channel per subscription,
  proven both against a stub the test controls and against a Module's own
  subscription over the real App Schema
- generated TauRPC TypeScript bindings

Web Target:

- the composed Server Process, driven over real loopback sockets: real App
  Schema queries and mutations whose effects are confirmed in a temporary Store
- static root assets and the unmatched-path fallback that makes client-side
  deep links load the application
- a Content Security Policy stating the Desktop Target's intent for one HTTP
  origin, sent with `X-Content-Type-Options` and `Referrer-Policy` on every
  response of the composed process, asserted on the wire. These constrain what
  the served document may load; they are not what makes the Server Process safe
  to expose
- the full graphql-ws lifecycle — initialisation, subscribe, at least one
  event, complete — with server-side resources confirmed released, proven at
  the Transport level against a stub schema, and a Module's own subscription
  driven end to end over the same socket
- a malformed request rejected without stopping the process
- the loopback-only default bind address
- the Server Process linking no Tauri code and building with no webview
  toolchain, asserted against its resolved dependency graph on every platform
  rather than against a build on a machine that already has the Tauri
  prerequisites installed

- every script in `scripts/` parses, which is the only check that reaches
  `dev-web.sh`; the decisions both Web Target commands encode — the Target the
  bundler builds, the package and profile the Server Process is built with, the
  paths the build announces, the supervised child process, and the development
  Store default — are asserted against those scripts' text

Both Targets are proven by one command, `verify`, and CI runs it under both
supported package managers. CI also runs each Target's ship command, so
`build-web.sh` is executed rather than only read. `dev-web.sh` is never run to
completion by a check: it starts a session and waits.

## Extension points that require their first proof

- authentication and authorization, including per-user scoping over generated
  CRUD, whose filters are unscoped. This is what gates exposing the Server
  Process beyond loopback: it ships no authorization at all, so anything that
  reaches its port can read and write every row.
- any sharing of data between the two Targets' Stores — no sync, export,
  import, or migration path exists, by decision rather than omission
- distributing a Module outside this repository — publishing its crate and npm
  halves, as Tauri plugins ship — is a later tooling step; the folder under
  `modules/` is the authoring format
- cross-module migration dependencies: each Module's entities are generated
  from a scratch Store holding only its own migrations, so a migration that
  needs another Module's tables fails generation today rather than composing
- cross-module transactions, relations, and type references. A Module's
  transaction reaches its own Models; combining Modules is host-level code
- capabilities beyond the Store. A Module reaches the filesystem, the network,
  or a process only through a handle `module-host` gives it, and no such handle
  exists yet
- public deployment concerns: TLS, reverse proxies, container images, process
  supervision, scaling, and pooling for concurrent users, and the transport
  headers that come with them — `Strict-Transport-Security`, frame and
  permissions policy, and any CSP widening a real bundle needs
- multi-user behaviour: per-user data, presence, and conflict handling
- foreign-key relation exposure
- per-field hiding on generated entities, and the per-field resolver override
  it would enable. Seaography 2.0.0-rc.9 hides a column only through an
  attribute on the entity, and entities here are regenerated wholesale, so this
  needs an upstream change before hide-plus-computed-field becomes the idiom
- policy annotations on generated entities
- event delivery guarantees: a Module's subscription is a live notification
  channel with a bounded backlog, so an event raised while nobody is listening
  is dropped and a subscriber that falls far enough behind skips ahead
- custom client-visible GraphQL error mapping
- browser-driven end-to-end testing
- updater, signing, distribution, telemetry, and crash reporting

Do not infer a stable pattern for an extension from an adjacent mechanism.
Build the narrowest executable experiment, record the contract, then promote
the proven shape into the shared packages.
