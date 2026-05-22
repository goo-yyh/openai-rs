use std::time::Duration;

use openai_core::{Client, Provider};
use serial_test::serial;

use super::common::{
    LiveCase, LiveTier, assert_contains_chinese, assert_no_markdown_fence, assert_no_think_block,
    assert_sentence_count_at_most, env_or_skip, first_content, retry_live, sanitize_visible_text,
};

fn live_client(api_key: String) -> Client {
    Client::builder()
        .provider(Provider::deepseek())
        .api_key(api_key)
        .timeout(Duration::from_secs(90))
        .max_retries(4)
        .build()
        .unwrap()
}

fn chat_model() -> String {
    std::env::var("DEEPSEEK_CHAT_MODEL").unwrap_or_else(|_| "deepseek-v4-pro".into())
}

#[tokio::test]
#[ignore = "requires DEEPSEEK_API_KEY"]
#[serial(provider_live)]
async fn test_live_deepseek_chat_completion_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "deepseek",
        "chat_completion_basic",
        LiveTier::Smoke,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("DEEPSEEK_API_KEY") else {
        case.skip("DEEPSEEK_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("deepseek chat basic", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请用一句话说明本地优先应用的隐私优势。")
                .extra_body("thinking", serde_json::json!({"type":"disabled"}))
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("deepseek basic chat request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    assert!(!response.choices.is_empty());
    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    assert_contains_chinese(&text);
    assert_sentence_count_at_most(&text, 1);
    case.success(
        request_id.as_deref(),
        format!(
            "output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}
