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
        .message_user("只回答一个英文单词。")
        .extra_body("logprobs", serde_json::Value::Bool(true))
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ChatCompletionRuntimeEvent::LogProbsContentDelta(event) => {
                println!("delta logprobs: {:?}", event.values);
            }
            ChatCompletionRuntimeEvent::LogProbsContentDone(event) => {
                println!("done logprobs: {:?}", event.values);
            }
            _ => {}
        }
    }

    Ok(())
}
