#[path = "support/mod.rs"]
mod support;

use openai_core::{AudioPlaybackInput, play_audio};
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let audio = client
        .audio()
        .speech()
        .create()
        .model("gpt-4o-mini-tts")
        .voice("nova")
        .input("Rust makes fearless concurrency practical.")
        .send()
        .await?;

    play_audio(AudioPlaybackInput::bytes(audio)).await?;
    Ok(())
}
