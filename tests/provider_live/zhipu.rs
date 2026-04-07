use std::time::Duration;

use openai_rs::{ApiErrorKind, Client, ProviderKind};
use serde::Deserialize;
use serial_test::serial;

use super::common::{
    LiveCase, LiveTier, add_numbers_tool, assert_contains_any, assert_contains_chinese,
    assert_no_markdown_fence, assert_no_think_block, assert_sentence_count_at_most, env_or_skip,
    expect_api_error_shape, first_content, force_tool_choice, multiply_numbers_tool, parse_jsonish,
    parse_tool_arguments, retry_live, sanitize_visible_text,
};
#[cfg(feature = "tool-runner")]
use super::common::{add_numbers_runner_tool, multiply_numbers_runner_tool};

#[derive(Debug, Deserialize)]
struct LocationAnswer {
    city: String,
    country: String,
}

fn live_client(api_key: String) -> Client {
    Client::builder()
        .provider(openai_rs::Provider::zhipu())
        .api_key(api_key)
        .timeout(Duration::from_secs(90))
        .max_retries(4)
        .build()
        .unwrap()
}

fn live_client_no_retry(api_key: String) -> Client {
    Client::builder()
        .provider(openai_rs::Provider::zhipu())
        .api_key(api_key)
        .timeout(Duration::from_secs(90))
        .max_retries(0)
        .build()
        .unwrap()
}

fn chat_model() -> String {
    std::env::var("ZHIPU_CHAT_MODEL").unwrap_or_else(|_| "glm-5".into())
}

fn responses_model() -> String {
    std::env::var("ZHIPU_RESPONSES_MODEL").unwrap_or_else(|_| chat_model())
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_completion_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "chat_completion_basic",
        LiveTier::Smoke,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat basic", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请用两句话介绍 Rust 的所有权模型。")
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("zhipu basic chat request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "zhipu basic output: request_id={}, text={text}",
        request_id.as_deref().unwrap_or("-")
    );

    assert!(!response.choices.is_empty());
    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    assert_contains_chinese(&text);
    assert_sentence_count_at_most(&text, 2);
    assert_contains_any(&text, &["所有权", "所有者", "借用", "内存安全"]);
    case.success(
        request_id.as_deref(),
        format!(
            "output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_completion_stream_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "chat_completion_stream_basic",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let (request_id, content) = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat stream", 3, || async {
            let stream = client
                .chat()
                .completions()
                .stream()
                .model(model.clone())
                .message_user("请只用一句话说明 Rust 为什么重视内存安全。")
                .send()
                .await?;
            let request_id = stream.meta().request_id.clone();
            let content = stream.final_content().await?;
            Ok((request_id, content.unwrap_or_default()))
        })
        .await
    })
    .await
    .expect("zhipu streaming chat request timed out")
    .unwrap();

    let raw_content = content.clone();
    let content = sanitize_visible_text(&content);
    eprintln!(
        "zhipu stream output: request_id={}, text={content}",
        request_id.as_deref().unwrap_or("-")
    );
    assert_no_markdown_fence(&raw_content);
    assert_no_think_block(&raw_content);
    assert_contains_chinese(&content);
    assert_sentence_count_at_most(&content, 1);
    assert_contains_any(&content, &["内存安全", "数据竞争", "悬垂指针", "性能"]);
    case.success(
        request_id.as_deref(),
        format!(
            "stream_output={content}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_structured_json_output() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "chat_structured_json_output",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat structured output", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .temperature(0.0)
                .message_user(
                    "从字符串 'Paris, France' 中提取 city 和 country，直接返回 JSON 对象，格式固定为 {\"city\":\"Paris\",\"country\":\"France\"}，不要 markdown，不要额外说明。",
                )
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("zhipu structured output request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "zhipu structured output: request_id={}, text={text}",
        request_id.as_deref().unwrap_or("-")
    );

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
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_tool_calling() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "chat_tool_calling",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat tool calling", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请调用 add_numbers 工具计算 2 + 3，不要直接给出答案。")
                .tool(add_numbers_tool())
                .tool(multiply_numbers_tool())
                .tool_choice(force_tool_choice("add_numbers"))
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("zhipu tool calling request timed out")
    .unwrap();

    let message = &response.choices[0].message;
    let request_id = response.meta.request_id.clone();
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    eprintln!(
        "zhipu tool call: request_id={}, name={}, arguments={}",
        request_id.as_deref().unwrap_or("-"),
        tool_call.function.name,
        tool_call.function.arguments
    );

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
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_responses_text_or_provider_error_shape() {
    let model = responses_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "responses_text_or_provider_error_shape",
        LiveTier::Slow,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let result = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu responses", 3, || async {
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
    .expect("zhipu responses request timed out");

    match result {
        Ok(response) => {
            let request_id = response.meta.request_id.clone();
            let raw_text = response.output_text().unwrap_or_default();
            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!(
                "zhipu responses output: request_id={}, text={text}",
                request_id.as_deref().unwrap_or("-")
            );
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
            let api = expect_api_error_shape(error, ProviderKind::Zhipu);
            eprintln!(
                "zhipu responses api error: request_id={}, status={}, kind={:?}, message={}",
                api.request_id.as_deref().unwrap_or("-"),
                api.status,
                api.kind,
                api.message
            );
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
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_invalid_model_error_shape() {
    let Some(case) = LiveCase::begin(
        "zhipu",
        "invalid_model_error_shape",
        LiveTier::Smoke,
        Some("definitely-not-a-real-zhipu-model"),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client_no_retry(api_key);

    let error = tokio::time::timeout(Duration::from_secs(90), async {
        let result = client
            .chat()
            .completions()
            .create()
            .model("definitely-not-a-real-zhipu-model")
            .message_user("hello")
            .send()
            .await;
        result.unwrap_err()
    })
    .await
    .expect("zhipu invalid model request timed out");

    let api = expect_api_error_shape(error, ProviderKind::Zhipu);
    eprintln!(
        "zhipu invalid model error: request_id={}, status={}, kind={:?}, message={}",
        api.request_id.as_deref().unwrap_or("-"),
        api.status,
        api.kind,
        api.message
    );
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

#[cfg(feature = "tool-runner")]
#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_run_tools_runner() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "zhipu",
        "chat_run_tools_runner",
        LiveTier::Slow,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        case.skip("ZHIPU_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let runner = tokio::time::timeout(Duration::from_secs(150), async {
        retry_live("zhipu run_tools", 3, || async {
            client
                .chat()
                .completions()
                .run_tools()
                .model(model.clone())
                .message_user(
                    "你必须调用 add_numbers 工具计算 2 + 3。在拿到工具结果后，只回复最终数字 5，不要附加解释。",
                )
                .register_tool(add_numbers_runner_tool())
                .register_tool(multiply_numbers_runner_tool())
                .max_rounds(4)
                .into_runner()
                .await
        })
        .await
    })
    .await
    .expect("zhipu run_tools request timed out")
    .unwrap();

    let final_text = sanitize_visible_text(runner.final_content().unwrap_or_default());
    eprintln!(
        "zhipu run_tools final output: tool_results={}, text={final_text}",
        runner.tool_results().len()
    );

    assert_eq!(runner.tool_results().len(), 1);
    assert_sentence_count_at_most(&final_text, 1);
    assert_contains_any(&final_text, &["5"]);
    case.success(
        None,
        format!(
            "tool_results={}; final_content={final_text}",
            runner.tool_results().len()
        ),
    );
}
