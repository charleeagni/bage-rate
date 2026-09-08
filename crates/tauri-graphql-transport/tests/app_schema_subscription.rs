//! The Desktop Target half of the subscription proof: a Module's own
//! subscription field, over the real App Schema, over the real Transport.
//!
//! `transport.rs` proves the Transport's lifecycle against a stub whose
//! stream it controls. This proves the other half — that an event a Module
//! publishes from inside one of its operations reaches a Desktop subscriber
//! — so neither test has to stand in for the other.

use std::time::Duration;

use app_schema::{database::in_memory_database, schema::app_schema};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri_graphql_transport::{GraphQlEndpoint, TransportApi, TransportApiImpl};

async fn installed_api() -> TransportApiImpl {
    let store = in_memory_database()
        .await
        .expect("migrate a temporary Store");
    let api = TransportApiImpl::new();
    api.install_endpoint(GraphQlEndpoint::new(
        app_schema(store).expect("compose the App Schema"),
    ))
    .expect("install the endpoint");
    api
}

async fn execute(api: &TransportApiImpl, query: &str) -> serde_json::Value {
    let response: serde_json::Value = serde_json::from_str(
        &api.clone()
            .graphql_execute(serde_json::json!({ "query": query }).to_string())
            .await,
    )
    .expect("decode the response envelope");
    assert!(
        response.get("errors").is_none(),
        "GraphQL errors: {response:#}"
    );
    response
}

fn capture_channel() -> (
    Channel<String>,
    tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let channel = Channel::new(move |message: InvokeResponseBody| {
        let InvokeResponseBody::Json(encoded) = message else {
            panic!("subscription events must cross as JSON");
        };
        let event: String = serde_json::from_str(&encoded).expect("decode the channel payload");
        sender.send(event).expect("record the subscription event");
        Ok(())
    });
    (channel, receiver)
}

#[tokio::test]
async fn a_module_subscription_delivers_over_the_desktop_transport() {
    let api = installed_api().await;

    execute(
        &api,
        r#"mutation {
             documentsCreateOne(data: {
               id: "d1", taskId: "t", scope: "design",
               rootDir: "/workspace", relPath: "docs/design.md",
               createdAt: "2026-08-24T00:00:00Z", updatedAt: "2026-08-24T00:00:00Z"
             }) { id }
           }"#,
    )
    .await;

    let (channel, mut events) = capture_channel();
    assert_eq!(
        api.clone()
            .graphql_subscribe(
                "documents-saved".to_owned(),
                serde_json::json!({
                    "query": "subscription Saved { documentsSaved { documentId digest } }",
                    "operationName": "Saved",
                    "variables": null
                })
                .to_string(),
                channel,
            )
            .await,
        r#"{"type":"accepted"}"#
    );

    let saved = execute(
        &api,
        r#"mutation {
             documentsSave(documentId: "d1", expectedDigest: "",
                           digest: "digest-1", savedAt: "2026-08-24T01:00:00Z")
             { saved stale }
           }"#,
    )
    .await;
    assert_eq!(saved["data"]["documentsSave"]["saved"], true);

    let encoded = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .expect("the event crosses the Tauri channel within five seconds")
        .expect("the subscription channel stays open");
    let event: serde_json::Value =
        serde_json::from_str(&encoded).expect("decode the event envelope");
    assert_eq!(
        event["payload"]["data"]["documentsSaved"]["documentId"],
        "d1"
    );
    assert_eq!(
        event["payload"]["data"]["documentsSaved"]["digest"],
        "digest-1"
    );

    assert!(api.graphql_unsubscribe("documents-saved".to_owned()).await);
}
