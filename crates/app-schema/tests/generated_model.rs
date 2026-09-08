use app_schema::{database::in_memory_database, schema::app_schema};
use seaography::async_graphql::{dynamic::Schema, Response};
use serde_json::{json, Value};

fn data(response: Response) -> Value {
    assert!(
        response.errors.is_empty(),
        "GraphQL errors: {:#?}",
        response.errors
    );
    response.data.into_json().expect("response data is JSON")
}

async fn execute(schema: &Schema, operation: &str) -> Value {
    data(schema.execute(operation).await)
}

#[tokio::test]
async fn generated_crud_orders_pages_updates_and_deletes() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");

    let created = execute(
        &schema,
        r#"
        mutation {
          projectsCreateBatch(data: [
            { name: "Gamma", workspaceRoot: "/tmp/gamma" }
            { name: "Alpha", workspaceRoot: "/tmp/alpha" }
            { name: "Beta", workspaceRoot: "/tmp/beta" }
          ]) { id name workspaceRoot }
        }
        "#,
    )
    .await;
    assert_eq!(created["projectsCreateBatch"].as_array().unwrap().len(), 3);

    let page = execute(
        &schema,
        r#"
        query {
          projects(
            orderBy: { name: ASC }
            pagination: { page: { limit: 2, page: 0 } }
          ) {
            nodes { id name workspaceRoot }
            paginationInfo { pages current total }
          }
        }
        "#,
    )
    .await;
    assert_eq!(
        page,
        json!({
            "projects": {
                "nodes": [
                    { "id": 2, "name": "Alpha", "workspaceRoot": "/tmp/alpha" },
                    { "id": 3, "name": "Beta", "workspaceRoot": "/tmp/beta" }
                ],
                "paginationInfo": { "pages": 2, "current": 0, "total": 3 }
            }
        })
    );

    let updated = execute(
        &schema,
        r#"mutation { projectsUpdate(data: { name: "Alpha renamed" }, filter: { id: { eq: 2 } }) { id name } }"#,
    )
    .await;
    assert_eq!(
        updated,
        json!({ "projectsUpdate": [{ "id": 2, "name": "Alpha renamed" }] })
    );

    let deleted = execute(
        &schema,
        r#"mutation { projectsDelete(filter: { id: { eq: 2 } }) }"#,
    )
    .await;
    assert_eq!(deleted, json!({ "projectsDelete": 1 }));
}

#[tokio::test]
async fn generated_writes_preserve_constraints_and_batch_atomicity() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");

    let failed = schema
        .execute(
            r#"
            mutation {
              projectsCreateBatch(data: [
                { name: "First", workspaceRoot: "/tmp/shared" }
                { name: "Second", workspaceRoot: "/tmp/shared" }
              ]) { id }
            }
            "#,
        )
        .await;
    assert_eq!(failed.errors.len(), 1);
    assert_eq!(
        execute(&schema, "query { projects { nodes { id } } }").await,
        json!({ "projects": { "nodes": [] } })
    );
}

#[tokio::test]
async fn schema_is_valid_and_unifies_query_and_mutation_entity_types() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");
    let sdl = schema.sdl();

    assert!(!sdl.contains("ProjectsBasic"));
    assert!(sdl.contains("projectsCreateOne(data: ProjectsInsertInput!): Projects!"));
    assert!(sdl.contains("nodes: [Projects!]!"));
    assert_eq!(sdl.matches("type Projects {").count(), 1);
    assert!(!sdl.contains("_ping"));
    assert!(!sdl.contains("schema {\n  subscription:"));
}
