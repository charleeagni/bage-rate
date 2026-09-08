//! Drives `WS /graphql/ws` as an external `graphql-ws` client does: a real
//! loopback socket on an ephemeral port, a temporary Store, and a temporary
//! asset root.
//!
//! The lifecycle itself is proven against a stub schema whose stream owns a
//! guard, which is how the test observes that completing a subscription
//! releases the server-side resources rather than merely silencing the
//! socket — the App Schema's own subscription ends when its Module says so,
//! and would prove nothing about teardown. A separate test drives that real
//! field end to end over the same socket.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use app_schema::{database::file_database, schema::app_schema};
use async_graphql::dynamic::{
    Field, FieldFuture, FieldValue, Object, Schema, Subscription, SubscriptionField,
    SubscriptionFieldFuture, TypeRef,
};
use futures_util::{SinkExt, StreamExt};
use sea_orm::DatabaseConnection;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
    MaybeTlsStream, WebSocketStream,
};

const EPHEMERAL_LOOPBACK: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
const PROTOCOL: &str = "graphql-transport-ws";
const ENTRY_DOCUMENT: &str = "<!doctype html><title>Web Target</title><div id=\"root\"></div>";
const PATIENCE: Duration = Duration::from_secs(5);

/// Counts the subscription streams the server currently holds open. One is
/// created per accepted `subscribe` and dropped when the server tears that
/// subscription down.
struct LiveSubscriptions(Arc<AtomicUsize>);

impl LiveSubscriptions {
    fn open(count: &Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(count))
    }
}

impl Drop for LiveSubscriptions {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A schema with a subscription field that never ends on its own, so only the
/// client's completion can release it.
fn stub_schema(live: Arc<AtomicUsize>, store: DatabaseConnection) -> Schema {
    let query = Object::new("Query").field(Field::new(
        "ping",
        TypeRef::named_nn(TypeRef::STRING),
        |_| FieldFuture::new(async { Ok(Some(FieldValue::value("pong"))) }),
    ));
    let subscription = Subscription::new("Subscription").field(SubscriptionField::new(
        "ticks",
        TypeRef::named_nn(TypeRef::INT),
        move |_| {
            let guard = LiveSubscriptions::open(&live);
            SubscriptionFieldFuture::new(async move {
                Ok(futures_util::stream::unfold(
                    (0i32, guard),
                    |(previous, guard)| async move {
                        let tick = previous + 1;
                        if previous > 0 {
                            tokio::time::sleep(Duration::from_millis(20)).await;
                        }
                        Some((Ok(FieldValue::value(tick)), (tick, guard)))
                    },
                ))
            })
        },
    ));

    Schema::build("Query", None, Some("Subscription"))
        .register(query)
        .register(subscription)
        .data(store)
        .finish()
        .expect("build the stub subscription schema")
}

/// A Store this test owns and throws away, so no run can reach a real one.
async fn temporary_store(directory: &Path) -> DatabaseConnection {
    file_database(&directory.join("subscription-store.db"))
        .await
        .expect("open a temporary Store")
}

fn web_root(directory: &Path) -> &Path {
    std::fs::write(directory.join("index.html"), ENTRY_DOCUMENT).expect("write entry document");
    directory
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Opens the socket the way a browser client does, naming the `graphql-ws`
/// subprotocol and requiring the server to agree to it.
async fn connect(address: SocketAddr) -> Socket {
    let mut request = format!("ws://{address}/graphql/ws")
        .into_client_request()
        .expect("build the subscription request");
    request
        .headers_mut()
        .insert("sec-websocket-protocol", HeaderValue::from_static(PROTOCOL));

    let (socket, response) = connect_async(request)
        .await
        .expect("open the subscription socket");
    assert_eq!(
        response
            .headers()
            .get("sec-websocket-protocol")
            .map(HeaderValue::as_bytes),
        Some(PROTOCOL.as_bytes()),
        "the server must negotiate the standard graphql-ws subprotocol"
    );
    socket
}

async fn send(socket: &mut Socket, message: Value) {
    socket
        .send(Message::text(message.to_string()))
        .await
        .expect("send a graphql-ws message");
}

/// The next protocol message, ignoring the ping/pong keep-alive traffic that
/// says nothing about the subscription's state.
async fn next_message(socket: &mut Socket) -> Value {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    loop {
        let frame = tokio::time::timeout_at(deadline, socket.next())
            .await
            .expect("receive a graphql-ws message within five seconds")
            .expect("the subscription socket stays open")
            .expect("read the subscription socket");
        match frame {
            Message::Text(text) => {
                let message: Value =
                    serde_json::from_str(&text).expect("decode a graphql-ws message");
                if message["type"] == "ping" {
                    send(socket, json!({ "type": "pong" })).await;
                    continue;
                }
                return message;
            }
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("unexpected graphql-ws frame: {other:?}"),
        }
    }
}

async fn initialize(socket: &mut Socket) {
    send(socket, json!({ "type": "connection_init", "payload": {} })).await;
    let acknowledgement = next_message(socket).await;
    assert_eq!(acknowledgement["type"], "connection_ack");
}

/// Subscribes under `id` and waits for its first event.
async fn subscribe_to_first_tick(socket: &mut Socket, id: &str) {
    send(
        socket,
        json!({
            "id": id,
            "type": "subscribe",
            "payload": { "query": "subscription Ticks { ticks }" }
        }),
    )
    .await;

    let event = next_message(socket).await;
    assert_eq!(event["type"], "next", "unexpected message: {event:#}");
    assert_eq!(event["id"], id);
    assert_eq!(event["payload"]["data"]["ticks"], 1);
}

/// Completes `id` and waits for the server to confirm it, skipping any event
/// that was already in flight when completion was sent.
async fn complete_subscription(socket: &mut Socket, id: &str) {
    send(socket, json!({ "id": id, "type": "complete" })).await;
    loop {
        let message = next_message(socket).await;
        if message["type"] == "next" {
            continue;
        }
        assert_eq!(
            message["type"], "complete",
            "unexpected message after completing: {message:#}"
        );
        assert_eq!(message["id"], id);
        return;
    }
}

async fn wait_for_live_subscriptions(live: &AtomicUsize, expected: usize) {
    let deadline = tokio::time::Instant::now() + PATIENCE;
    while live.load(Ordering::SeqCst) != expected {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the server still holds {} subscription(s), expected {expected}",
            live.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn a_graphql_ws_client_initializes_subscribes_completes_and_disconnects() {
    let directory = TempDir::new().expect("temporary directory");
    let store = temporary_store(directory.path()).await;
    let live = Arc::new(AtomicUsize::new(0));
    let server = web_server::start_with_schema(
        stub_schema(Arc::clone(&live), store),
        EPHEMERAL_LOOPBACK,
        web_root(directory.path()),
    )
    .await
    .expect("start server");
    let address = server.local_addr();

    let mut socket = connect(address).await;
    initialize(&mut socket).await;
    subscribe_to_first_tick(&mut socket, "ticks-1").await;
    assert_eq!(live.load(Ordering::SeqCst), 1);

    // Completion is the client's, not the stream's: the stub never ends on its
    // own, so releasing the resource can only be the server reacting.
    complete_subscription(&mut socket, "ticks-1").await;
    wait_for_live_subscriptions(&live, 0).await;

    // A stale registration under the completed id would make the server reject
    // this with close code 4409 instead of streaming again.
    subscribe_to_first_tick(&mut socket, "ticks-1").await;
    assert_eq!(live.load(Ordering::SeqCst), 1);

    complete_subscription(&mut socket, "ticks-1").await;
    wait_for_live_subscriptions(&live, 0).await;

    socket.close(None).await.expect("close the socket");
    while let Some(frame) = socket.next().await {
        match frame.expect("drain the closing socket") {
            Message::Close(_) => break,
            _ => continue,
        }
    }

    server.shutdown().await.expect("stop server");
}

#[tokio::test]
async fn disconnecting_mid_stream_releases_the_subscription() {
    let directory = TempDir::new().expect("temporary directory");
    let store = temporary_store(directory.path()).await;
    let live = Arc::new(AtomicUsize::new(0));
    let server = web_server::start_with_schema(
        stub_schema(Arc::clone(&live), store),
        EPHEMERAL_LOOPBACK,
        web_root(directory.path()),
    )
    .await
    .expect("start server");

    let mut socket = connect(server.local_addr()).await;
    initialize(&mut socket).await;
    subscribe_to_first_tick(&mut socket, "ticks-1").await;
    assert_eq!(live.load(Ordering::SeqCst), 1);

    drop(socket);
    wait_for_live_subscriptions(&live, 0).await;

    server.shutdown().await.expect("stop server");
}

#[tokio::test]
async fn the_app_schema_declares_the_subscription_root_its_modules_registered() {
    let directory = TempDir::new().expect("temporary directory");
    let database = file_database(&directory.path().join("server-store.db"))
        .await
        .expect("open a temporary Store");
    let sdl = app_schema(database).expect("compose the App Schema").sdl();

    // The root exists because a Module registered a field on it, not because
    // Seaography declares one by default: an empty Subscription root is
    // invalid SDL, and composing one would fail before this assertion.
    assert!(sdl.contains("type Subscription {"), "{sdl}");
    assert!(sdl.contains("subscription: Subscription"), "{sdl}");
    assert!(
        sdl.contains("documentsSaved: DocumentsSavedEvent!"),
        "{sdl}"
    );
}

/// A Module's own subscription, over the composed Server Process, over a real
/// graphql-ws socket: subscribe, run the operation that publishes, receive
/// the event.
#[tokio::test]
async fn a_module_subscription_delivers_over_the_web_transport() {
    let directory = TempDir::new().expect("temporary directory");
    let config = web_server::ServerConfig {
        bind: EPHEMERAL_LOOPBACK,
        store: directory.path().join("server-store.db"),
        web_root: web_root(directory.path()).to_owned(),
    };
    let server = web_server::start(&config).await.expect("start server");
    let address = server.local_addr();

    let http = |query: String| async move {
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
    };

    http(
        r#"mutation {
              documentsCreateOne(data: {
                id: "d1", taskId: "t", scope: "design",
                rootDir: "/workspace", relPath: "docs/design.md",
                createdAt: "2026-08-24T00:00:00Z", updatedAt: "2026-08-24T00:00:00Z"
              }) { id }
            }"#
        .to_owned(),
    )
    .await;

    let mut socket = connect(address).await;
    initialize(&mut socket).await;
    send(
        &mut socket,
        json!({
            "id": "documents-saved",
            "type": "subscribe",
            "payload": { "query": "subscription Saved { documentsSaved { documentId digest } }" }
        }),
    )
    .await;

    // The subscriber has to be registered before the save runs, and the
    // protocol has no "subscribed" acknowledgement. Retrying the save until
    // an event arrives is what makes that ordering the test's problem rather
    // than a sleep's.
    let mut digest = 0;
    let event = loop {
        digest += 1;
        let saved = http(format!(
            r#"mutation {{
                 documentsSave(documentId: "d1", expectedDigest: "{}",
                               digest: "digest-{digest}", savedAt: "2026-08-24T01:00:00Z")
                 {{ saved }}
               }}"#,
            if digest == 1 {
                String::new()
            } else {
                format!("digest-{}", digest - 1)
            }
        ))
        .await;
        assert_eq!(saved["documentsSave"]["saved"], true);

        if let Ok(message) =
            tokio::time::timeout(Duration::from_millis(500), next_message(&mut socket)).await
        {
            break message;
        }
        assert!(digest < 10, "no event arrived after {digest} saves");
    };

    assert_eq!(event["type"], "next", "unexpected message: {event:#}");
    assert_eq!(event["id"], "documents-saved");
    assert_eq!(
        event["payload"]["data"]["documentsSaved"]["documentId"],
        "d1"
    );
    assert_eq!(
        event["payload"]["data"]["documentsSaved"]["digest"],
        format!("digest-{digest}")
    );

    complete_subscription(&mut socket, "documents-saved").await;
    server.shutdown().await.expect("stop server");
}
