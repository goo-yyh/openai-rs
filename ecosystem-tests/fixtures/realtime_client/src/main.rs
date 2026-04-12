use openai_core::Client;
use serde_json::json;

#[tokio::main]
async fn main() {
    let base_url = std::env::var("OPENAI_RS_FIXTURE_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:4010/v1".to_string());
    let client = Client::builder()
        .api_key("sk-fixture")
        .base_url(base_url)
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _socket = client
        .realtime()
        .ws()
        .model("gpt-realtime")
        .extra_header("x-fixture", "realtime_client");

    let secret = client
        .realtime()
        .client_secrets()
        .create()
        .body_value(json!({"session": {"type": "realtime"}}))
        .send()
        .await
        .unwrap();

    assert_eq!(secret.secret_value(), Some("ek_fixture_123"));
}
