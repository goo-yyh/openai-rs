#[cfg(feature = "structured-output")]
#[path = "support/mod.rs"]
mod support;

#[cfg(feature = "structured-output")]
use schemars::JsonSchema;
#[cfg(feature = "structured-output")]
use serde::Deserialize;

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct GeneratedUi {
    screen_title: String,
    layout: String,
    primary_action: String,
    sections: Vec<UiSection>,
}

#[cfg(feature = "structured-output")]
#[derive(Debug, Deserialize, JsonSchema)]
struct UiSection {
    heading: String,
    component: String,
    fields: Vec<String>,
}

#[cfg(feature = "structured-output")]
#[tokio::main]
async fn main() -> support::ExampleResult {
    let client = support::openai_client()?;

    let parsed = client
        .chat()
        .completions()
        .parse::<GeneratedUi>()
        .model("gpt-5.4")
        .messages(vec![
            openai_core::ChatCompletionMessage::system(
                "You generate JSON UI specs for product teams. Only output JSON.",
            ),
            openai_core::ChatCompletionMessage::user(
                "Design a mobile onboarding screen for a finance app with identity verification.",
            ),
        ])
        .send()
        .await?;

    println!("screen_title: {}", parsed.parsed.screen_title);
    println!("layout: {}", parsed.parsed.layout);
    println!("primary_action: {}", parsed.parsed.primary_action);
    for section in parsed.parsed.sections {
        println!("section.heading: {}", section.heading);
        println!("section.component: {}", section.component);
        println!("section.fields: {:?}", section.fields);
    }
    Ok(())
}

#[cfg(not(feature = "structured-output"))]
fn main() {
    eprintln!("该示例需要开启 `structured-output` feature");
}
