#[cfg(feature = "tool-runner")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "tool-runner")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;

    let runner = client
        .chat()
        .completions()
        .run_tools()
        .model("gpt-5.4")
        .message_user("旧金山现在天气怎么样？")
        .register_tool(support::weather_tool())
        .into_streaming_runner()
        .await?;

    for event in runner.events() {
        println!("event: {event:?}");
    }

    println!("messages: {:#?}", runner.messages());
    println!("final content: {:?}", runner.final_content());
    Ok(())
}

#[cfg(not(feature = "tool-runner"))]
fn main() {
    eprintln!("该示例需要开启 `tool-runner` feature");
}
