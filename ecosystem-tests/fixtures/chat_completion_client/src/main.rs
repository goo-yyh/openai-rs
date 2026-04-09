use openai_rs::Client;

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
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_user("hello from chat fixture")
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some("fixture assistant reply")
    );
    assert_eq!(
        response.choices[0].message.reasoning_details[0].as_raw()["summary"],
        "fixture"
    );
    assert_eq!(
        response
            .usage
            .as_ref()
            .and_then(|usage| usage.completion_tokens_details.as_ref())
            .and_then(|details| details.reasoning_tokens),
        Some(1)
    );
}
