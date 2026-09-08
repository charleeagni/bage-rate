//! The authoring primitives, proven against a real Module.
//!
//! Every test here drives `modules/documents` — the worked example — through
//! the composed schema rather than through a stub, so what passes is the
//! contract an application would actually get.

mod support;

use std::time::Duration;

use documents_module::entities::documents;
use futures_util::StreamExt;
use module_host::compose;
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use sea_orm_migration::{prelude::async_trait, MigrationTrait, MigratorTrait};
use seaography::async_graphql::{dynamic::Schema, Request, Value};
use support::{context, database};

struct DocumentsMigrator;

#[async_trait::async_trait]
impl MigratorTrait for DocumentsMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        documents_module::module_def().migrations()
    }
}

async fn documents_store() -> DatabaseConnection {
    let store = Database::connect("sqlite::memory:")
        .await
        .expect("open the store");
    DocumentsMigrator::up(&store, None)
        .await
        .expect("run the documents migrations");
    store
}

async fn documents_schema() -> Schema {
    let store = documents_store().await;
    compose(context(), store, &[documents_module::module_def()]).expect("compose documents")
}

async fn run(schema: &Schema, query: &str) -> Value {
    let response = schema.execute(Request::new(query)).await;
    assert!(
        response.errors.is_empty(),
        "unexpected errors: {:?}",
        response.errors
    );
    response.data
}

async fn run_expecting_error(schema: &Schema, query: &str) -> String {
    let response = schema.execute(Request::new(query)).await;
    assert!(!response.errors.is_empty(), "expected an error");
    response.errors[0].message.clone()
}

fn register(id: &str, path: &str) -> String {
    format!(
        r#"mutation {{
            documentsCreateOne(data: {{
                id: "{id}", taskId: "t", scope: "design",
                rootDir: "/workspace", relPath: "{path}",
                createdAt: "2026-08-24T00:00:00Z", updatedAt: "2026-08-24T00:00:00Z"
            }}) {{ id relPath neverSaved }}
        }}"#
    )
}

fn check(id: &str, digest: &str) -> String {
    format!(
        r#"query {{
            documentsSaveCheck(documentId: "{id}", digest: "{digest}")
            {{ documentId held known upToDate }}
        }}"#
    )
}

fn save(id: &str, expected: &str, digest: &str) -> String {
    format!(
        r#"mutation {{
            documentsSave(documentId: "{id}", expectedDigest: "{expected}",
                          digest: "{digest}", savedAt: "2026-08-24T01:00:00Z")
            {{ documentId digest saved stale }}
        }}"#
    )
}

// Per-operation mutation selection. The Module drops update outright, so the
// compare-and-swap is the only write path to a document's digest — and the
// SDL says so, which is what a caller reads.
#[tokio::test]
async fn dropping_a_generated_mutation_removes_exactly_that_field() {
    let sdl = documents_schema().await.sdl();

    assert!(sdl.contains("documentsCreateOne(data: DocumentsInsertInput!): Documents!"));
    assert!(sdl.contains("documentsDelete(filter: DocumentsFilterInput): Int!"));
    assert!(sdl.contains("documentsSave(documentId: String!"));

    assert!(!sdl.contains("documentsUpdate"));
    assert!(!sdl.contains("documentsCreateBatch"));
    // The input existed only to serve the dropped mutation, so it goes too.
    assert!(!sdl.contains("DocumentsUpdateInput"));
    // Everything the remaining writes still need is untouched.
    assert!(sdl.contains("input DocumentsInsertInput"));
    assert!(sdl.contains("input DocumentsFilterInput"));
}

// A Module that selects nothing keeps the full generated bundle, so the
// selection above is a decision rather than a side effect of the mechanism.
#[tokio::test]
async fn a_module_that_selects_nothing_keeps_every_generated_write() {
    let schema = compose(
        context(),
        database().await,
        &[projects_module::module_def()],
    )
    .expect("compose projects");
    let sdl = schema.sdl();

    assert!(sdl.contains("projectsCreateOne"));
    assert!(sdl.contains("projectsCreateBatch"));
    assert!(sdl.contains("projectsUpdate"));
    assert!(sdl.contains("projectsDelete"));
}

// A custom query: one Query root field beside the generated connection,
// answering a question about the rows rather than returning them.
#[tokio::test]
async fn a_custom_query_answers_from_the_store() {
    let schema = documents_schema().await;
    assert!(schema
        .sdl()
        .contains("documentsSaveCheck(documentId: String!, digest: String!): DocumentsSaveCheck!"));

    run(&schema, &register("a", "docs/design.md")).await;

    let before = run(&schema, &check("a", "digest-1")).await;
    assert_eq!(
        before,
        serde_json::from_str::<Value>(
            r#"{"documentsSaveCheck":{"documentId":"a","held":null,"known":true,"upToDate":false}}"#
        )
        .unwrap(),
        "a document that has never been saved holds no digest, which is not the empty string"
    );

    run(&schema, &save("a", "", "digest-1")).await;
    let after = run(&schema, &check("a", "digest-1")).await;
    assert_eq!(
        after,
        serde_json::from_str::<Value>(
            r#"{"documentsSaveCheck":{"documentId":"a","held":"digest-1","known":true,"upToDate":true}}"#
        )
        .unwrap()
    );

    // A document nobody registered is a different answer from one whose
    // digest differs, and keeping the two apart is why the rule lives here
    // rather than in each caller.
    let unknown = run(&schema, &check("b", "digest-1")).await;
    assert_eq!(
        unknown,
        serde_json::from_str::<Value>(
            r#"{"documentsSaveCheck":{"documentId":"b","held":null,"known":false,"upToDate":false}}"#
        )
        .unwrap()
    );
}

#[tokio::test]
async fn the_compare_and_swap_writes_when_the_expected_digest_still_holds() {
    let schema = documents_schema().await;
    run(&schema, &register("a", "docs/design.md")).await;

    let first = run(&schema, &save("a", "", "digest-1")).await;
    assert_eq!(
        first,
        serde_json::from_str::<Value>(
            r#"{"documentsSave":{"documentId":"a","digest":"digest-1","saved":true,"stale":false}}"#
        )
        .unwrap()
    );

    let second = run(&schema, &save("a", "digest-1", "digest-2")).await;
    assert_eq!(
        second,
        serde_json::from_str::<Value>(
            r#"{"documentsSave":{"documentId":"a","digest":"digest-2","saved":true,"stale":false}}"#
        )
        .unwrap()
    );
}

#[tokio::test]
async fn a_conflicting_save_comes_back_as_data_and_writes_nothing() {
    let schema = documents_schema().await;
    run(&schema, &register("a", "docs/design.md")).await;
    run(&schema, &save("a", "", "digest-1")).await;

    // Someone else saved first, so this caller's expectation is out of date.
    let stale = run(&schema, &save("a", "digest-stale", "digest-mine")).await;
    assert_eq!(
        stale,
        serde_json::from_str::<Value>(
            r#"{"documentsSave":{"documentId":"a","digest":"digest-1","saved":false,"stale":true}}"#
        )
        .unwrap(),
        "the conflict carries the digest actually held, not an error string"
    );

    // And the losing write really did not land.
    let held = run(
        &schema,
        r#"query { documents { nodes { contentDigest neverSaved } } }"#,
    )
    .await;
    assert_eq!(
        held,
        serde_json::from_str::<Value>(
            r#"{"documents":{"nodes":[{"contentDigest":"digest-1","neverSaved":false}]}}"#
        )
        .unwrap()
    );
}

// The write hook, on a write the Module registered as hooked.
#[tokio::test]
async fn a_write_hook_rejects_the_write_and_leaves_the_store_untouched() {
    let schema = documents_schema().await;

    let message = run_expecting_error(&schema, &register("bad", "../outside.md")).await;
    assert!(
        message.contains("climbs out of its rootDir"),
        "the hook's own reason reaches the caller: {message}"
    );

    let message = run_expecting_error(&schema, &register("bad", "/etc/passwd")).await;
    assert!(message.contains("is absolute"), "{message}");

    let rows = run(&schema, r#"query { documents { nodes { id } } }"#).await;
    assert_eq!(
        rows,
        serde_json::from_str::<Value>(r#"{"documents":{"nodes":[]}}"#).unwrap(),
        "a rejected write rolls its transaction back"
    );

    // The same mutation with a path the rule accepts still works, so the hook
    // rejects a write rather than disabling one.
    run(&schema, &register("good", "docs/design.md")).await;
}

#[tokio::test]
async fn a_computed_field_reads_from_the_row() {
    let schema = documents_schema().await;
    let created = run(&schema, &register("a", "docs/design.md")).await;
    assert_eq!(
        created,
        serde_json::from_str::<Value>(
            r#"{"documentsCreateOne":{"id":"a","relPath":"docs/design.md","neverSaved":true}}"#
        )
        .unwrap()
    );

    run(&schema, &save("a", "", "digest-1")).await;
    let after = run(&schema, r#"query { documents { nodes { neverSaved } } }"#).await;
    assert_eq!(
        after,
        serde_json::from_str::<Value>(r#"{"documents":{"nodes":[{"neverSaved":false}]}}"#).unwrap()
    );
}

#[tokio::test]
async fn a_subscription_delivers_the_event_its_operation_published() {
    let schema = documents_schema().await;
    run(&schema, &register("a", "docs/design.md")).await;

    let events = schema.execute_stream(Request::new(
        r#"subscription { documentsSaved { documentId digest savedAt } }"#,
    ));
    tokio::pin!(events);

    // The subscriber only exists once the stream has been polled, so drive it
    // until it parks before publishing anything. Nothing arrives in this
    // window: a save has not happened yet.
    assert!(
        tokio::time::timeout(Duration::from_millis(100), events.next())
            .await
            .is_err(),
        "no event before the save"
    );

    run(&schema, &save("a", "", "digest-1")).await;

    let delivered = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .expect("the event arrives")
        .expect("the stream is still open");
    assert!(delivered.errors.is_empty(), "{:?}", delivered.errors);
    assert_eq!(
        delivered.data,
        serde_json::from_str::<Value>(
            r#"{"documentsSaved":{"documentId":"a","digest":"digest-1","savedAt":"2026-08-24T01:00:00Z"}}"#
        )
        .unwrap()
    );
}

#[tokio::test]
async fn a_conflicting_save_publishes_nothing() {
    let schema = documents_schema().await;
    run(&schema, &register("a", "docs/design.md")).await;
    run(&schema, &save("a", "", "digest-1")).await;

    let events = schema.execute_stream(Request::new(
        r#"subscription { documentsSaved { documentId } }"#,
    ));
    tokio::pin!(events);
    assert!(
        tokio::time::timeout(Duration::from_millis(100), events.next())
            .await
            .is_err()
    );

    run(&schema, &save("a", "digest-stale", "digest-mine")).await;

    assert!(
        tokio::time::timeout(Duration::from_millis(300), events.next())
            .await
            .is_err(),
        "a save that did not happen is not announced as one"
    );
}

// The Store is the Module's own, and a rejected write leaves it as it was —
// which is only true because the write ran inside a transaction the seam
// owns.
#[tokio::test]
async fn the_store_a_module_reaches_is_the_one_the_host_composed_against() {
    let store = documents_store().await;
    let schema = compose(context(), store.clone(), &[documents_module::module_def()])
        .expect("compose documents");
    run(&schema, &register("a", "docs/design.md")).await;
    run(&schema, &save("a", "", "digest-1")).await;

    let document = documents::Entity::find_by_id("a".to_owned())
        .one(&store)
        .await
        .expect("read the store directly")
        .expect("the document is there");
    assert_eq!(document.content_digest, Some("digest-1".to_owned()));
}
