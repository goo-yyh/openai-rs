use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_core::{Client, LogLevel, LogRecord};

#[tokio::test]
async fn test_should_retry_after_ms_then_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after-ms", "15")
                .set_body_json(json!({
                    "error": {
                        "message": "slow down"
                    }
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req_retry_ms_ok")
                .set_body_json(json!({
                    "id": "resp_retry_ms",
                    "object": "response",
                    "status": "completed",
                    "output": [{"type":"output_text","text":"ok"}]
                })),
        )
        .mount(&server)
        .await;

    let records = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .max_retries(1)
        .log_level(LogLevel::Info)
        .logger({
            let records = records.clone();
            move |record: &LogRecord| {
                records.lock().unwrap().push(record.clone());
            }
        })
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

    assert_eq!(response.id, "resp_retry_ms");
    assert_eq!(response.meta.attempts, 2);
    assert_eq!(response.meta.request_id.as_deref(), Some("req_retry_ms_ok"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);

    let records = records.lock().unwrap();
    assert!(records.iter().any(|record| {
        record.level == LogLevel::Info
            && record.message == "请求失败，准备重试"
            && record.fields.get("status").map(String::as_str) == Some("429")
            && record.fields.get("delay_ms").map(String::as_str) == Some("15")
    }));
}

#[tokio::test]
async fn test_should_retry_after_header_then_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .set_body_json(json!({
                    "error": {
                        "message": "retry immediately"
                    }
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_retry_after",
            "object": "response",
            "status": "completed",
            "output": [{"type":"output_text","text":"ok"}]
        })))
        .mount(&server)
        .await;

    let records = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .max_retries(1)
        .log_level(LogLevel::Info)
        .logger({
            let records = records.clone();
            move |record: &LogRecord| {
                records.lock().unwrap().push(record.clone());
            }
        })
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_retry_after");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);

    let records = records.lock().unwrap();
    assert!(records.iter().any(|record| {
        record.level == LogLevel::Info
            && record.message == "请求失败，准备重试"
            && record.fields.get("status").map(String::as_str) == Some("429")
            && record.fields.get("delay_ms").map(String::as_str) == Some("0")
    }));
}

#[tokio::test]
async fn test_should_retry_after_timeout_then_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(80))
                .set_body_json(json!({
                    "id": "resp_timeout_slow",
                    "object": "response",
                    "status": "completed",
                    "output": [{"type":"output_text","text":"slow"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req_timeout_ok")
                .set_body_json(json!({
                    "id": "resp_timeout_ok",
                    "object": "response",
                    "status": "completed",
                    "output": [{"type":"output_text","text":"ok"}]
                })),
        )
        .mount(&server)
        .await;

    let records = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .timeout(Duration::from_millis(20))
        .max_retries(1)
        .log_level(LogLevel::Info)
        .logger({
            let records = records.clone();
            move |record: &LogRecord| {
                records.lock().unwrap().push(record.clone());
            }
        })
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

    assert_eq!(response.id, "resp_timeout_ok");
    assert_eq!(response.meta.attempts, 2);
    assert_eq!(response.meta.request_id.as_deref(), Some("req_timeout_ok"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);

    let records = records.lock().unwrap();
    assert!(records.iter().any(|record| {
        record.level == LogLevel::Info
            && record.message == "请求执行异常，准备重试"
            && record.fields.get("delay_ms").map(String::as_str) == Some("100")
    }));
}
