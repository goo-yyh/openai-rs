#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use openai_core::ChatCompletionRuntimeEvent;
use serde::Deserialize;
use serde_json::json;
use support::ExampleResult;

#[derive(Debug, Deserialize)]
struct QueryOrdersArgs {
    table_name: String,
    columns: Vec<String>,
    conditions: Vec<QueryCondition>,
    order_by: String,
}

#[derive(Debug, Deserialize)]
struct QueryCondition {
    column: String,
    operator: String,
    value: String,
}

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .message_system("When the task is about orders, answer by calling the query_orders tool.")
        .message_user("Find delayed orders from last month. Do not answer directly.")
        .extra_body("tools", support::query_tool_json())
        .extra_body(
            "tool_choice",
            json!({
                "type": "function",
                "function": { "name": "query_orders" }
            }),
        )
        .send_events()
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            ChatCompletionRuntimeEvent::ToolCallArgumentsDelta(event) => {
                println!("delta: {}", event.arguments_delta);
                if let Some(parsed) = event.parsed_arguments {
                    let parsed: QueryOrdersArgs = serde_json::from_value(parsed.into())?;
                    println!("partial.table_name: {}", parsed.table_name);
                    println!("partial.columns: {:?}", parsed.columns);
                    println!("partial.order_by: {}", parsed.order_by);
                    for condition in parsed.conditions {
                        println!("partial.condition.column: {}", condition.column);
                        println!("partial.condition.operator: {}", condition.operator);
                        println!("partial.condition.value: {}", condition.value);
                    }
                }
            }
            ChatCompletionRuntimeEvent::ToolCallArgumentsDone(event) => {
                if let Some(parsed) = event.parsed_arguments {
                    let parsed: QueryOrdersArgs = serde_json::from_value(parsed.into())?;
                    println!("final.table_name: {}", parsed.table_name);
                    println!("final.columns: {:?}", parsed.columns);
                    println!("final.order_by: {}", parsed.order_by);
                    for condition in parsed.conditions {
                        println!("final.condition.column: {}", condition.column);
                        println!("final.condition.operator: {}", condition.operator);
                        println!("final.condition.value: {}", condition.value);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
