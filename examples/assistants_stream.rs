#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_core::AssistantRuntimeEvent;
use serde_json::{Value, json};
use support::ExampleResult;

fn text_value(value: &Value) -> Option<&str> {
    value
        .get("text")
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
}

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let assistant = client
        .beta()
        .assistants()
        .create()
        .body_value(json!({
            "model": "gpt-5.4",
            "name": "Math Tutor",
            "instructions": "你是一个数学助教。"
        }))
        .send()
        .await?;

    let thread = client
        .beta()
        .threads()
        .create()
        .body_value(json!({
            "messages": [{
                "role": "user",
                "content": "请解释 3x + 11 = 14 怎么求解。"
            }]
        }))
        .send()
        .await?;

    let mut stream = client
        .beta()
        .threads()
        .runs()
        .create_and_stream(thread.id)
        .body_value(json!({
            "assistant_id": assistant.id,
            "additional_instructions": "请用中文回答。"
        }))
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            AssistantRuntimeEvent::TextDelta(event) => {
                if let Some(text) = text_value(&event.delta) {
                    print!("{text}");
                }
            }
            AssistantRuntimeEvent::TextDone(event) => {
                if let Some(text) = text_value(&event.text) {
                    println!("\n\nfinal: {text}");
                }
            }
            _ => {}
        }
    }

    Ok(())
}
