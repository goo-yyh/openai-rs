use openai_core::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .azure_endpoint(std::env::var("AZURE_OPENAI_ENDPOINT")?)
        .azure_api_version("2024-02-15-preview")
        .azure_deployment(std::env::var("AZURE_OPENAI_DEPLOYMENT")?)
        .api_key(std::env::var("AZURE_OPENAI_API_KEY")?)
        .build()?;

    let response = client
        .responses()
        .create()
        .model("ignored-when-deployment-is-configured")
        .input_text("用一句话解释借用检查器")
        .send()
        .await?;

    println!("{:?}", response.output_text());
    Ok(())
}
