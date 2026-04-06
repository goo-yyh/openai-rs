#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::time::Duration;

use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = openai_rs::Client::builder()
        .webhook_secret(std::env::var("OPENAI_WEBHOOK_SECRET")?)
        .build()?;

    let raw_body = r#"{"type":"response.completed","data":{"id":"resp_123"}}"#;
    let headers = BTreeMap::from([
        ("openai-signature".to_string(), "v1=dummy".to_string()),
        ("openai-timestamp".to_string(), "1735689600".to_string()),
    ]);

    let verify_result =
        client
            .webhooks()
            .verify_signature(raw_body, &headers, None, Duration::from_secs(300));
    println!("verify result: {verify_result:?}");

    let unwrap_result: Result<serde_json::Value, _> =
        client
            .webhooks()
            .unwrap(raw_body, &headers, None, Duration::from_secs(300));
    println!("unwrap result: {unwrap_result:?}");

    Ok(())
}
