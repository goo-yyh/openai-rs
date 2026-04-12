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

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("hello from ecosystem fixture")
        .extra_body("metadata", json!({"fixture": "tokio_responses_client"}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.output_text().as_deref(), Some("{\"city\":\"Shanghai\"}"));

    let streamed = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("hello from response stream fixture")
        .send()
        .await
        .unwrap()
        .final_response()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(streamed.output_text().as_deref(), Some("stream fixture"));
}
