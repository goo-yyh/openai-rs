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
#[ignore = "requires ZENMUX_API_KEY"]
async fn test_live_zenmux_models_list() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        return;
    };

    let client = Client::builder()
        .provider(openai_rs::Provider::zenmux())
        .api_key(api_key)
        .build()
        .unwrap();

    let page = client.models().list().send().await.unwrap();
    assert!(!page.data.is_empty());
}
