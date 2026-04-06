#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use serde_json::json;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .beta()
        .threads()
        .create_and_run_stream()
        .body_value(json!({
            "assistant_id": std::env::var("OPENAI_ASSISTANT_ID")?,
            "thread": {
                "messages": [{
                    "role": "user",
                    "content": "请用一句话介绍 Rust。"
                }]
            }
        }))
        .send()
        .await?;

    while let Some(event) = stream.next().await {
        let event = event?;
        println!("event: {}", event.event);
        println!("data: {}", serde_json::to_string_pretty(&event.data)?);
    }

    println!("snapshot: {:#?}", stream.snapshot());
    Ok(())
}
