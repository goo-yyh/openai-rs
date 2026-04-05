use openai_rs::{Client, UploadSource};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .api_key(std::env::var("OPENAI_API_KEY")?)
        .build()?;

    let file =
        UploadSource::from_bytes("hello from openai-rs", "demo.txt").with_mime_type("text/plain");

    let uploaded = client
        .files()
        .create()
        .multipart_text("purpose", "assistants")
        .multipart_file("file", file)
        .send()
        .await?;

    println!("{uploaded:#?}");
    Ok(())
}
