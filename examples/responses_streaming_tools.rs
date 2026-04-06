#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_rs::ResponseRuntimeEvent;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("look up all my delayed orders from last month")
        .extra_body("tools", support::query_tool_json())
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ResponseRuntimeEvent::FunctionCallArgumentsDelta(event) => {
                println!("delta: {}", event.delta);
                println!("snapshot: {}", event.snapshot);
                println!("parsed: {:?}", event.parsed_arguments);
            }
            ResponseRuntimeEvent::Completed(response) => {
                println!("completed: {response:#?}");
            }
            _ => {}
        }
    }

    Ok(())
}
