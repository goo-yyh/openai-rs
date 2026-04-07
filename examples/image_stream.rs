#[path = "support/mod.rs"]
mod support;

use base64::Engine;
use futures_util::StreamExt;
use serde_json::Value;
use support::ExampleResult;
use tokio::fs;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .images()
        .generate()
        .model("gpt-image-1")
        .prompt("A cute baby sea otter")
        .size("1024x1024")
        .partial_images(3)
        .send_sse()
        .await?;

    while let Some(event) = stream.next().await {
        let event = event?;
        println!("event: {}", event["type"]);
        if let Some(b64) = event.get("b64_json").and_then(Value::as_str) {
            let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
            let filename = match event["type"].as_str().unwrap_or_default() {
                "image_generation.partial_image" => {
                    let index = event["partial_image_index"].as_u64().unwrap_or_default();
                    format!("partial_{index}.png")
                }
                _ => "final_image.png".into(),
            };
            fs::write(support::output_path(&filename), bytes).await?;
        }
    }

    Ok(())
}
