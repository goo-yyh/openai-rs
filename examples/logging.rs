#[path = "support/mod.rs"]
mod support;

use std::sync::{Arc, Mutex};

use openai_core::{LogLevel, LogRecord};
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let collected: Arc<Mutex<Vec<LogRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&collected);

    let client = openai_core::Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .log_level(LogLevel::Info)
        .logger(move |record: &LogRecord| {
            println!("[{}] {}", record.target, record.message);
            sink.lock().expect("poisoned").push(record.clone());
        })
        .build()?;

    let _ = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("用一句话解释 Rust trait")
        .send()
        .await?;

    println!(
        "captured logs: {}",
        collected.lock().expect("poisoned").len()
    );
    Ok(())
}
