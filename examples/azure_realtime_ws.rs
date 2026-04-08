#[cfg(feature = "realtime")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "realtime")]
use futures_util::StreamExt;
#[cfg(feature = "realtime")]
use openai_rs::SocketStreamMessage;

#[cfg(feature = "realtime")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::azure_client()?;

    let socket = client.realtime().ws().connect().await?;
    let mut events = socket.stream();

    if let Some(SocketStreamMessage::Open) = events.next().await {
        socket
            .send_json(&serde_json::json!({
                "type": "response.create",
                "response": {
                    "modalities": ["text"],
                    "instructions": "Give a short status update about Azure Realtime support."
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
