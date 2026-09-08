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

async fn create_workspace(schema: &Schema) {
    let created = execute(
        schema,
        r#"
        mutation {
          workspacesCreateOne(data: {
            id: "workspace-1"
            canonicalRoot: "/tmp/rust"
            displayName: "rust"
            lastOpenedAt: "2026-08-24T00:00:00Z"
            createdAt: "2026-08-24T00:00:00Z"
          }) {
            id
            canonicalRoot
            displayName
            lastOpenedAt
            createdAt
          }
        }
        "#,
    )
    .await;
    assert_eq!(created["workspacesCreateOne"]["id"], "workspace-1");
}

#[tokio::test]
async fn caller_operations_create_touch_list_and_forget_a_workspace() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");
    create_workspace(&schema).await;

    let touched = execute(
        &schema,
        r#"
        mutation {
          workspacesUpdate(
            data: { lastOpenedAt: "2026-08-24T01:00:00Z" }
            filter: { id: { eq: "workspace-1" } }
          ) { id lastOpenedAt }
        }
        "#,
    )
    .await;
    assert_eq!(
        touched,
        json!({
            "workspacesUpdate": [{
                "id": "workspace-1",
                "lastOpenedAt": "2026-08-24 01:00:00 UTC"
            }]
        })
    );

    let listed = execute(
        &schema,
        "query { workspaces(orderBy: { lastOpenedAt: DESC }) { nodes { id } } }",
    )
    .await;
    assert_eq!(
        listed,
        json!({ "workspaces": { "nodes": [{ "id": "workspace-1" }] } })
    );

    let forgotten = execute(
        &schema,
        r#"mutation { workspacesDelete(filter: { id: { eq: "workspace-1" } }) }"#,
    )
    .await;
    assert_eq!(forgotten, json!({ "workspacesDelete": 1 }));
}

#[tokio::test]
async fn update_rejects_workspace_identity_and_creation_time_changes() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");
    create_workspace(&schema).await;

    let changed_id = schema
        .execute(
            r#"
            mutation {
              workspacesUpdate(
                data: { id: "workspace-2" }
                filter: { id: { eq: "workspace-1" } }
              ) { id }
            }
            "#,
        )
        .await;
    assert_eq!(changed_id.errors.len(), 1);
    assert!(changed_id.errors[0]
        .message
        .contains("a workspace update cannot change id"));

    let changed_created_at = schema
        .execute(
            r#"
            mutation {
              workspacesUpdate(
                data: { createdAt: "2026-08-25T00:00:00Z" }
                filter: { id: { eq: "workspace-1" } }
              ) { id }
            }
            "#,
        )
        .await;
    assert_eq!(changed_created_at.errors.len(), 1);
    assert!(changed_created_at.errors[0]
        .message
        .contains("a workspace update cannot change createdAt"));

    let unchanged = execute(&schema, "query { workspaces { nodes { id createdAt } } }").await;
    assert_eq!(unchanged["workspaces"]["nodes"][0]["id"], "workspace-1");
    assert_eq!(
        unchanged["workspaces"]["nodes"][0]["createdAt"],
        "2026-08-24 00:00:00 UTC"
    );
}

#[tokio::test]
async fn reviewed_workspace_write_surface_excludes_batch_create() {
    let database = in_memory_database().await.expect("migrate database");
    let schema = app_schema(database).expect("build schema");
    let sdl = schema.sdl();

    assert!(sdl.contains("workspacesCreateOne(data: WorkspacesInsertInput!): Workspaces!"));
    assert!(sdl.contains("workspacesUpdate(data: WorkspacesUpdateInput!"));
    assert!(sdl.contains("workspacesDelete(filter: WorkspacesFilterInput): Int!"));
    assert!(!sdl.contains("workspacesCreateBatch"));
}
