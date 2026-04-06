#[path = "support/mod.rs"]
mod support;

use serde_json::json;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let file = client
        .files()
        .create()
        .multipart_text("purpose", "fine-tune")
        .multipart_file("file", support::sample_training_file())
        .send()
        .await?;

    println!("uploaded file: {}", file.id);

    let job = client
        .fine_tuning()
        .jobs()
        .create()
        .body_value(json!({
            "model": "gpt-4o-mini",
            "training_file": file.id
        }))
        .send()
        .await?;

    println!("job: {job:#?}");

    if let Some(job_id) = job.get("id").and_then(serde_json::Value::as_str) {
        let events = client
            .fine_tuning()
            .jobs()
            .list_events(job_id)
            .limit(20)
            .send()
            .await?;
        println!("events: {:#?}", events.data);
    }

    Ok(())
}
