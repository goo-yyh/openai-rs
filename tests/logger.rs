use std::sync::{Arc, Mutex};

use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_core::{Client, LogLevel, LogRecord};

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
async fn test_should_emit_sdk_logs_to_custom_logger() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_log",
            "object": "response",
            "status": "completed",
            "output": []
        })))
        .mount(&server)
        .await;

    let records = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .log_level(LogLevel::Debug)
        .logger({
            let records = records.clone();
            move |record: &LogRecord| {
                records.lock().unwrap().push(record.clone());
            }
        })
        .build()
        .unwrap();

    let _ = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    let records = records.lock().unwrap();
    assert!(records.iter().any(|record| {
        record.level == LogLevel::Debug
            && record.target == "openai_core::transport"
            && record.message == "发送请求"
    }));
}

#[tokio::test]
#[serial]
async fn test_should_read_log_level_from_env() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_env_log",
            "object": "response",
            "status": "completed",
            "output": []
        })))
        .mount(&server)
        .await;

    let server_uri = server.uri();
    let _guard = EnvGuard::set(&[
        ("OPENAI_BASE_URL", server_uri.as_str()),
        ("OPENAI_API_KEY", "sk-env-log"),
        ("OPENAI_LOG", "debug"),
    ]);

    let records = Arc::new(Mutex::new(Vec::<LogRecord>::new()));
    let client = Client::builder()
        .disable_proxy_for_local_base_url(true)
        .logger({
            let records = records.clone();
            move |record: &LogRecord| {
                records.lock().unwrap().push(record.clone());
            }
        })
        .build()
        .unwrap();

    let _ = client
        .responses()
        .create()
        .model("gpt-5")
        .input_text("hello")
        .send()
        .await
        .unwrap();

    let records = records.lock().unwrap();
    assert!(records.iter().any(|record| record.level == LogLevel::Debug));
}
