# Security notes

## Reporting a vulnerability

Do not file a public issue for a suspected vulnerability. Use [GitHub private vulnerability reporting](https://github.com/charleeagni/bage-rate/security/advisories/new)
and include a minimal reproduction and the affected version.

## Trust boundaries

The desktop app uses Tauri IPC. Its bundled pet documents share the app origin
and must be treated as trusted application code.

Lock-in animations supplied through Settings are different: the app stores the
HTML locally and serves it through a separate custom protocol. They run with
scripts enabled and an opaque sandbox origin that prevents access to the app
origin. The protocol's Content Security Policy blocks network connections,
forms, and frames. Import only HTML you
are comfortable executing within that sandbox.

`web-server` is an unauthenticated local-development executable. It binds to
loopback by default. Do not expose it to a LAN or the public internet; a
reachable client can use its GraphQL API and change the server store.
