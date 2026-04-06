#[path = "support/mod.rs"]
mod support;

use futures_util::StreamExt;
use support::ExampleResult;

#[tokio::main]
async fn main() -> ExampleResult {
    let client = support::openai_client()?;

    let first_page = client.models().list().limit(20).send().await?;
    for model in &first_page.data {
        println!("page1 model: {}", model.id);
    }

    if first_page.has_next_page() {
        let next_page = first_page.next_page().await?;
        for model in &next_page.data {
            println!("page2 model: {}", model.id);
        }
    }

    let mut stream = client.models().list().limit(20).send().await?.into_stream();
    while let Some(model) = stream.next().await {
        println!("stream model: {}", model?.id);
    }

    Ok(())
}
