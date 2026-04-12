use openai_core::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let completion = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_system("你是一个 Rust 助手")
        .message_user("用一句话解释 Tokio 的运行时模型")
        .send()
        .await?;

    println!("{completion:#?}");
    Ok(())
}
