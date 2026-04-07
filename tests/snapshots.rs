use futures_util::StreamExt;
use insta::{assert_debug_snapshot, assert_snapshot};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::Client;
#[cfg(feature = "realtime")]
use openai_rs::RealtimeServerEvent;
#[cfg(feature = "responses-ws")]
use openai_rs::ResponsesServerEvent;

#[tokio::test]
async fn test_should_snapshot_chat_completion_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_snapshot",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "ok"
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _ = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_system("你是一个测试助手")
        .message_user("请返回一句话")
        .temperature(0.2)
        .extra_body("metadata", json!({"suite":"snapshot"}))
        .send()
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_snapshot!(
        "chat_completion_request_body",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[tokio::test]
async fn test_should_snapshot_api_error_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-request-id", "req_snapshot")
                .set_body_json(json!({
                    "error": {
                        "message": "too many requests"
                    }
                })),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .max_retries(0)
        .build()
        .unwrap();

    let error = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("hello")
        .send()
        .await
        .unwrap_err();

    assert_debug_snapshot!("api_error_mapping", error);
}

#[tokio::test]
async fn test_should_snapshot_response_stream_aggregation() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hel\"}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lo\"}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"text\":\"hello\"}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut stream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }

    assert_snapshot!(
        "response_stream_aggregation",
        serde_json::to_string_pretty(&json!({
            "events": events,
            "output_text": stream.output_text(),
        }))
        .unwrap()
    );
}

#[cfg(feature = "responses-ws")]
#[test]
fn test_should_snapshot_responses_websocket_event_decode() {
    let event: ResponsesServerEvent = serde_json::from_value(json!({
        "type": "response.output_text.delta",
        "response_id": "resp_1",
        "item_id": "item_1",
        "delta": "hello"
    }))
    .unwrap();

    assert_debug_snapshot!("responses_websocket_event_decode", event);
}

#[cfg(feature = "realtime")]
#[test]
fn test_should_snapshot_realtime_websocket_event_decode() {
    let event: RealtimeServerEvent = serde_json::from_value(json!({
        "type": "session.created",
        "session": {
            "id": "sess_1",
            "object": "realtime.session"
        }
    }))
    .unwrap();

    assert_debug_snapshot!("realtime_websocket_event_decode", event);
}
