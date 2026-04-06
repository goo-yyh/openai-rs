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
        .input_text("solve 8x + 31 = 2")
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ResponseRuntimeEvent::OutputTextDelta(event) => {
                print!("{}", event.text);
            }
            ResponseRuntimeEvent::Completed(response) => {
                println!("\n\nfinal: {:?}", response.output_text());
            }
            _ => {}
        }
    }

    Ok(())
}
