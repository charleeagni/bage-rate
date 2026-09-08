# Add a custom query

Use a custom query when the generated connection cannot return the answer you
need. This guide follows the existing `documentsSaveCheck` query in
`modules/documents/`.

## Check whether you need one

Do not add a custom query for either of these cases:

- To fetch a different set of rows, use `filters`, `orderBy`, and `pagination`
  on the generated `<module>` field.
- To calculate a value from one row, use `CustomOps::computed_field`. See
  `neverSaved` in `modules/documents/rust/src/custom.rs`.

Add a custom query when the result is not a set of rows. Typical results are a
decision, summary, or comparison that the generated filters cannot express.

The example checks whether `documentsSave` would succeed. Keeping that logic
on the server matters. An absent digest, an unknown document, and a different
digest are separate cases. If each caller implemented those rules, the check
could drift away from the mutation that performs the save.

## 1. Define the result

Create one Rust file for the operation. `#[output]` turns a plain struct into a
GraphQL object.

```rust
// modules/documents/rust/src/check.rs
use module_host::{custom_fields, output, store::*, ModuleCtx, Result};

use crate::entities::documents;

/// What a save holding `digest` would do, without doing it.
#[output]
pub struct DocumentsSaveCheck {
    pub document_id: String,
    /// Null while the document has never been saved.
    pub held: Option<String>,
    pub known: bool,
    pub up_to_date: bool,
}
```

The struct name becomes the GraphQL type name. It must start with the Module
name. Rust field names become camel case, so `up_to_date` becomes `upToDate`.

## 2. Write the resolver

```rust
pub struct DocumentsSaveQueries;

#[custom_fields]
impl DocumentsSaveQueries {
    /// Answer whether `documentsSave` would write or report a conflict.
    async fn documents_save_check(
        ctx: &ModuleCtx<'_>,
        document_id: String,
        digest: String,
    ) -> Result<DocumentsSaveCheck> {
        // This check needs one read. The save repeats it in its transaction.
        let document = documents::Entity::find_by_id(document_id.clone())
            .one(ctx.store()?)
            .await?;

        let known = document.is_some();
        let held = document.and_then(|document| document.content_digest);
        Ok(DocumentsSaveCheck {
            up_to_date: known && held.as_deref().unwrap_or_default() == digest,
            document_id,
            held,
            known,
        })
    }
}
```

Each function in a `#[custom_fields]` block becomes one root field.
`documents_save_check` becomes `documentsSaveCheck`, and its arguments also
become camel case.

Registration decides whether the functions belong to `Query` or `Mutation`.
The implementation block itself does not.

The resolver receives `ModuleCtx`, not the raw GraphQL context. Use it to get
the Store, start a transaction, or publish an event. If a Module needs another
capability, add it to `ModuleCtx`. Do not import the implementation directly.

## 3. Register it

Register the output and query in `custom.rs`:

```rust
// modules/documents/rust/src/custom.rs
    ops.output::<DocumentsSaveCheck>();
    ops.query::<DocumentsSaveQueries>();
```

One output type may be shared by several operations. Also add `mod check;` to
the Module's `lib.rs`.

## 4. Regenerate the schema

```console
$ npm run generate
```

The generated SDL contains the new field and type:

```graphql
type Query {
	documents(filters: DocumentsFilterInput, …): DocumentsConnection!
	documentsSaveCheck(documentId: String!, digest: String!): DocumentsSaveCheck!
}

type DocumentsSaveCheck {
	documentId: String!
	held: String
	known: Boolean!
	upToDate: Boolean!
}
```

Review this diff as public API. `Option<String>` produces nullable `String`.
The other fields are non-null.

## 5. Add the caller operation

```graphql
# modules/documents/ui/operations/documents.graphql
# This result is advisory. The save checks again inside its transaction.
query CheckDocumentSave($documentId: String!, $digest: String!) {
  documentsSaveCheck(documentId: $documentId, digest: $digest) {
    documentId
    held
    known
    upToDate
  }
}
```

Code generation checks this document against the documents mini-schema and
creates `CheckDocumentSaveDocument` in `ui/src/generated/graphql.ts`. Pass
that document to Apollo. Do not add mirror interfaces or manual generics.

## 6. Define cache behavior

This query returns a result object, not a Model. Tell Apollo not to normalize
it as an entity:

```ts
// modules/documents/ui/src/cache.ts
  DocumentsSaveCheck: { keyFields: false },
```

This particular answer can become stale immediately, so bypass the cache:

```tsx
// modules/documents/ui/src/DocumentsPanel.tsx
  const [checkSave, checkState] = useLazyQuery(CheckDocumentSaveDocument, {
    fetchPolicy: "network-only",
  });
```

A query about current data may use the cache. A query that predicts a later
write usually should not because the data can change between the check and the
write. The mutation must still enforce the rule in its own transaction.

## 7. Test the GraphQL contract

```rust
// crates/module-host/tests/primitives.rs
#[tokio::test]
async fn a_custom_query_answers_from_the_store() {
    let schema = documents_schema().await;
    assert!(schema
        .sdl()
        .contains("documentsSaveCheck(documentId: String!, digest: String!): DocumentsSaveCheck!"));

    run(&schema, &register("a", "docs/design.md")).await;
    // Never saved: held is null and upToDate is false.
    run(&schema, &save("a", "", "digest-1")).await;
    // Now held is "digest-1" and upToDate is true.

    let unknown = run(&schema, &check("b", "digest-1")).await;
    // An unknown document has known set to false.
}
```

Test through the composed schema. Calling the resolver directly would miss
GraphQL names, nullability, registration, and the SDL seen by callers.

## Reference

### `#[output]`

| Rule | Reason |
| --- | --- |
| Use named fields | Each field becomes a GraphQL field. |
| Do not use generics | GraphQL needs one unambiguous type name. |
| Use supported scalar fields such as `String`, `bool`, `i32`, and `Option<T>` | GraphQL reads each value directly from the struct. |
| Prefix the type name with the Module name | Schema composition checks the prefix. |
| Derive `Clone` for event types | Subscriptions need one copy per subscriber. Queries do not. |

### `#[custom_fields]`

The macro reports these errors at the offending code:

| Rule | Diagnostic |
| --- | --- |
| Use an inherent impl | `#[custom_fields] goes on an inherent impl block, not a trait impl` |
| Put only resolver functions in the block | `a #[custom_fields] impl block holds only resolver functions` |
| Make every resolver async | `a resolver is async fn; it runs inside the request` |
| Put `ctx: &ModuleCtx<'_>` first | `a resolver's first argument is ctx: &ModuleCtx<'_>; the only Store access a Module has` |
| Do not take `self` | `a resolver is an associated function, so it takes no self` |
| Return `module_host::Result<T>` | `a resolver returns module_host::Result<T>, so its output type is declared` |

### Type mapping

SeaORM scalar types map to their GraphQL scalars. `Option<T>` makes a field or
argument nullable. For example, `Option<String>` becomes `String`, while
`String` becomes `String!`.

### Read from the Store

| Need | Use |
| --- | --- |
| One read or several independent reads | `ctx.store()?` |
| Reads that need one consistent snapshot | `ctx.transaction(…)` |
| A failure the caller cannot resolve | `Err(Error::new(…))` |
| A result the caller must inspect and act on | `Ok(…)` with a field for the result |

Use errors for failures the caller cannot resolve. Return expected outcomes as
data. [Mutations](mutation.md#return-a-conflict-as-data) explains this rule in
more detail.

### Common failures

| Symptom | Fix |
| --- | --- |
| `module "documents" registered Query field "saveCheck", which does not carry the module's name as its prefix` | Rename the function to `documents_save_check`. |
| `module "documents" registered type "SaveCheck", …` | Prefix the output type with `Documents`. |
| `names "sea_orm", which lives below the seam; reach it through module_host` | Import from `module_host::store::*`. |
| `imports from "seaography"` | Import the allowed re-export from `module_host`. |
| Codegen says `Cannot query field "documentsSaveCheck" on type "Query"` | Run generation after registration, and keep the operation in the owning Module. |
| Verify says `ERROR: the committed GraphQL SDL is stale.` | Run `npm run generate`. |
