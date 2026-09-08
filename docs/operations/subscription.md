# Add a subscription

A subscription tells connected clients that something happened. It is not a
durable event log.

This guide follows `documentsSaved` in `modules/documents/`. The repository
tests it over both the desktop and web transports.

## Check whether you need one

Subscriptions have strict delivery limits:

- Events published with no active subscribers are dropped.
- A subscriber more than 64 events behind skips to the newest event.
- There is no replay or backlog.

Use a query when a client must learn the current state. Use a subscription to
tell an open client when it may need to run that query. A subscription also
works for live UI details where a missed event is acceptable.

## 1. Define the event

An event is an `#[output]` struct that derives `Clone`. The publisher makes one
copy for each subscriber.

```rust
// modules/documents/rust/src/save.rs
/// One document saved. The mutation publishes this after commit.
#[output]
#[derive(Clone)]
pub struct DocumentsSavedEvent {
    pub document_id: String,
    pub digest: String,
    pub saved_at: String,
}
```

Send the identity and enough data for the subscriber to decide what to do.
Do not treat the event as a copy of the database row. A client that needs the
whole row should query it.

## 2. Register the subscription

```rust
// modules/documents/rust/src/custom.rs
    ops.subscription::<DocumentsSavedEvent>("documentsSaved");
```

Unlike a query or mutation, a subscription has no resolver function from
which to derive a field name. Pass the root field name to `subscription`. It
must start with the Module name.

`subscription` also registers the output type, so do not add a separate
`ops.output::<DocumentsSavedEvent>()` call.

The event type identifies the channel. Two fields registered for the same type
receive the same events. The composed schema has a `Subscription` root only
when at least one Module registers a subscription.

## 3. Publish after commit

```rust
// modules/documents/rust/src/save.rs
        let outcome = ctx.transaction(|transaction| { /* … */ }).await?;

        if outcome.saved {
            ctx.publish(DocumentsSavedEvent {
                document_id: outcome.document_id.clone(),
                digest: outcome.digest.clone(),
                saved_at,
            })?;
        }
        Ok(outcome)
```

Publish only after `ctx.transaction` returns. Otherwise a subscriber could see
an event for a write that later rolls back.

The `outcome.saved` check matters too. A compare-and-swap conflict returns
`Ok` without changing a row, so the empty transaction commits. A committed
transaction does not necessarily mean that a write happened.

Publish from the custom operation that performs the change. A generated write
hook runs before persistence and cannot publish an after-commit event.

## 4. Regenerate the schema

```console
$ npm run generate
```

The generated SDL contains:

```graphql
type Subscription {
	documentsSaved: DocumentsSavedEvent!
}
```

## 5. Add the caller operation

```graphql
# modules/documents/ui/operations/documents.graphql
subscription OnDocumentSaved {
  documentsSaved {
    documentId
    digest
    savedAt
  }
}
```

## 6. Consume the event

```tsx
// modules/documents/ui/src/DocumentsPanel.tsx
  const saved = useSubscription(OnDocumentSavedDocument);

  // …
  {saved.data ? (
    <p aria-live="polite">
      Last save seen live: {saved.data.documentsSaved.documentId} at{" "}
      {saved.data.documentsSaved.savedAt}
    </p>
  ) : null}
```

Module UI does not choose a transport. Desktop builds use Tauri IPC and web
builds use `graphql-ws`. `virtual:target-transport` selects the Apollo link at
build time.

Tell Apollo that each event is a value, not a cached entity:

```ts
// modules/documents/ui/src/cache.ts
  DocumentsSavedEvent: { keyFields: false },
```

Then decide how the event affects the cache:

| Event contents | Action |
| --- | --- |
| Your own mutation already made this change | Do nothing. The mutation's cache update already ran. |
| The event identifies a cached row and includes the changed fields | Use `cache.modify`, as `applySavedDigest` does. |
| The event says something changed but lacks the new row data | Refetch the affected query. |
| The event concerns another part of the app | Display the notification or ignore it. Do not rebuild unrelated cached state. |

Do not reconstruct the Store from subscription events. Events can be dropped,
so a cache assembled from them can become wrong without warning. Query the
Store when correctness matters.

## 7. Test each layer

| Test | What it proves |
| --- | --- |
| `a_subscription_delivers_the_event_its_operation_published` in `crates/module-host/tests/primitives.rs` | A mutation's event reaches an open stream on the composed schema. |
| `a_conflicting_save_publishes_nothing` in the same file | A failed compare-and-swap emits no event. |
| `a_module_subscription_delivers_over_the_desktop_transport` in `crates/tauri-graphql-transport/tests/` | Tauri IPC carries the event. |
| `a_module_subscription_delivers_over_the_web_transport` in `crates/web-server/tests/subscription_lifecycle.rs` | A real `graphql-ws` client receives the event over a socket. |

The transport tests catch failures that a schema-level test cannot. The web
server also has lifecycle tests for initialize, subscribe, complete, and
disconnect during a stream.

## Reference

### `CustomOps::subscription`

```rust
ops.subscription::<T>(name)
```

| Argument | Meaning |
| --- | --- |
| `T` | An `#[output]` struct that derives `Clone` |
| `name` | Subscription root field, prefixed with the Module name |

The call registers the field, output type, and broadcast channel for `T`.

### `ModuleCtx::publish`

```rust
ctx.publish(event)?
```

| Behavior | Detail |
| --- | --- |
| Delivery | Sends to every active subscription for the event type. |
| No subscribers | Drops the event. |
| Missing registration | Returns an error if no subscription was registered for the event type. |
| Backlog | Holds 64 events. A slower subscriber skips to the newest. |

### Common failures

| Symptom | Fix |
| --- | --- |
| `module "documents" registered Subscription field "saved", which does not carry the module's name as its prefix` | Rename the field to include the Module prefix, such as `documentsSaved`. |
| `publish` returns an error | Register a subscription for that event type. |
| Subscribers see rolled-back writes | Move `publish` after `ctx.transaction`. |
| Events arrive but the UI stays stale | Connect the subscription result to the appropriate cache update or refetch. |
| One build target receives nothing | Run the desktop and web transport tests to isolate the failing transport. |
