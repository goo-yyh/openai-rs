#[cfg(feature = "realtime")]
use futures_util::StreamExt;
#[cfg(feature = "realtime")]
use openai_core::{Client, SocketStreamMessage};

#[cfg(feature = "realtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let socket = client
        .realtime()
        .ws()
        .model("gpt-4o-realtime-preview")
        .connect()
        .await?;

    let mut events = socket.stream();
    if let Some(SocketStreamMessage::Open) = events.next().await {
        socket
            .send_json(&serde_json::json!({
                "type": "response.create",
                "response": {
                    "modalities": ["text"],
                    "instructions": "介绍 Rust trait object"
                }
            }))
            .await?;
    }

    while let Some(event) = events.next().await {
        println!("{event:?}");
        if matches!(
            event,
            SocketStreamMessage::Close | SocketStreamMessage::Error(_)
        ) {
            break;
        }
    }

    Ok(())
}

#[cfg(not(feature = "realtime"))]
fn main() {
    eprintln!("该示例需要开启 `realtime` feature");
}
