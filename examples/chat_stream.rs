#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_core::ChatCompletionRuntimeEvent;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .message_user("请把“Sheep sleep deep”连续说三遍。")
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ChatCompletionRuntimeEvent::ContentDelta(event) => {
                print!("{}", event.delta);
            }
            ChatCompletionRuntimeEvent::ContentDone(event) => {
                println!("\n\nfinal: {}", event.content);
            }
            _ => {}
        }
    }

    Ok(())
}
