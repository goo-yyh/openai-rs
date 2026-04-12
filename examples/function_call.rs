#[path = "support/mod.rs"]
mod support;

use openai_core::ChatCompletionMessage;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;
    let mut messages = support::demo_messages();

    loop {
        let completion = client
            .chat()
            .completions()
            .create()
            .model("gpt-5.4")
            .messages(messages.clone())
            .extra_body("tools", support::book_tools_json())
            .send()
            .await?;

        let message = completion.choices[0].message.clone();
        println!("assistant: {message:#?}");
        messages.push(message.clone());

        if message.tool_calls.is_empty() {
            break;
        }

        for tool_call in message.tool_calls {
            let output = support::dispatch_book_tool(
                &tool_call.function.name,
                &tool_call.function.arguments,
            )?;
            messages.push(ChatCompletionMessage::tool(
                tool_call.id,
                serde_json::to_string(&output)?,
            ));
        }
    }

    Ok(())
}
