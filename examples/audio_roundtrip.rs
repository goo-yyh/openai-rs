#[path = "support/mod.rs"]
mod support;

use support::ExampleResult;
use tokio::fs;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mp3 = client
        .audio()
        .speech()
        .create()
        .body_value(serde_json::json!({
            "model": "gpt-4o-mini-tts",
            "voice": "alloy",
            "input": "the quick brown fox jumped over the lazy dogs"
        }))
        .send()
        .await?;

    let speech_path = support::output_path("speech.mp3");
    fs::write(&speech_path, &mp3).await?;
    println!("saved speech to {}", speech_path.display());

    let upload =
        openai_rs::UploadSource::from_bytes(mp3.clone(), "speech.mp3").with_mime_type("audio/mpeg");

    let transcription = client
        .audio()
        .transcriptions()
        .create()
        .multipart_text("model", "gpt-4o-mini-transcribe")
        .multipart_file("file", upload.clone())
        .send()
        .await?;

    let translation = client
        .audio()
        .translations()
        .create()
        .multipart_text("model", "gpt-4o-mini-transcribe")
        .multipart_file("file", upload)
        .send()
        .await?;

    println!("transcription: {transcription:#?}");
    println!("translation: {translation:#?}");
    Ok(())
}
