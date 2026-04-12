use openai_core::Client;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherAnswer {
    city: String,
}

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

    let parsed = client
        .responses()
        .parse::<WeatherAnswer>()
        .model("gpt-5.4")
        .input_text("return json")
        .send()
        .await
        .unwrap();

    assert_eq!(parsed.parsed.city, "Shanghai");
}
