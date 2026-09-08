# Add or restrict a mutation

Generated CRUD handles most writes. This guide covers the rest:

- check generated writes with a hook
- remove generated writes from the schema
- add a custom mutation

The examples come from `modules/documents/`.

## Choose the smallest change

| Requirement | Use |
| --- | --- |
| Reject invalid rows | [Write hook](#guard-generated-writes) |
| Set a field on every write | Write hook that changes `proposed` |
| Prevent callers from using a generated operation | [`CustomOps::writes`](#remove-a-generated-write) |
| Make a conditional or atomic write, or return a custom result | [Custom mutation](#add-a-custom-mutation) |

The documents Module uses all three. It removes the generated update, checks
the generated create and delete, and provides a compare-and-swap save.

```rust
// modules/documents/rust/src/custom.rs
pub fn register(ops: &mut CustomOps) {
    ops.writes::<documents::Entity, documents::ActiveModel>(Writes::HOOKED.without(Write::Update));
    ops.write_hook::<documents::Entity, _>(RelativePathRule);

    ops.output::<DocumentsSaveOutcome>();
    ops.mutation::<DocumentsSaveMutations>();
    // …
}
```

## Guard generated writes

A write hook runs inside the generated mutation's transaction. It receives
the complete set of rows after the proposed values have been applied, but
before the database write.

```rust
// modules/documents/rust/src/rules.rs
use module_host::{Decision, Txn, WriteAction, WriteHook, WriteSet};

use crate::entities::documents;

pub struct RelativePathRule;

impl WriteHook<documents::Entity> for RelativePathRule {
    fn check(
        &self,
        write: &mut WriteSet<'_, '_, documents::Entity>,
        _transaction: &Txn,
    ) -> Decision {
        if write.action() == WriteAction::Delete {
            return Decision::Allow;
        }
        for index in 0..write.len() {
            let Some(proposed) = write.proposed(index) else {
                continue;
            };
            let module_host::store::ActiveValue::Set(path) = &proposed.rel_path else {
                continue;
            };
            if let Some(reason) = rejection(path) {
                return Decision::reject(reason);
            }
        }
        Decision::Allow
    }
}
```

Register the hook next to the write selection:

```rust
    ops.write_hook::<documents::Entity, _>(RelativePathRule);
```

A hook can do three useful things:

- Compare `current(index)` with `proposed(index)` to validate a transition.
  `current` is `None` during create.
- Change `proposed(index)` to set a value before it reaches the database.
- Reject the entire mutation with `Decision::reject`.

Hooks run only for writes registered as hooked. They do not run for standard
Seaography writes. Registering a hook while leaving the Model's writes as
generated does nothing, so keep the hook and write-selection calls together.

A rejection rolls back the mutation and returns its reason to the caller.
Write a message that tells the caller what to fix:

```rust
Decision::reject(format!(
    "relPath \"{path}\" climbs out of its rootDir"
))
```

## Remove a generated write

Use `CustomOps::writes` to choose each generated operation and whether its
hooks run:

```rust
    ops.writes::<documents::Entity, documents::ActiveModel>(Writes::HOOKED.without(Write::Update));
```

For this Model, the line above:

- keeps create-one and delete with hooks
- disables create-batch because no hooked batch-create implementation exists
- removes update so callers must use `documentsSave`

After `npm run generate`, the SDL no longer contains `documentsUpdate`,
`documentsCreateBatch`, or `DocumentsUpdateInput`. Review the SDL diff and
test both what disappeared and what stayed.

Once you call `ops.writes` for any Model, you must call it for every Model in
that Module. A Model without a call publishes no writes.

### Write-selection options

| Option | Result |
| --- | --- |
| `Writes::GENERATED` | Publish all four Seaography writes without hooks. This is the default when the Module makes no selection. |
| `Writes::HOOKED` | Publish create-one, update, and delete with hooks. Disable create-batch. |
| `Writes::NONE` | Publish no generated writes. |
| `.without(Write::Update)` | Remove update. |
| `.hooked(Write::Delete)` | Publish delete with hooks. The SDL is unchanged. |
| `.generated(Write::CreateBatch)` | Publish Seaography's batch create without hooks. |

The `Write` variants are `CreateOne`, `CreateBatch`, `Update`, and `Delete`.
Start from a `Writes` constant and narrow it.

## Add a custom mutation

Add one when generated CRUD cannot express the write itself. The example is a
compare-and-swap. It saves a digest only when the document still has the
caller's expected digest.

The generated update cannot distinguish a stale digest from a missing
document. The custom mutation can, and it returns the stale digest as data.

### 1. Define the result

```rust
// modules/documents/rust/src/save.rs
/// What a save did. A stale result includes the digest currently stored.
#[output]
pub struct DocumentsSaveOutcome {
    pub document_id: String,
    pub digest: String,
    pub saved: bool,
    pub stale: bool,
}
```

Shape the result around the caller's next action. If the caller must branch,
return a field that supports that branch.

### 2. Write the resolver

```rust
#[custom_fields]
impl DocumentsSaveMutations {
    /// Write `digest` only if the document still holds `expected_digest`.
    async fn documents_save(
        ctx: &ModuleCtx<'_>,
        document_id: String,
        expected_digest: String,
        digest: String,
        saved_at: String,
    ) -> Result<DocumentsSaveOutcome> {
        let outcome = ctx
            .transaction(|transaction| {
                let document_id = document_id.clone();
                let digest = digest.clone();
                let saved_at = saved_at.clone();
                Box::pin(async move {
                    // Read the row. Return stale on a digest mismatch.
                    // Write and return saved on a match.
                })
            })
            .await?;

        if outcome.saved {
            ctx.publish(DocumentsSavedEvent { /* … */ })?;
        }
        Ok(outcome)
    }
}
```

`ctx.transaction` commits when its closure returns `Ok` and rolls back on
`Err`. Copy the `Box::pin` and clone pattern. It lets the async closure borrow
the transaction.

Several GraphQL root mutations execute one after another. They do not share a
transaction. Put all writes that must be atomic in one custom mutation.

Publish events after `ctx.transaction` returns. Publishing inside the
transaction could notify clients about a write that later rolls back. See the
[subscription guide](subscription.md).

### Return a conflict as data

Use `Err` when the caller cannot resolve the failure. Return expected outcomes
through `Ok`.

A stale digest is an expected conflict. The caller can fetch the new version,
merge, and retry. Return `stale: true` and the stored digest so the UI can act
without parsing an error string.

```tsx
// modules/documents/ui/src/DocumentsPanel.tsx
    setConflict(
      result.data?.documentsSave.stale
        ? `${id} moved on; it now holds ${result.data.documentsSave.digest}`
        : null,
    );
```

A missing document is an error because retrying the same operation cannot fix
it.

Returning `Err` for a conflict rolls back the transaction but discards the
result the caller needs. Compute the conflict inside the transaction and
return it through `Ok`. An outcome with no writes is safe to commit.

### 3. Register it

```rust
    ops.output::<DocumentsSaveOutcome>();
    ops.mutation::<DocumentsSaveMutations>();
```

Registration places the functions on the Mutation root. The same
`#[custom_fields]` implementation could instead be registered as a query.

### 4. Regenerate

Run `npm run generate`. Review everything added to and removed from
`schema.graphql`.

### 5. Add the caller operation

```graphql
# `documentsUpdate` is absent, so callers cannot skip the compare-and-swap.
mutation SaveDocument(
  $documentId: String!
  $expectedDigest: String!
  $digest: String!
  $savedAt: String!
) {
  documentsSave(
    documentId: $documentId
    expectedDigest: $expectedDigest
    digest: $digest
    savedAt: $savedAt
  ) {
    documentId
    digest
    saved
    stale
  }
}
```

For identity-scoped generated writes, put the identity variable in a literal
filter. Do not expose a filter variable. A non-null empty filter still matches
every row.

```graphql
mutation ForgetDocument($id: String!) {
  documentsDelete(filter: { id: { eq: $id } })
}
```

### 6. Update the Apollo cache

Every mutation must leave Apollo's cache consistent. A custom result is not a
Model, so Apollo cannot infer which cached entity changed.

`documentsSave` does not change list membership or ordering. Update the cached
document by identity:

```ts
// modules/documents/ui/src/cache.ts
export function applySavedDigest(
  cache: ApolloCache,
  documentId: string,
  digest: string,
  savedAt: string,
) {
  cache.modify({
    id: cache.identify({ __typename: "Documents", id: documentId }),
    fields: {
      contentDigest: () => digest,
      updatedAt: () => savedAt,
      neverSaved: () => false,
    },
  });
}
```

Choose cache behavior from the write:

| Write behavior | Cache behavior |
| --- | --- |
| Changes fields but not list membership or order | Update the entity by identity. |
| May change list membership or order | Refetch the affected lists, as `documentListConvergence` does. |
| Deletes a row | Evict its identity and call `gc()`, as `evictDocument` does. |
| Returns the entity and cannot change membership | No extra work. Apollo normalizes the returned entity. |

Tell Apollo that the custom result is a value, not an entity:

```ts
  DocumentsSaveOutcome: { keyFields: false },
```

Without that policy, Apollo may merge separate outcomes into one cached
object.

### 7. Test the write and its effects

| Test in `crates/module-host/tests/primitives.rs` | What it proves |
| --- | --- |
| `the_compare_and_swap_writes_when_the_expected_digest_still_holds` | Two successful saves write their digests. |
| `a_conflicting_save_comes_back_as_data_and_writes_nothing` | The caller receives the conflict and the losing write does not land. |
| `a_write_hook_rejects_the_write_and_leaves_the_store_untouched` | The rejection reaches the caller, the Store stays unchanged, and valid input still works. |
| `dropping_a_generated_mutation_removes_exactly_that_field` | The intended fields disappear and the other fields remain. |
| `a_conflicting_save_publishes_nothing` | A write that did not happen emits no event. |

Always include a valid-input case for a hook. A hook that rejects every input
would otherwise pass a rejection-only test.

## Reference

### `WriteSet`

| Method | Returns |
| --- | --- |
| `action()` | `WriteAction::{Create, Update, Delete}` |
| `len()`, `is_empty()` | Number of rows in this mutation |
| `current(index)` | Stored row, or `None` during create |
| `proposed(index)` | Row as it will be persisted. Change it to amend the write. |

`check` also receives `&Txn`. Use it when the rule needs another read in the
same transaction.

### `ModuleCtx`

| Method | Use |
| --- | --- |
| `ctx.store()?` | Reads and single writes |
| `ctx.transaction(\|txn\| Box::pin(async move { … }))` | Commit on `Ok`, roll back on `Err` |
| `ctx.publish(event)?` | Send an event to each active subscription for its type |

### Common failures

| Symptom | Fix |
| --- | --- |
| A hook never runs | Register that Model's writes as hooked. |
| A generated mutation is unexpectedly missing | Add `ops.writes` for every Model in the Module. |
| Verify says `ERROR: the committed GraphQL SDL is stale.` | Run `npm run generate`. |
| Apollo warns about data loss while replacing a field | Set `keyFields: false` on the custom result type. |
| A list stays stale after create | Refetch lists whose membership or ordering may have changed. |
