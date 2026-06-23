use std::time::Duration;

use openai_core::{ApiErrorKind, Client, Provider, ProviderKind};
use serde::Deserialize;
use serial_test::serial;

use super::common::{
    LiveCase, LiveTier, add_numbers_tool, assert_contains_any, assert_contains_chinese,
    assert_no_markdown_fence, assert_no_think_block, assert_sentence_count_at_most, env_or_skip,
    env_var_or, expect_api_error_shape, first_content, force_tool_choice, multiply_numbers_tool,
    parse_jsonish, parse_tool_arguments, retry_live, sanitize_visible_text,
};

#[derive(Debug, Deserialize)]
struct LocationAnswer {
    city: String,
    country: String,
}

fn live_client(api_key: String) -> Client {
    Client::builder()
        .provider(Provider::deepseek())
        .api_key(api_key)
        .timeout(Duration::from_secs(90))
        .max_retries(4)
        .build()
        .unwrap()
}

fn live_client_no_retry(api_key: String) -> Client {
    Client::builder()
        .provider(Provider::deepseek())
        .api_key(api_key)
        .timeout(Duration::from_secs(90))
        .max_retries(0)
        .build()
        .unwrap()
}

fn chat_model() -> String {
    env_var_or("DEEPSEEK_CHAT_MODEL", "deepseek-v4-pro")
}

fn responses_model() -> String {
    env_var_or("DEEPSEEK_RESPONSES_MODEL", chat_model())
}

#[tokio::test]
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

#[tokio::test]
#[serial(provider_live)]
async fn test_live_deepseek_chat_completion_stream_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "deepseek",
        "chat_completion_stream_basic",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("DEEPSEEK_API_KEY") else {
        case.skip("DEEPSEEK_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let (request_id, content) = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("deepseek chat stream", 3, || async {
            let stream = client
                .chat()
                .completions()
                .stream()
                .model(model.clone())
                .message_user("请只用一句话说明 OpenAI 兼容 API 的一个工程价值。")
                .extra_body("thinking", serde_json::json!({"type":"disabled"}))
                .send()
                .await?;
            let request_id = stream.meta().request_id.clone();
            let content = stream.final_content().await?;
            Ok((request_id, content.unwrap_or_default()))
        })
        .await
    })
    .await
    .expect("deepseek streaming chat request timed out")
    .unwrap();

    let raw_content = content.clone();
    let content = sanitize_visible_text(&content);
    assert_no_markdown_fence(&raw_content);
    assert_no_think_block(&raw_content);
    assert_contains_chinese(&content);
    assert_sentence_count_at_most(&content, 1);
    case.success(
        request_id.as_deref(),
        format!(
            "stream_output={content}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[serial(provider_live)]
async fn test_live_deepseek_chat_structured_json_output() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "deepseek",
        "chat_structured_json_output",
        LiveTier::Extended,
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
        retry_live("deepseek chat structured output", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user(
                    "从字符串 'Paris, France' 中提取 city 和 country，直接返回 JSON 对象，格式固定为 {\"city\":\"Paris\",\"country\":\"France\"}，不要 markdown，不要额外说明。",
                )
                .extra_body("thinking", serde_json::json!({"type":"disabled"}))
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("deepseek structured output request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    let parsed: LocationAnswer = parse_jsonish(&text).unwrap();
    assert_eq!(parsed.city, "Paris");
    assert_eq!(parsed.country, "France");
    case.success(
        request_id.as_deref(),
        format!(
            "structured_output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[serial(provider_live)]
async fn test_live_deepseek_chat_tool_calling() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "deepseek",
        "chat_tool_calling",
        LiveTier::Extended,
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
        retry_live("deepseek chat tool calling", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请调用 add_numbers 工具计算 2 + 3，不要直接给出答案。")
                .tool(add_numbers_tool())
                .tool(multiply_numbers_tool())
                .tool_choice(force_tool_choice("add_numbers"))
                .extra_body("thinking", serde_json::json!({"type":"disabled"}))
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("deepseek tool calling request timed out")
    .unwrap();

    let message = &response.choices[0].message;
    let request_id = response.meta.request_id.clone();
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    assert_eq!(tool_call.function.name, "add_numbers");
    assert_eq!(arguments["a"], 2);
    assert_eq!(arguments["b"], 3);
    case.success(
        request_id.as_deref(),
        format!(
            "tool={} args={}; request_id={}",
            tool_call.function.name,
            tool_call.function.arguments,
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[serial(provider_live)]
async fn test_live_deepseek_responses_text_or_provider_error_shape() {
    let model = responses_model();
    let Some(case) = LiveCase::begin(
        "deepseek",
        "responses_text_or_provider_error_shape",
        LiveTier::Slow,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("DEEPSEEK_API_KEY") else {
        case.skip("DEEPSEEK_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let result = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("deepseek responses", 3, || async {
            client
                .responses()
                .create()
                .model(model.clone())
                .input_text("请只回答 OK。")
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("deepseek responses request timed out");

    match result {
        Ok(response) => {
            let request_id = response.meta.request_id.clone();
            let raw_text = response.output_text().unwrap_or_default();
            let text = sanitize_visible_text(&raw_text);
            assert_no_markdown_fence(&raw_text);
            assert_no_think_block(&raw_text);
            assert_contains_any(&text, &["OK", "好", "可以"]);
            case.success(
                request_id.as_deref(),
                format!(
                    "responses_output={text}; request_id={}",
                    request_id.as_deref().unwrap_or("-")
                ),
            );
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::DeepSeek);
            assert!(matches!(
                api.kind,
                ApiErrorKind::BadRequest
                    | ApiErrorKind::NotFound
                    | ApiErrorKind::UnprocessableEntity
                    | ApiErrorKind::Unknown
                    | ApiErrorKind::InternalServer
            ));
            case.expected_api_error(
                &api,
                format!(
                    "status={} kind={:?} message={}",
                    api.status, api.kind, api.message
                ),
            );
        }
    }
}

#[tokio::test]
#[serial(provider_live)]
async fn test_live_deepseek_invalid_model_error_shape() {
    let Some(case) = LiveCase::begin(
        "deepseek",
        "invalid_model_error_shape",
        LiveTier::Smoke,
        Some("definitely-not-a-real-deepseek-model"),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("DEEPSEEK_API_KEY") else {
        case.skip("DEEPSEEK_API_KEY missing");
        return;
    };

    let client = live_client_no_retry(api_key);

    let error = tokio::time::timeout(Duration::from_secs(90), async {
        let result = client
            .chat()
            .completions()
            .create()
            .model("definitely-not-a-real-deepseek-model")
            .message_user("hello")
            .send()
            .await;
        result.unwrap_err()
    })
    .await
    .expect("deepseek invalid model request timed out");

    let api = expect_api_error_shape(error, ProviderKind::DeepSeek);
    assert!(matches!(
        api.kind,
        ApiErrorKind::BadRequest
            | ApiErrorKind::NotFound
            | ApiErrorKind::UnprocessableEntity
            | ApiErrorKind::Unknown
    ));
    case.expected_api_error(
        &api,
        format!(
            "status={} kind={:?} message={}",
            api.status, api.kind, api.message
        ),
    );
}
