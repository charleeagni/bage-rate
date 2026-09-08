# CODING-654 — A Web Target for the template

## Problem Statement

The template produces exactly one deployable form of an application: a Tauri
desktop application whose UI reaches its data through Tauri IPC. Anyone who
wants the same application in a browser has no path forward.

The obstacle is not packaging. The application has no HTTP server, no open
port, and no network transport of any kind; its GraphQL execution is reachable
only from inside the Tauri process. The UI's only Apollo link is the TauRPC
one, so a browser build of the existing UI compiles, loads, and then fails on
its first operation with no way to reach a schema. The two scripts that appear
to build a browser version in fact build the asset bundle that the desktop
application embeds.

A developer who copies this template today is therefore committed to desktop
only, and discovers that commitment late — after the Models, Caller
Operations, and UI are already written against a Transport that cannot leave
the process.

## Solution

The template gains a second Target. The same UI, the same App Schema, and the
same Generated Contract are deployable either as the existing Desktop Target or
as a new Web Target served in a browser by a headless Server Process.

Both Targets ship in every copied application and both are proven by `verify`,
so the browser path cannot rot while nobody is looking at it.

The two Targets are deliberately independent. The Desktop Target is unchanged:
still local-first, still owning a Store on the user's own machine, still
working with no network. The Web Target has its own Store owned by the Server
Process. Data is never exchanged between them.

This ticket delivers the mechanism, not a hosted product. The Server Process
ships no authentication or authorization and binds to loopback by default. It
is documented as unsafe to expose publicly, and serving real users over the
internet is gated on a separate authorization ticket with its own proof.

## User Stories

### Choosing and running a Target

1. As a developer evaluating the template, I want it to state that it produces
   both a Desktop Target and a Web Target, so that I can tell before I commit
   whether it covers the deployments I need.
2. As a developer who has copied the template, I want to run the Desktop Target
   exactly as before, so that adopting the Web Target costs me nothing if I do
   not want it.
3. As a developer, I want a single command to run the Web Target in
   development, so that I can see the UI in a browser without assembling a
   server and a bundler by hand.
4. As a developer, I want the development Web Target to keep hot module
   replacement and browser devtools, so that UI iteration is faster in the
   browser than it is in the desktop webview.
5. As a developer, I want a single command to build the Web Target for
   deployment, so that shipping it is not a bespoke procedure I have to
   discover.
6. As a developer, I want the script that builds the desktop asset bundle to be
   named for what it does, so that I am not misled into thinking the template
   already had a Web Target.
7. As a developer, I want the Server Process to be one binary I can run, so
   that deploying the Web Target does not require an application server, a
   process manager, or a reverse proxy to get started.

### Using the Web Target

8. As a user of the Web Target, I want to open a URL and see the application,
   so that I can use it without installing anything.
9. As a user of the Web Target, I want to list, create, rename, and delete
   Models exactly as I can on the Desktop Target, so that the browser version
   is not a degraded imitation.
10. As a user of the Web Target, I want live data to update without me
    refreshing, so that subscriptions behave the same as they do on desktop.
11. As a user of the Web Target, I want a deep link or a refresh on any route
    to load the application rather than a not-found page, so that the browser's
    address bar works the way I expect.
12. As a user of the Desktop Target, I want the application to keep working
    with no network, so that the arrival of a Web Target does not make my
    offline application depend on a server.

### Understanding the data boundary

13. As a developer choosing between Targets, I want it stated plainly that the
    two Targets hold unrelated data, so that I do not assume a project created
    on desktop will appear in the browser.
14. As a developer, I want the absence of any sync or migration path between
    the two Stores documented as a decision rather than a gap, so that I do not
    go looking for a feature that was deliberately not built.
15. As an operator of the Server Process, I want to choose where its Store
    lives, so that I can put it on the volume I intend to back up.
16. As a developer running tests, I want no test to ever touch a real Store, so
    that running the suite cannot destroy my own data.

### Safety of the unauthenticated server

17. As an operator, I want the Server Process to bind to loopback unless I say
    otherwise, so that starting it does not silently expose an unauthenticated
    database to my network.
18. As an operator, I want the lack of authorization stated in the
    documentation and in the Stability Boundary, so that I am not left to infer
    it from the absence of a login screen.
19. As a maintainer, I want the loopback default asserted by a test, so that a
    future refactor cannot quietly widen it.
20. As a developer, I want to know that generated CRUD exposes unscoped
    filters, so that I understand why exposing this server publicly is unsafe
    rather than merely unaudited.
21. As a maintainer, I want authorization tracked as its own work with its own
    proof, so that the Stability Boundary keeps meaning what it says.

### The shape of the codebase

22. As a developer reading the repository for the first time, I want the file
    tree to show that this project has two Targets, so that I learn its shape
    before opening any code.
23. As a developer, I want the Rust code that is not Tauri to live outside the
    Tauri directory, so that the tree does not misdescribe what the project is.
24. As a developer, I want Migrations, Models, the App Schema, and Store access
    to live in one crate shared by both Targets, so that there is exactly one
    definition of the domain.
25. As a developer, I want the Server Process to link no Tauri code, so that
    building and deploying it does not require desktop platform prerequisites.
26. As a developer deploying to a container, I want a headless build that does
    not need a webview toolchain, so that images stay small and builds stay
    fast.
27. As a developer, I want crate and script names drawn from the project
    glossary, so that the tree and the documentation use one vocabulary.

### One UI, two Transports

28. As a developer, I want one UI codebase serving both Targets, so that a
    feature written once appears in both.
29. As a developer, I want each Target's bundle to contain only its own
    Transport, so that a defect in one Transport cannot reach the other Target.
30. As a user of the Web Target, I want no desktop IPC code delivered to my
    browser, so that I am not downloading code that can never run.
31. As a developer, I want Caller Operations, generated documents, and Cache
    Convergence rules to work unchanged across both Targets, so that the
    Generated Contract stays the single source of truth.
32. As a developer, I want the client to reach its endpoints without any URL
    configuration, so that there is no environment variable to get wrong
    between development and deployment.
33. As a maintainer, I want no CORS configuration to exist, so that there is no
    security surface to review that the design does not need.

### Proof and continuous integration

34. As a maintainer, I want `verify` to prove the Web Target answers real
    queries and mutations over a real socket, so that the Web Target is inside
    the Stability Boundary on the same terms as everything else.
35. As a maintainer, I want `verify` to prove a subscription completes a full
    handshake, delivers an event, and releases its server-side resources, so
    that the browser subscription path is proven and not merely compiled.
36. As a maintainer, I want `verify` to prove the Web Target bundle contains no
    desktop Transport code, so that the two-bundle split is a checked fact.
37. As a maintainer, I want `verify` to prove static assets and the deep-link
    fallback are served, so that the serving model is exercised and not
    assumed.
38. As a maintainer, I want CI to run the Web Target checks under both
    supported package managers, so that neither installation path silently
    breaks.
39. As a maintainer, I want the dependency audit to keep covering the whole
    Rust workspace after it moves, so that the security check does not quietly
    stop running.
40. As a developer, I want drift detection over the Generated Contract to keep
    working after the restructure, so that the template's core guarantee
    survives the change.
41. As a developer, I want the template configuration script and its own tests
    to keep passing, so that copying the template still produces a working
    application.

## Implementation Decisions

### Two Targets, two Stores (ADR-0001)

The Desktop Target is unchanged in behaviour: local-first, its Store a file in
the platform application-data directory, no network required. The Web Target's
Store belongs to the Server Process. The two hold unrelated data, and no sync,
export, or migration path between them is built. This is recorded as intended
behaviour in the README, because a reader will otherwise assume the opposite.

A thin-client desktop and a dual-mode desktop were both considered and
rejected: each would trade away the offline-by-construction property that the
Desktop Target currently has for free.

### The workspace moves to the repository root (ADR-0002)

Migrations, generated Models, App Schema composition, and Store access move out
of the Tauri crate into a crate that depends on no Tauri code. Both Targets
depend on it. The reusable TauRPC transport crate moves alongside it, keeping
its name. A new crate holds the Server Process. The Tauri crate is reduced to
composition and its binary entry point.

The Cargo workspace root moves from the Tauri directory to the repository root.
Crate names are drawn from the glossary so that the tree reads as the
vocabulary: a schema crate for the domain, the existing transport crate, and a
web server crate for the Server Process.

This is a single indivisible change. Manifest paths in every script, the
generation and drift-detection scripts, the pre-development hook, and the CI
dependency-audit working directory all move together.

### The Server Process contract (ADR-0004)

The Server Process serves one origin:

| Route | Purpose |
| --- | --- |
| `GET /` | the Web Target bundle |
| any unmatched path | the Web Target bundle, for client-side routing |
| `POST /graphql` | queries and mutations |
| `WS /graphql/ws` | subscriptions, graphql-ws protocol |

It is configured by command-line flags, not by baked-in values:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--bind` | loopback, fixed port | listen address |
| `--store` | a path in the working directory | the Store |
| `--web-root` | a build output directory | the assets to serve |

Because the UI and the endpoints share an origin, the client uses a relative
URL. There is no CORS configuration and no API base URL — both would be
unproven configuration surfaces in a template that admits only mechanisms
backed by executable checks.

Assets are read from a directory at runtime rather than embedded in the binary.
Embedding would produce a single file to deploy but would make the Rust build
depend on the bundler having run, and would introduce a stale-bundle failure
mode that nothing detects. Reading from a directory keeps the two build graphs
independent and makes that failure mode impossible.

No authentication or authorization ships. Authorization stays outside the
Stability Boundary and requires its own proof before the Web Target may host
real users.

### Transport selection at build time (ADR-0003)

There is one UI. Its Apollo link is chosen by a build-time flag, producing two
bundles. The Desktop Target bundle imports the TauRPC Transport and the
generated TauRPC bindings; the Web Target bundle imports the HTTP and
graphql-ws Transport. Neither contains the other's code, and dead-code
elimination is what enforces this rather than convention.

Runtime detection of the Tauri IPC global was rejected: it produces one
artifact, but it ships desktop IPC code to browsers and a network Transport
into the desktop application, so a defect in either Transport reaches both
Targets.

Client-side module organisation follows the same split: one module per
Transport, and one module that selects between them. Everything downstream —
Caller Operations, generated documents, the Apollo cache and its Cache
Convergence rules — is shared unchanged.

### Subscriptions use the standard protocol

Subscriptions travel over graphql-ws, the standard protocol, using the
async-graphql server integration and the corresponding Apollo link. No custom
envelope is invented for the network path; the bespoke accept/next/complete
envelope stays where it belongs, on the Tauri channel Transport.

The App Schema still declares no Subscription root, because the Seaography
release in use emits invalid SDL for an empty one. The graphql-ws path is
therefore proven at the Transport level against a stub schema — exactly as the
existing TauRPC subscription proof works — rather than against an application
subscription field. A real application subscription field remains an
unproven extension point.

### Script vocabulary

The script that builds the shared asset bundle is renamed to say that it builds
the UI assets. The name it currently occupies is freed to mean what a reader
expects: building the Web Target. The desktop development and build scripts are
unchanged. A development script runs the bundler and the Server Process
together, with the bundler proxying GraphQL and WebSocket traffic to the
Server Process so that development and deployment share one origin model.

### Both Targets ship, both are verified

The Web Target is a first-class part of the template rather than an optional
add-on or a documented recipe. Every copied application receives the Server
Process crate and both build paths, and `verify` proves both. An application
that only ever ships desktop carries a server crate it does not run; that cost
is accepted so that the browser path cannot silently rot.

## Testing Decisions

### What makes a good test here

A good test in this repository drives the highest available seam and asserts
only externally observable behaviour: what crosses a socket, what a running
process writes to its Store, what a build produces. It never reaches into
handler functions, module internals, or intermediate representations, because
those are free to change. Every test supplies its own Store location and its
own schema, so no test can read or destroy real data.

### Seams

Two seams, one of them new.

**The composed Server Process (new).** The Server Process exposes a start
function that takes an App Schema, a bind address, a Store path, and an asset
root, and returns a running server. This mirrors the two seams the repository
already uses: the Tauri composition test seam that takes an explicit Store
path, and the GraphQL endpoint that takes an explicit schema. Tests bind an
ephemeral port on loopback, use a temporary Store and a temporary asset
directory, and drive the server as an external client would.

Through this one seam:

- a query and a mutation are sent over real HTTP against the real App Schema,
  and the mutation's effect is confirmed to have reached the temporary Store;
- a graphql-ws client completes initialisation, subscribes, receives at least
  one event, completes, and the server-side resources are confirmed released —
  run against a stub schema carrying a subscription field, since the App Schema
  has none;
- a request for the root path and a request for an arbitrary unmatched path
  both return the asset bundle's entry document;
- the default bind address is confirmed to be loopback.

**The verify script (existing layer).** One assertion inspects the built Web
Target bundle and fails if it contains desktop IPC or TauRPC code. This cannot
be observed at runtime — it is a property of a build artifact — so it belongs
in the build pipeline rather than in a test binary.

### Rejected seams

Per-handler tests of individual routes, and tests of the HTTP Apollo link
against a mocked fetch, were both rejected. They sit below the composed-server
seam, they assert implementation structure rather than behaviour, and they
duplicate coverage that the single composed seam already provides. A
browser-driven end-to-end seam was also rejected for this ticket: no
browser-driving toolchain exists in the repository, and adding one brings CI
dependencies and flakiness disproportionate to what it would prove beyond the
composed-server seam.

### Prior art

The composed Server Process tests follow the Tauri composition test, which
builds the whole application against a temporary Store and asserts the Store
was created. The graphql-ws test follows the existing transport test, which
registers a stub schema with a subscription field, drives a real subscription
to a first event, and then asserts that teardown leaves no stale registry
entry. Both patterns carry over directly.

### Continuous integration

CI gains Web Target coverage on both package-manager paths. The Rust dependency
audit's working directory is corrected for the new workspace root. Existing
jobs — generated-contract drift, TypeScript checks, the multi-platform Rust
matrix, and the desktop build — keep running against the moved paths.

## Out of Scope

- **Authentication and authorization.** Deliberately excluded. The Web Target
  ships unauthenticated and loopback-bound. Hosting real users over the
  internet is blocked on a separate ticket that must supply its own proof,
  including per-user scoping over generated CRUD, whose filters are currently
  unscoped.
- **Any sharing of data between Targets.** No sync, no export, no import, no
  migration between the Desktop Store and the Server Store.
- **An application subscription field in the App Schema.** The Subscription
  root is still absent by design. Only the Transport is proven.
- **Public deployment concerns.** TLS, reverse proxies, container images,
  process supervision, horizontal scaling, and connection pooling for
  concurrent users.
- **Multi-user behaviour.** Per-user data, presence, conflict handling, and
  anything that follows from more than one person using one Store.
- **Making the Web Target optional.** Both Targets ship in every copied
  application; no strip-out flag is added to the configuration script.
- **Embedding assets in the Server Process binary.**
- **CORS and configurable endpoint URLs.** The same-origin design removes the
  need; adding them would add unproven configuration surface.
- **Browser-driven end-to-end testing.**
- **Changing the Desktop Target's behaviour** in any way beyond the crate
  reorganisation.

## Further Notes

The vocabulary used throughout this spec is recorded in the project glossary,
which gained the Target, Desktop Target, Web Target, Transport, App Schema,
Store, and Server Process terms during the design session for this ticket.

Four decisions from that session are recorded as ADRs: separate Stores per
Target; the workspace root move and the Tauri-free domain crate; build-time
Transport selection; and the unauthenticated same-origin Server Process.

Two tensions are worth carrying forward rather than forgetting:

The stated goal is a hosted web application for real users, but what this
ticket delivers is loopback-bound and unauthenticated. That is deliberate. The
mechanism lands here; the hosted-for-real-users milestone is gated on the
authorization ticket. The spec should not be read as claiming the Web Target is
production-ready.

The workspace restructure is the riskiest part of the change and is unrelated
to the Web Target in user-visible terms. It touches every script, every
manifest path, and CI. It cannot be split from the rest, because the Server
Process cannot exist without it, but it is where regressions are most likely
and where review attention is best spent.

Documentation to update alongside the code: the README's description of what
the template produces and its explicit statement that Targets do not share
data; the architecture document's claim that no HTTP server or open port
exists, which this ticket makes false for the Web Target; and the Stability
Boundary document, which gains the Web Target's proven mechanisms and keeps
authorization on the unproven side.
