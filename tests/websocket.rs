#![cfg(any(feature = "realtime", feature = "responses-ws"))]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

#[cfg(feature = "realtime")]
use openai_rs::OpenAIRealtimeWebSocket;
#[cfg(feature = "responses-ws")]
use openai_rs::OpenAIResponsesWebSocket;
use openai_rs::{
    Client, RealtimeServerEvent, ResponsesServerEvent, SocketCloseOptions, SocketStreamMessage,
};

#[derive(Debug, Clone)]
struct RecordedHandshake {
    uri: String,
    headers: BTreeMap<String, String>,
}

#[allow(clippy::result_large_err)]
async fn spawn_websocket_server(
    event_after_client_message: Option<serde_json::Value>,
) -> (
    String,
    oneshot::Receiver<RecordedHandshake>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));

    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let sender = sender.clone();
        let mut websocket =
            accept_hdr_async(stream, move |request: &Request, response: Response| {
                let recorded = RecordedHandshake {
                    uri: request.uri().to_string(),
                    headers: request
                        .headers()
                        .iter()
                        .filter_map(|(key, value)| {
                            value
                                .to_str()
                                .ok()
                                .map(|value| (key.as_str().to_owned(), value.to_owned()))
                        })
                        .collect(),
                };
                if let Some(sender) = sender.lock().unwrap().take() {
                    let _ = sender.send(recorded);
                }
                Ok(response)
            })
            .await
            .unwrap();

        if let Some(event) = event_after_client_message {
            let message = websocket.next().await.unwrap().unwrap();
            assert!(matches!(message, Message::Text(_)));
            websocket
                .send(Message::Text(event.to_string().into()))
                .await
                .unwrap();
        }

        websocket.close(None).await.unwrap();
    });

    (format!("http://{address}"), receiver, handle)
}

fn parse_query(uri: &str) -> BTreeMap<String, String> {
    let url = url::Url::parse(&format!("http://localhost{uri}")).unwrap();
    url.query_pairs().into_owned().collect()
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn test_should_connect_realtime_ws_and_receive_message() {
    let (server_url, handshake, handle) =
        spawn_websocket_server(Some(json!({"type":"response.created","id":"evt_1"}))).await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(format!("{server_url}/v1"))
        .build()
        .unwrap();

    let socket = client
        .realtime()
        .ws()
        .model("gpt-4o-realtime-preview")
        .connect()
        .await
        .unwrap();

    let mut events = socket.stream();
    assert!(matches!(
        events.next().await,
        Some(SocketStreamMessage::Open)
    ));

    socket
        .send_json(&json!({"type":"response.create"}))
        .await
        .unwrap();

    let message = events.next().await.unwrap();
    match message {
        SocketStreamMessage::Message(RealtimeServerEvent::ResponseCreated(event)) => {
            assert_eq!(event.id.as_deref(), Some("evt_1"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let recorded = handshake.await.unwrap();
    assert_eq!(
        recorded.headers.get("authorization"),
        Some(&"Bearer sk-test".into())
    );
    let query = parse_query(&recorded.uri);
    assert_eq!(
        query.get("model").map(String::as_str),
        Some("gpt-4o-realtime-preview")
    );
    assert!(recorded.uri.starts_with("/v1/realtime?"));

    handle.await.unwrap();
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn test_should_use_azure_realtime_deployment_and_api_key_header() {
    let (server_url, handshake, handle) = spawn_websocket_server(None).await;

    let client = Client::builder()
        .azure_endpoint(server_url)
        .azure_api_version("2024-02-15-preview")
        .azure_deployment("rt-deployment")
        .api_key("azure-key")
        .build()
        .unwrap();

    let socket = client.realtime().ws().connect().await.unwrap();
    socket.close(SocketCloseOptions::default()).await.unwrap();

    let recorded = handshake.await.unwrap();
    assert_eq!(recorded.headers.get("api-key"), Some(&"azure-key".into()));
    let query = parse_query(&recorded.uri);
    assert_eq!(
        query.get("deployment").map(String::as_str),
        Some("rt-deployment")
    );
    assert_eq!(
        query.get("api-version").map(String::as_str),
        Some("2024-02-15-preview")
    );
    assert!(recorded.uri.starts_with("/openai/realtime?"));

    handle.await.unwrap();
}

#[cfg(feature = "responses-ws")]
#[tokio::test]
async fn test_should_connect_responses_ws_and_use_bearer_auth() {
    let (server_url, handshake, handle) = spawn_websocket_server(Some(
        json!({"type":"response.output_text.delta","delta":"hi"}),
    ))
    .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(format!("{server_url}/v1"))
        .build()
        .unwrap();

    let socket = client.responses().ws().connect().await.unwrap();
    let mut events = socket.stream();
    assert!(matches!(
        events.next().await,
        Some(SocketStreamMessage::Open)
    ));

    socket
        .send_json(&json!({"type":"response.create","response":{"input":"hello"}}))
        .await
        .unwrap();

    let message = events.next().await.unwrap();
    match message {
        SocketStreamMessage::Message(ResponsesServerEvent::ResponseOutputTextDelta(event)) => {
            assert_eq!(event.delta.as_deref(), Some("hi"));
            assert_eq!(
                event
                    .raw
                    .data
                    .get("delta")
                    .and_then(serde_json::Value::as_str),
                Some("hi"),
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let recorded = handshake.await.unwrap();
    assert_eq!(
        recorded.headers.get("authorization"),
        Some(&"Bearer sk-test".into())
    );
    assert_eq!(recorded.uri, "/v1/responses");

    handle.await.unwrap();
}

#[cfg(feature = "realtime")]
#[tokio::test]
async fn test_should_connect_standalone_realtime_websocket_client() {
    let (server_url, handshake, handle) = spawn_websocket_server(None).await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(format!("{server_url}/v1"))
        .build()
        .unwrap();

    let socket = OpenAIRealtimeWebSocket::connect(client, "gpt-4o-realtime-preview")
        .await
        .unwrap();
    socket.close(SocketCloseOptions::default()).await.unwrap();

    let recorded = handshake.await.unwrap();
    let query = parse_query(&recorded.uri);
    assert_eq!(
        query.get("model").map(String::as_str),
        Some("gpt-4o-realtime-preview")
    );

    handle.await.unwrap();
}

#[cfg(feature = "responses-ws")]
#[tokio::test]
async fn test_should_connect_standalone_responses_websocket_client() {
    let (server_url, handshake, handle) = spawn_websocket_server(None).await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(format!("{server_url}/v1"))
        .build()
        .unwrap();

    let socket = OpenAIResponsesWebSocket::connect(client).await.unwrap();
    socket.close(SocketCloseOptions::default()).await.unwrap();

    let recorded = handshake.await.unwrap();
    assert_eq!(recorded.uri, "/v1/responses");

    handle.await.unwrap();
}
