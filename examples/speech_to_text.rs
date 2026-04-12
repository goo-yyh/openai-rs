#[path = "support/mod.rs"]
mod support;

use openai_core::{RecordAudioOptions, record_audio};
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
        .model("gpt-4o-mini-transcribe")
        .file(audio)
        .send()
        .await?;

    println!("{transcription:#?}");
    Ok(())
}
