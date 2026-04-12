#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_core::ResponseRuntimeEvent;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut upstream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("Write three short lines about why typed streams are useful.")
        .send_events()
        .await?;

    while let Some(event) = upstream.next().await {
        match event? {
            ResponseRuntimeEvent::OutputTextDelta(delta) => {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "type": "response.output_text.delta",
                        "delta": delta.text,
                        "snapshot": delta.snapshot,
                    }))?
                );
            }
            ResponseRuntimeEvent::Completed(response) => {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({
                        "type": "response.completed",
                        "response": response,
                    }))?
                );
            }
            _ => {}
        }
    }

    Ok(())
}
