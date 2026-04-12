#[cfg(feature = "responses-ws")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "responses-ws")]
use futures_util::StreamExt;
#[cfg(feature = "responses-ws")]
use openai_core::SocketStreamMessage;

#[cfg(feature = "responses-ws")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;
    let socket = client.responses().ws().connect().await?;
    let mut stream = socket.stream();

    socket
        .send_json(&serde_json::json!({
            "type": "response.create",
            "response": {
                "model": "gpt-5.4",
                "input": "hello from websocket"
            }
        }))
        .await?;

    while let Some(event) = stream.next().await {
        match event {
            SocketStreamMessage::Message(message) => println!("{message:#?}"),
            SocketStreamMessage::Error(error) => {
                eprintln!("{error}");
                break;
            }
            SocketStreamMessage::Close => break,
            _ => {}
        }
    }

    Ok(())
}

#[cfg(not(feature = "responses-ws"))]
fn main() {
    eprintln!("该示例需要开启 `responses-ws` feature");
}
