#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_rs::ChatCompletionRuntimeEvent;
use serde_json::json;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .message_system("只输出 JSON。")
        .message_user("返回 JSON，字段为 city 和 weather。")
        .extra_body(
            "response_format",
            json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "weather",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "city": { "type": "string" },
                            "weather": { "type": "string" }
                        },
                        "required": ["city", "weather"]
                    }
                }
            }),
        )
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        if let ChatCompletionRuntimeEvent::ContentDelta(event) = event? {
            println!("delta: {}", event.delta);
            println!("partial parsed: {:?}", event.parsed);
        }
    }

    Ok(())
}
