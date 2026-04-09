use openai_rs::helpers::ToolDefinition;
use openai_rs::resources::{ChatToolDefinition, ChatToolFunction};
use openai_rs::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherArgs {
    city: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
struct WeatherAnswer {
    city: String,
}

fn main() {
    let client = Client::builder()
        .api_key("sk-canary")
        .base_url("http://127.0.0.1:4010/v1")
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _ = client
        .responses()
        .parse::<WeatherAnswer>()
        .model("gpt-5.4")
        .input_text("return weather json");

    let tool = ToolDefinition::from_schema::<WeatherArgs, _, _, _>(
        "lookup_weather",
        Some("Return the weather for a city."),
        |arguments: Value| async move {
            let _ = arguments;
            Ok(json!({"city": "Shanghai"}))
        },
    );

    let _ = client
        .chat()
        .completions()
        .run_tools()
        .model("gpt-5.4")
        .message_user("what is the weather?")
        .register_tool(tool);

    let _ = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("call a tool")
        .tool(ChatToolDefinition {
            tool_type: "function".into(),
            function: ChatToolFunction {
                name: "lookup_weather".into(),
                description: Some("Return the weather for a city.".into()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    },
                    "required": ["city"]
                })
                .into(),
            },
        });
}
