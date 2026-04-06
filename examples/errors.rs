#[path = "support/mod.rs"]
mod support;

use openai_rs::{ApiErrorKind, Error};
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let result = client
        .chat()
        .completions()
        .create()
        .model("unknown-model")
        .message_user("Say this is a test")
        .send()
        .await;

    match result {
        Ok(response) => {
            println!("{response:#?}");
        }
        Err(Error::Api(api)) => {
            println!("request_id: {:?}", api.request_id);
            println!("status: {}", api.status);
            println!("kind: {:?}", api.kind);
            println!("message: {}", api.message);
            if matches!(api.kind, ApiErrorKind::NotFound) {
                println!("模型不存在或当前账号不可用。");
            }
        }
        Err(other) => return Err(other.into()),
    }

    Ok(())
}
