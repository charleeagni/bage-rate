---
status: accepted
---

# Each Target selects its Transport at build time

There is one React UI. Its Apollo link is chosen by a build-time flag rather
than by detecting the runtime environment, producing two bundles: the Desktop
Target bundle imports the TauRPC Transport and the Web Target bundle imports
the HTTP and graphql-ws Transport. Neither bundle contains the other's code.

## Considered Options

Runtime detection of the Tauri IPC global would have produced a single artifact
and a simpler build, but it would ship Tauri client code to browsers and a
network Transport into the desktop application, so a defect in either Transport
would reach both Targets.

## Consequences

A bundle cannot use the Transport its Target does not support, because that
code is not present. `verify` asserts the Web Target bundle contains no Tauri
code, which is what turns this from an intention into a checked fact.
