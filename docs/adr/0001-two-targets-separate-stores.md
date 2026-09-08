---
status: accepted
---

# The Desktop and Web Targets keep separate Stores

The template gains a Web Target alongside the existing Desktop Target. Both
present the same UI and execute the same App Schema, but each owns its own
Store: the Desktop Target keeps its local app-data SQLite file, and the Web
Target uses a Store owned by the Server Process. The two never exchange data.

## Considered Options

Making the Desktop Target a thin client of the Server Process would have given
one shared dataset, but it would have traded away the property the template
exists to demonstrate — that Rust owns durable state locally and the
application works with no network. Offline would have become a feature to
build rather than a property already held. A dual-mode desktop was rejected for
the same reason plus the cost of two runtime modes to test.

## Consequences

A Model created in the Desktop Target is not visible in the Web Target, and
there is no migration or sync path between them. This is intended behaviour and
must be stated in the README, because it is the first thing a reader will
assume otherwise.
