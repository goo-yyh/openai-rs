#[path = "support/mod.rs"]
mod support;

use openai_rs::ChatCompletionMessage;
use serde_json::json;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let messages = vec![
        ChatCompletionMessage::system("You are a concise release assistant."),
        ChatCompletionMessage::user("Summarize the latest SDK release plan in two bullets."),
    ];

    let response = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .messages(messages)
        .temperature(0.2)
        .n(1)
        .max_tokens(180)
        .extra_body("stop", json!(["\n\n"]))
        .extra_body("metadata", json!({"example": "chat_params_types"}))
        .send_with_meta()
        .await?;

    println!("request_id: {:?}", response.meta.request_id);
    println!("{response:#?}");
    Ok(())
}
