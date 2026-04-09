use openai_rs::resources::{ChatToolDefinition, ChatToolFunction};
use openai_rs::Client;
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
        .input_text("call a tool")
        .tool(ChatToolDefinition {
            tool_type: "function".into(),
            function: ChatToolFunction {
                name: "lookup_city".into(),
                description: Some("Return a city payload.".into()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    },
                    "required": ["city"]
                })
                .into(),
            },
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.output_text().as_deref(), Some("{\"city\":\"Shanghai\"}"));
}
