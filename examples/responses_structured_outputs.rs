#[cfg(feature = "structured-output")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
#[cfg(feature = "structured-output")]
use serde::Deserialize;

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct MathResponse {
    steps: Vec<String>,
    final_answer: String,
}

#[cfg(feature = "structured-output")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;

    let response = client
        .responses()
        .parse::<MathResponse>()
        .model("gpt-5.4")
        .input_text("solve 8x + 31 = 2")
        .send()
        .await?;

    println!("{:#?}", response.parsed);
    println!("steps: {:?}", response.parsed.steps);
    println!("answer: {}", response.parsed.final_answer);
    Ok(())
}

#[cfg(not(feature = "structured-output"))]
fn main() {
    eprintln!("该示例需要开启 `structured-output` feature");
}
