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
        .input_text("Write three short lines about Rust release engineering.")
        .send_events()
        .await?;

    println!("content-type: text/event-stream");
    println!("cache-control: no-cache\n");

    while let Some(event) = upstream.next().await {
        match event? {
            ResponseRuntimeEvent::OutputTextDelta(delta) => {
                println!("event: response.output_text.delta");
                println!(
                    "data: {}\n",
                    serde_json::to_string(&serde_json::json!({
                        "delta": delta.text,
                        "snapshot": delta.snapshot,
                    }))?
                );
            }
            ResponseRuntimeEvent::Completed(response) => {
                println!("event: response.completed");
                println!("data: {}\n", serde_json::to_string(&response)?);
            }
            _ => {}
        }
    }

    Ok(())
}
