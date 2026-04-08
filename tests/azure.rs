use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{Client, Provider};

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
        .disable_proxy_for_local_base_url(true)
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
async fn test_should_use_configured_azure_deployment_over_body_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/deployments/chat-prod/chat/completions"))
        .and(query_param("api-version", "2024-02-15-preview"))
        .and(header("api-key", "azure-key"))
        .and(body_json(json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_azure_deployment",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "deployment ok"
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .azure_endpoint(server.uri())
        .disable_proxy_for_local_base_url(true)
        .azure_api_version("2024-02-15-preview")
        .azure_deployment("chat-prod")
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

    assert_eq!(response.id, "chatcmpl_azure_deployment");
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
        .disable_proxy_for_local_base_url(true)
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

#[tokio::test]
async fn test_should_preserve_azure_realtime_http_path_without_deployment_prefix() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/realtime/client_secrets"))
        .and(query_param("api-version", "2024-02-15-preview"))
        .and(header("api-key", "azure-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "secret": "rt_secret"
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .azure_endpoint(server.uri())
        .disable_proxy_for_local_base_url(true)
        .azure_api_version("2024-02-15-preview")
        .azure_deployment("chat-prod")
        .api_key("azure-key")
        .build()
        .unwrap();

    let response = client
        .realtime()
        .client_secrets()
        .create()
        .body_value(json!({
            "session": {
                "type": "realtime"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.secret_value(), Some("rt_secret"));
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
