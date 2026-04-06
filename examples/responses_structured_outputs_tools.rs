#[path = "support/mod.rs"]
mod support;

use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("look up all delayed orders from last month")
        .extra_body("tools", support::query_tool_json())
        .send()
        .await?;

    println!("{response:#?}");
    if let Some(first) = response.output.first() {
        println!("first output item: {first:#?}");
    }
    Ok(())
}
