#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut first = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("给我一个分步骤推导，解方程 8x + 31 = 2")
        .extra_body("background", serde_json::Value::Bool(true))
        .send()
        .await?;

    let mut response_id = None;
    let mut sequence_number = None;

    while let Some(event) = first.next().await {
        let event = event?;
        println!("event: {event}");

        if response_id.is_none() {
            response_id = event
                .get("response")
                .and_then(|value| value.get("id"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    event
                        .get("response_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                });
        }

        sequence_number = event
            .get("sequence_number")
            .and_then(serde_json::Value::as_u64)
            .or(sequence_number);

        if sequence_number.unwrap_or_default() >= 10 {
            break;
        }
    }

    let response_id = response_id.ok_or("未拿到 response_id")?;
    let starting_after = sequence_number.ok_or("未拿到 sequence_number")?;

    println!("interrupted, continue from {response_id} after #{starting_after}");

    let mut resumed = client
        .responses()
        .stream_response(response_id)
        .starting_after(starting_after)
        .send()
        .await?;

    while let Some(event) = resumed.next().await {
        println!("resume event: {}", event?);
    }

    println!(
        "final: {:?}",
        resumed
            .final_response()
            .await?
            .and_then(|r| r.output_text())
    );
    Ok(())
}
