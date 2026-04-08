#[cfg(feature = "structured-output")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
#[cfg(feature = "structured-output")]
use serde::Deserialize;
#[cfg(feature = "structured-output")]
use serde_json::json;

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct QueryOrdersArgs {
    table_name: String,
    columns: Vec<String>,
    conditions: Vec<QueryCondition>,
    order_by: String,
}

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct QueryCondition {
    column: String,
    operator: String,
    value: String,
}

#[cfg(feature = "structured-output")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;

    let parsed = client
        .chat()
        .completions()
        .parse::<QueryOrdersArgs>()
        .model("gpt-5.4")
        .messages(vec![
            openai_rs::ChatCompletionMessage::system(
                "When the task is about orders, answer by calling the query_orders tool.",
            ),
            openai_rs::ChatCompletionMessage::user(
                "Find delayed orders from last month. Do not answer directly.",
            ),
        ])
        .extra_body("tools", support::query_tool_json())
        .extra_body(
            "tool_choice",
            json!({
                "type": "function",
                "function": { "name": "query_orders" }
            }),
        )
        .send()
        .await?;

    println!("table_name: {}", parsed.parsed.table_name);
    println!("columns: {:?}", parsed.parsed.columns);
    for condition in parsed.parsed.conditions {
        println!("condition.column: {}", condition.column);
        println!("condition.operator: {}", condition.operator);
        println!("condition.value: {}", condition.value);
    }
    println!("order_by: {}", parsed.parsed.order_by);
    Ok(())
}

#[cfg(not(feature = "structured-output"))]
fn main() {
    eprintln!("该示例需要开启 `structured-output` feature");
}
