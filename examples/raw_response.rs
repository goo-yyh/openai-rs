#[path = "support/mod.rs"]
mod support;

use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let raw = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_user("Say this is a test")
        .send_raw()
        .await?;

    println!("raw status: {}", raw.status());
    println!("raw headers: {:#?}", raw.headers());

    let response = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_user("Say this is a second test")
        .send_with_meta()
        .await?;

    println!("request_id: {:?}", response.meta.request_id);
    println!("status: {}", response.meta.status);
    println!("content: {:?}", response.choices[0].message.content);

    Ok(())
}
