#[path = "support/mod.rs"]
mod support;

use serde_json::json;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let assistant = client
        .beta()
        .assistants()
        .create()
        .body_value(json!({
            "model": "gpt-5.4",
            "name": "Math Tutor",
            "instructions": "你是一个数学助教。"
        }))
        .send()
        .await?;

    let thread = client
        .beta()
        .threads()
        .create()
        .body_value(json!({
            "messages": [{
                "role": "user",
                "content": "请解释 3x + 11 = 14 怎么求解。"
            }]
        }))
        .send()
        .await?;

    let run = client
        .beta()
        .threads()
        .runs()
        .create_and_poll(
            thread.id.clone(),
            &json!({
                "assistant_id": assistant.id,
                "additional_instructions": "请用中文回答。"
            }),
            None,
        )
        .await?;

    println!("run status: {:?}", run.status);

    let messages = client
        .beta()
        .threads()
        .messages()
        .list(thread.id)
        .limit(20)
        .send()
        .await?;

    for message in messages.data {
        println!("{message:#?}");
    }

    Ok(())
}
