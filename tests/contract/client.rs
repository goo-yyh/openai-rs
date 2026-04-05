use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{body_json, header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{Client, Provider};

#[tokio::test]
async fn test_should_build_default_openai_base_url() {
    let client = Client::builder()
        .provider(Provider::openai())
        .api_key("sk-test")
        .build()
        .unwrap();
    assert_eq!(client.base_url(), "https://api.openai.com/v1");
}

#[tokio::test]
async fn test_should_override_base_url_with_builder_option() {
    let client = Client::builder()
        .provider(Provider::openai())
        .api_key("sk-test")
        .base_url("https://example.com/v1")
        .build()
        .unwrap();
    assert_eq!(client.base_url(), "https://example.com/v1");
}

#[tokio::test]
async fn test_should_merge_default_headers_and_request_headers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(header("x-default", "1"))
        .and(header("x-request", "2"))
        .and(header_exists("authorization"))
        .and(body_json(json!({"model": "gpt-5", "prompt": "hello"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"cmpl_1"})))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .default_header("x-default", "1")
        .build()
        .unwrap();

    let value: serde_json::Value = client
        .completions()
        .create()
        .extra_header("x-request", "2")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(value["id"], "cmpl_1");
}

#[tokio::test]
async fn test_should_remove_header_when_value_is_none() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(header("x-keep", "yes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .default_header("x-remove", "no")
        .default_header("x-keep", "yes")
        .build()
        .unwrap();

    let _: serde_json::Value = client
        .completions()
        .create()
        .remove_header("x-remove")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn test_should_merge_default_query_and_request_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(query_param("api-version", "2025-01-01"))
        .and(query_param("request", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .default_query("api-version", "2025-01-01")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    let _: serde_json::Value = client
        .completions()
        .create()
        .extra_query("request", "1")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn test_should_build_azure_request_from_endpoint_and_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/gpt-4o/chat/completions"))
        .and(query_param("api-version", "2024-02-15-preview"))
        .and(header("api-key", "azure-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_azure",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "azure ok"
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .azure_endpoint(server.uri())
        .azure_api_version("2024-02-15-preview")
        .api_key("azure-key")
        .build()
        .unwrap();

    let response = client
        .chat()
        .completions()
        .create()
        .model("gpt-4o")
        .message_user("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "chatcmpl_azure");
}

#[tokio::test]
async fn test_should_send_azure_bearer_token_when_using_ad_token_provider() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/responses"))
        .and(query_param("api-version", "2024-02-15-preview"))
        .and(header("authorization", "Bearer azure-ad-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_azure",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"ok"}]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .azure_endpoint(server.uri())
        .azure_api_version("2024-02-15-preview")
        .azure_ad_token_provider(|| async {
            Ok(secrecy::SecretString::new("azure-ad-token".into()))
        })
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-4o")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_azure");
}

#[test]
fn test_should_reject_base_url_and_azure_endpoint_together() {
    let error = Client::builder()
        .provider(Provider::azure())
        .base_url("https://example.com/openai")
        .azure_endpoint("https://example-resource.openai.azure.com")
        .api_key("azure-key")
        .build()
        .unwrap_err();

    assert!(matches!(error, openai_rs::Error::InvalidConfig(_)));
}
