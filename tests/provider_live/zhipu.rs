use openai_rs::Client;

fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            eprintln!("skip live test because {name} is missing");
            None
        }
    }
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
async fn test_live_zhipu_chat_completion_basic() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = Client::builder()
        .provider(openai_rs::Provider::zhipu())
        .api_key(api_key)
        .build()
        .unwrap();

    let response = client
        .chat()
        .completions()
        .create()
        .model(std::env::var("ZHIPU_CHAT_MODEL").unwrap_or_else(|_| "glm-5".into()))
        .message_user("介绍一下 Rust 的所有权模型")
        .send()
        .await
        .unwrap();

    assert!(!response.choices.is_empty());
}
