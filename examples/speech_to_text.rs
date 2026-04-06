#[path = "support/mod.rs"]
mod support;

use openai_rs::{RecordAudioOptions, record_audio};
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    println!("recording for 5 seconds...");
    let audio = record_audio(RecordAudioOptions {
        timeout: Some(std::time::Duration::from_secs(5)),
        ..RecordAudioOptions::default()
    })
    .await?;

    let transcription = client
        .audio()
        .transcriptions()
        .create()
        .multipart_text("model", "gpt-4o-mini-transcribe")
        .multipart_file("file", audio)
        .send()
        .await?;

    println!("{transcription:#?}");
    Ok(())
}
