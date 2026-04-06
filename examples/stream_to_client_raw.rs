#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_rs::ResponseRuntimeEvent;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut upstream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("用三句话介绍 Rust。")
        .send_events()
        .await?;

    while let Some(event) = upstream.next().await {
        let event = event?;
        match &event {
            ResponseRuntimeEvent::OutputTextDelta(delta) => {
                println!("event: response.output_text.delta");
                println!(
                    "data: {}\n",
                    serde_json::to_string(&serde_json::json!({
                        "delta": delta.text,
                        "snapshot": delta.snapshot
                    }))?
                );
            }
            ResponseRuntimeEvent::Completed(response) => {
                println!("event: response.completed");
                println!("data: {}\n", serde_json::to_string(response)?);
            }
            _ => {}
        }
    }

    Ok(())
}
