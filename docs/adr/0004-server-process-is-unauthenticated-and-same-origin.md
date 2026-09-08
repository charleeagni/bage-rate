---
status: accepted
---

# The Server Process is unauthenticated and serves a single origin

The Server Process serves the Web Target bundle, the GraphQL endpoint, and the
graphql-ws endpoint from one origin, and it ships no authentication or
authorization. It binds to loopback by default and is documented as unsafe to
expose publicly. Authorization remains outside the Stability Boundary and
requires its own executable proof before the Web Target can host real users.

## Consequences

Because the UI and the endpoints share an origin, the client uses a relative
URL: there is no CORS configuration and no API base URL to configure, both of
which would otherwise be unproven surfaces in a template that admits only
mechanisms backed by executable checks.

Generated CRUD currently exposes unscoped filters, which are harmless over
in-process IPC and are not harmless over a network. `verify` asserts the
default bind address is loopback so this guarantee cannot silently regress, and
no Web Target deployment may serve real users until the authorization proof
lands.
