#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .messages(support::demo_messages())
        .extra_body("tools", support::book_tools_json())
        .send()
        .await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        println!("{chunk:#?}");
    }

    println!(
        "final completion: {:#?}",
        stream.final_chat_completion().await?
    );
    Ok(())
}
