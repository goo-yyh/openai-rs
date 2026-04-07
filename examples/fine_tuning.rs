#[path = "support/mod.rs"]
mod support;

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
        .model("gpt-4o-mini")
        .training_file(file.id.clone())
        .send()
        .await?;

    println!("job: {job:#?}");

    let events = client
        .fine_tuning()
        .jobs()
        .list_events(&job.id)
        .limit(20)
        .send()
        .await?;
    println!("events: {:#?}", events.data);

    Ok(())
}
