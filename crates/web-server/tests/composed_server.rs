//! Drives the composed Server Process as an external client does: a real
//! loopback socket on an ephemeral port, a temporary Store, and a temporary
//! asset root, so no run of this suite can read or destroy a real Store.

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::Path,
};

use app_schema::database::file_database;
use clap::Parser;
use projects_module::entities::prelude::Projects;
use sea_orm::EntityTrait;
use serde_json::{json, Value};
use tempfile::TempDir;
use web_server::{Arguments, RunningServer, ServerConfig, DEFAULT_BIND};

const EPHEMERAL_LOOPBACK: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
const ENTRY_DOCUMENT: &str = "<!doctype html><title>Web Target</title><div id=\"root\"></div>";

fn web_root(directory: &Path) -> &Path {
    std::fs::write(directory.join("index.html"), ENTRY_DOCUMENT).expect("write entry document");
    directory
}

async fn graphql(address: SocketAddr, query: &str) -> Value {
    let response = reqwest::Client::new()
        .post(format!("http://{address}/graphql"))
        .json(&json!({ "query": query }))
        .send()
        .await
        .expect("post GraphQL request")
        .json::<Value>()
        .await
        .expect("GraphQL response is JSON");

    assert!(
        response.get("errors").is_none(),
        "GraphQL errors: {response:#}"
    );
    response["data"].clone()
}

#[tokio::test]
async fn graphql_crud_over_loopback_reaches_the_server_store() {
    let directory = TempDir::new().expect("temporary directory");
    let store = directory.path().join("server-store.db");
    let config = ServerConfig {
        bind: EPHEMERAL_LOOPBACK,
        store: store.clone(),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server = web_server::start(&config).await.expect("start server");
    let address = server.local_addr();

    let empty = graphql(address, "query { projects { nodes { id name } } }").await;
    assert_eq!(empty, json!({ "projects": { "nodes": [] } }));

    let created = graphql(
        address,
        r#"mutation {
             projectsCreateOne(data: { name: "Alpha", workspaceRoot: "/tmp/alpha" }) {
               id name workspaceRoot
             }
           }"#,
    )
    .await;
    let id = created["projectsCreateOne"]["id"].as_i64().expect("new id");
    assert_eq!(created["projectsCreateOne"]["name"], "Alpha");

    let listed = graphql(address, "query { projects { nodes { id name } } }").await;
    assert_eq!(
        listed,
        json!({ "projects": { "nodes": [{ "id": id, "name": "Alpha" }] } })
    );

    let renamed = graphql(
        address,
        &format!(
            r#"mutation {{
                 projectsUpdate(data: {{ name: "Alpha renamed" }}, filter: {{ id: {{ eq: {id} }} }}) {{
                   id name
                 }}
               }}"#
        ),
    )
    .await;
    assert_eq!(
        renamed,
        json!({ "projectsUpdate": [{ "id": id, "name": "Alpha renamed" }] })
    );

    // The rename must be in the Server Store itself, not only in a reply.
    server.shutdown().await.expect("stop server");
    let database = file_database(&store).await.expect("open Server Store");
    let stored = Projects::find()
        .all(&database)
        .await
        .expect("read Server Store");
    assert_eq!(
        stored
            .iter()
            .map(|project| project.name.as_str())
            .collect::<Vec<_>>(),
        ["Alpha renamed"]
    );

    // Deleting is proven the same way, through a second server over the same
    // Store, so the delete has to survive the socket as well.
    let server = web_server::start(&config).await.expect("restart server");
    let deleted = graphql(
        server.local_addr(),
        &format!(r#"mutation {{ projectsDelete(filter: {{ id: {{ eq: {id} }} }}) }}"#),
    )
    .await;
    assert_eq!(deleted, json!({ "projectsDelete": 1 }));
    server.shutdown().await.expect("stop server");

    let database = file_database(&store).await.expect("reopen Server Store");
    assert!(Projects::find()
        .all(&database)
        .await
        .expect("read Server Store")
        .is_empty());
}

#[tokio::test]
async fn root_and_deep_links_return_the_web_target_entry_document() {
    let directory = TempDir::new().expect("temporary directory");
    let config = ServerConfig {
        bind: EPHEMERAL_LOOPBACK,
        store: directory.path().join("server-store.db"),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server = web_server::start(&config).await.expect("start server");
    let address = server.local_addr();

    for path in ["/", "/projects/42", "/anything/deeper/still"] {
        let body = reqwest::get(format!("http://{address}{path}"))
            .await
            .expect("request asset")
            .text()
            .await
            .expect("asset body");
        assert_eq!(body, ENTRY_DOCUMENT, "unexpected document for {path}");
    }

    server.shutdown().await.expect("stop server");
}

#[tokio::test]
async fn the_entry_document_is_served_with_the_web_target_hardening_headers() {
    // The Desktop Target constrains its document through the Tauri CSP; the Web
    // Target has to say the same thing on the wire. The expected values are
    // spelled out here rather than read from the crate constants, so relaxing a
    // constant cannot relax this test with it.
    let directory = TempDir::new().expect("temporary directory");
    let config = ServerConfig {
        bind: EPHEMERAL_LOOPBACK,
        store: directory.path().join("server-store.db"),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server = web_server::start(&config).await.expect("start server");
    let address = server.local_addr();

    let response = reqwest::get(format!("http://{address}/"))
        .await
        .expect("request entry document");
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .unwrap_or_else(|| panic!("the entry document carries no {name}"))
            .to_str()
            .expect("header is text")
            .to_owned()
    };

    assert_eq!(
        header("content-security-policy"),
        "default-src 'self'; connect-src 'self'; img-src 'self'; style-src 'self'"
    );
    assert_eq!(header("x-content-type-options"), "nosniff");
    assert_eq!(header("referrer-policy"), "no-referrer");

    // The headers belong to the composed process, not to the asset route alone.
    let graphql_response = reqwest::Client::new()
        .post(format!("http://{address}/graphql"))
        .json(&json!({ "query": "query { projects { nodes { id } } }" }))
        .send()
        .await
        .expect("post GraphQL request");
    assert_eq!(
        graphql_response
            .headers()
            .get("x-content-type-options")
            .expect("the GraphQL endpoint carries no x-content-type-options"),
        "nosniff"
    );

    server.shutdown().await.expect("stop server");
}

#[test]
fn a_command_line_without_a_bind_flag_resolves_to_loopback() {
    // Resolved the way the binary resolves it — through the clap `Arguments`
    // type — so widening the default in `main.rs` cannot leave a loopback
    // constant behind to keep this green.
    let config = Arguments::parse_from(["web-server"]).into_config();

    assert!(
        config.bind.ip().is_loopback(),
        "a Server Process started with no --bind must not expose an \
         unauthenticated Store beyond loopback, got {}",
        config.bind
    );
    assert_eq!(config.bind, DEFAULT_BIND);
    assert_eq!(config, ServerConfig::default());
}

#[test]
fn an_explicit_bind_flag_is_the_address_the_server_starts_with() {
    let config = Arguments::parse_from(["web-server", "--bind", "127.0.0.1:1500"]).into_config();

    assert_eq!(
        config.bind,
        SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), 1500)
    );
}

#[tokio::test]
async fn a_server_started_from_the_default_bind_listens_on_loopback() {
    // The resolved default carried all the way to a live socket, on an
    // ephemeral port so the suite never contends for the default port.
    let directory = TempDir::new().expect("temporary directory");
    let default_bind = Arguments::parse_from(["web-server"]).into_config().bind;
    let config = ServerConfig {
        bind: SocketAddr::new(default_bind.ip(), 0),
        store: directory.path().join("server-store.db"),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server = web_server::start(&config).await.expect("start server");

    assert!(
        server.local_addr().ip().is_loopback(),
        "the Server Process listens beyond loopback by default: {}",
        server.local_addr()
    );

    server.shutdown().await.expect("stop server");
}

#[tokio::test]
async fn a_malformed_request_is_rejected_without_stopping_the_server() {
    let directory = TempDir::new().expect("temporary directory");
    let config = ServerConfig {
        bind: EPHEMERAL_LOOPBACK,
        store: directory.path().join("server-store.db"),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server: RunningServer = web_server::start(&config).await.expect("start server");
    let address = server.local_addr();

    let response = reqwest::Client::new()
        .post(format!("http://{address}/graphql"))
        .body("not a GraphQL request")
        .send()
        .await
        .expect("post malformed request");
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    // The rejection is async-graphql's own response. Client-visible GraphQL
    // error mapping is an unproven extension point, so the Server Process must
    // not ship a bespoke `extensions` shape a copier could read as a contract.
    let rejection: serde_json::Value = response.json().await.expect("rejection body");
    assert!(
        !rejection["errors"]
            .as_array()
            .expect("errors array")
            .is_empty(),
        "the rejection carries no GraphQL errors: {rejection}"
    );
    assert!(
        rejection["errors"][0]["extensions"].is_null(),
        "the Server Process invented a client-visible error mapping: {rejection}"
    );

    let still_serving = graphql(address, "query { projects { nodes { id } } }").await;
    assert_eq!(still_serving, json!({ "projects": { "nodes": [] } }));

    server.shutdown().await.expect("stop server");
}
