use bytes::Bytes;
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{Client, Completion, ConversationItem};

#[tokio::test]
async fn test_should_merge_default_query_and_request_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(query_param("api-version", "2025-01-01"))
        .and(query_param("request", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cmpl_query_1",
            "object": "text_completion",
            "created": 1,
            "model": "gpt-5",
            "choices": [{"index": 0, "finish_reason": "stop", "text": "ok", "logprobs": null}]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .default_query("api-version", "2025-01-01")
        .build()
        .unwrap();

    let response: Completion = client
        .completions()
        .create()
        .extra_query("request", "1")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.id, "cmpl_query_1");
}

#[tokio::test]
async fn test_should_encode_reserved_query_values() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(body_json(json!({
            "model": "gpt-5",
            "prompt": "hello"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"cmpl_1"})))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response: Completion = client
        .completions()
        .create()
        .extra_query("note", "hello world/你好")
        .extra_query("filter", "a&b=c")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.id, "cmpl_1");

    let requests = server.received_requests().await.unwrap();
    let raw_query = requests[0].url.query().unwrap_or_default().to_owned();
    assert!(raw_query.contains("filter=a%26b%3Dc"));
    assert!(raw_query.contains("note=hello+world%2F%E4%BD%A0%E5%A5%BD"));
}

#[tokio::test]
async fn test_should_encode_dynamic_path_segments() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp/unsafe?id=1",
            "object": "response",
            "status": "completed",
            "output": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .retrieve("resp/unsafe?id=1")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp/unsafe?id=1");
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].url.path(), "/responses/resp%2Funsafe%3Fid%3D1");
}

#[tokio::test]
async fn test_should_encode_nested_dynamic_path_segments() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "item?2=3",
            "object": "conversation.item",
            "type": "message",
            "role": "user",
            "content": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _: ConversationItem = client
        .conversations()
        .items()
        .retrieve("conv/1", "item?2=3")
        .send()
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].url.path(),
        "/conversations/conv%2F1/items/item%3F2%3D3"
    );
}

#[tokio::test]
async fn test_should_return_response_meta_via_send_with_meta() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req_meta_1")
                .set_body_json(json!({
                    "id": "resp_meta_1",
                    "object": "response",
                    "status": "completed",
                    "output": [{"type":"output_text","text":"ok"}]
                })),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("hello")
        .send_with_meta()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_meta_1");
    assert_eq!(response.meta.status.as_u16(), 200);
    assert_eq!(response.meta.request_id.as_deref(), Some("req_meta_1"));
    assert_eq!(response.meta.url, format!("{}/responses", server.uri()));
}

#[tokio::test]
async fn test_should_return_raw_http_response_via_send_raw() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/files/file_1/content"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain; charset=utf-8")
                .set_body_raw("hello", "text/plain"),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client.files().content("file_1").send_raw().await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/plain")
    );
    assert_eq!(response.into_body(), Bytes::from_static(b"hello"));
}
