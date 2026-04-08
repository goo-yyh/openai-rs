use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{body_json, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{Client, Completion, Provider};

#[derive(Debug)]
struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn set(pairs: &[(&str, &str)]) -> Self {
        let saved = pairs
            .iter()
            .map(|(key, _)| ((*key).to_owned(), std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in pairs {
            // SAFETY: 测试使用 `serial` 串行运行，避免并发修改进程环境变量。
            unsafe { std::env::set_var(key, value) };
        }
        Self { saved }
    }

    fn remove(keys: &[&str]) -> Self {
        let saved = keys
            .iter()
            .map(|key| ((*key).to_owned(), std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for key in keys {
            // SAFETY: 测试使用 `serial` 串行运行，避免并发修改进程环境变量。
            unsafe { std::env::remove_var(key) };
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => {
                    // SAFETY: 测试使用 `serial` 串行运行，避免并发修改进程环境变量。
                    unsafe { std::env::set_var(&key, value) };
                }
                None => {
                    // SAFETY: 测试使用 `serial` 串行运行，避免并发修改进程环境变量。
                    unsafe { std::env::remove_var(&key) };
                }
            }
        }
    }
}

#[tokio::test]
#[serial]
async fn test_should_build_default_openai_base_url() {
    let client = Client::builder()
        .provider(Provider::openai())
        .api_key("sk-test")
        .build()
        .unwrap();
    assert_eq!(client.base_url(), "https://api.openai.com/v1");
}

#[tokio::test]
#[serial]
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
#[serial]
async fn test_should_merge_default_headers_and_request_headers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(header("x-default", "1"))
        .and(header("x-request", "2"))
        .and(header_exists("authorization"))
        .and(body_json(json!({"model": "gpt-5", "prompt": "hello"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"cmpl_1",
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
        .default_header("x-default", "1")
        .build()
        .unwrap();

    let value: Completion = client
        .completions()
        .create()
        .extra_header("x-request", "2")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(value.id, "cmpl_1");
}

#[tokio::test]
#[serial]
async fn test_should_remove_header_when_value_is_none() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(header("x-keep", "yes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"cmpl_header_1",
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
        .default_header("x-remove", "no")
        .default_header("x-keep", "yes")
        .build()
        .unwrap();

    let response: Completion = client
        .completions()
        .create()
        .remove_header("x-remove")
        .body_value(json!({"model": "gpt-5", "prompt": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.id, "cmpl_header_1");
}

#[tokio::test]
#[serial]
async fn test_should_read_openai_base_url_and_api_key_from_env() {
    let server = MockServer::start().await;
    let server_uri = server.uri();
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(header("authorization", "Bearer sk-env"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_env",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"env ok"}]
        })))
        .mount(&server)
        .await;

    let _clear = EnvGuard::remove(&[
        "AZURE_OPENAI_ENDPOINT",
        "OPENAI_API_VERSION",
        "AZURE_OPENAI_API_KEY",
    ]);
    let _guard = EnvGuard::set(&[
        ("OPENAI_BASE_URL", server_uri.as_str()),
        ("OPENAI_API_KEY", "sk-env"),
    ]);

    let client = Client::builder()
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();
    assert_eq!(client.base_url(), server_uri);

    let response = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_env");
}

#[tokio::test]
#[serial]
async fn test_should_use_custom_reqwest_client_defaults() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(header("x-http-client", "custom"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_http_client",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"ok"}]
        })))
        .mount(&server)
        .await;

    let mut default_headers = HeaderMap::new();
    default_headers.insert("x-http-client", HeaderValue::from_static("custom"));
    let http_client = reqwest::Client::builder()
        .no_proxy()
        .default_headers(default_headers)
        .build()
        .unwrap();

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .http_client(http_client)
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_http_client");
}

#[tokio::test]
#[serial]
async fn test_should_keep_proxy_for_local_base_url_by_default() {
    let target = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_local_target",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"ok"}]
        })))
        .mount(&target)
        .await;

    let proxy = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502).set_body_json(json!({
            "error": {
                "message": "proxied local request"
            }
        })))
        .mount(&proxy)
        .await;

    let _clear = EnvGuard::remove(&[
        "NO_PROXY",
        "no_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]);
    let _guard = EnvGuard::set(&[
        ("HTTP_PROXY", proxy.uri().as_str()),
        ("http_proxy", proxy.uri().as_str()),
    ]);

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(target.uri())
        .max_retries(0)
        .build()
        .unwrap();

    let error = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap_err();

    match error {
        openai_rs::Error::Api(error) => {
            assert_eq!(error.status, 502);
            assert!(error.message.contains("proxied"));
        }
        other => panic!("expected api error, got {other:?}"),
    }

    let target_requests = target.received_requests().await.unwrap();
    let proxy_requests = proxy.received_requests().await.unwrap();
    assert_eq!(target_requests.len(), 0);
    assert_eq!(proxy_requests.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_should_disable_proxy_for_local_base_url_when_enabled() {
    let target = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_local_target",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"ok"}]
        })))
        .mount(&target)
        .await;

    let proxy = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502).set_body_json(json!({
            "error": {
                "message": "proxied local request"
            }
        })))
        .mount(&proxy)
        .await;

    let _clear = EnvGuard::remove(&[
        "NO_PROXY",
        "no_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ]);
    let _guard = EnvGuard::set(&[
        ("HTTP_PROXY", proxy.uri().as_str()),
        ("http_proxy", proxy.uri().as_str()),
    ]);

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(target.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_local_target");

    let target_requests = target.received_requests().await.unwrap();
    let proxy_requests = proxy.received_requests().await.unwrap();
    assert_eq!(target_requests.len(), 1);
    assert_eq!(proxy_requests.len(), 0);
}
