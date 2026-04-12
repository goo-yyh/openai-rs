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
        .messages(support::demo_messages())
        .extra_body("tools", support::book_tools_json())
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ChatCompletionRuntimeEvent::ToolCallArgumentsDelta(event) => {
                println!("tool {} delta: {}", event.name, event.arguments_delta);
                println!("parsed: {:?}", event.parsed_arguments);
            }
            ChatCompletionRuntimeEvent::ContentDelta(event) => {
                print!("{}", event.delta);
            }
            _ => {}
        }
    }

    if let Some(final_completion) = stream.snapshot() {
        println!("\nfinal: {final_completion:#?}");
    }

    Ok(())
}
