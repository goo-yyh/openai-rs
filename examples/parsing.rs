#[cfg(feature = "structured-output")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
#[cfg(feature = "structured-output")]
use serde::Deserialize;

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct UI {
    label: String,
    component_type: String,
    fields: Vec<String>,
}

#[cfg(feature = "structured-output")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;

    let parsed = client
        .chat()
        .completions()
        .parse::<UI>()
        .model("gpt-5.4")
        .messages(vec![
            openai_core::ChatCompletionMessage::system("你是一个 UI 生成器，只输出 JSON。"),
            openai_core::ChatCompletionMessage::user("生成一个用户资料编辑表单"),
        ])
        .send()
        .await?;

    println!("parsed: {:#?}", parsed.parsed);
    println!("label: {}", parsed.parsed.label);
    println!("component_type: {}", parsed.parsed.component_type);
    println!("fields: {:?}", parsed.parsed.fields);
    Ok(())
}

#[cfg(not(feature = "structured-output"))]
fn main() {
    eprintln!("该示例需要开启 `structured-output` feature");
}
