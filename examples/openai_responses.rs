use openai_core::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("用一句话解释 Rust 所有权")
        .send()
        .await?;

    println!("{:?}", response.output_text());
    Ok(())
}
