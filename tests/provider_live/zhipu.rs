use std::time::Duration;

use openai_rs::{ApiErrorKind, Client, ProviderKind};
use serde::Deserialize;
use serial_test::serial;

use super::common::{
    add_numbers_tool, assert_contains_any, env_or_skip, expect_api_error_shape,
    first_visible_content, force_tool_choice, parse_jsonish, parse_tool_arguments, retry_live,
    sanitize_visible_text,
};

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
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = live_client(api_key);
    let model = chat_model();

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat basic", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请用两句话介绍 Rust 的所有权模型。")
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zhipu basic chat request timed out")
    .unwrap();

    let text = first_visible_content(&response);
    eprintln!("zhipu basic output: {text}");

    assert!(!response.choices.is_empty());
    assert_contains_any(&text, &["所有权", "所有者", "借用", "内存安全"]);
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_completion_stream_basic() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = live_client(api_key);
    let model = chat_model();

    let content = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat stream", 3, || async {
            client
                .chat()
                .completions()
                .stream()
                .model(model.clone())
                .message_user("请只用一句话说明 Rust 为什么重视内存安全。")
                .send()
                .await?
                .final_content()
                .await
        })
        .await
    })
    .await
    .expect("zhipu streaming chat request timed out")
    .unwrap()
    .unwrap_or_default();

    let content = sanitize_visible_text(&content);
    eprintln!("zhipu stream output: {content}");
    assert_contains_any(&content, &["内存安全", "数据竞争", "悬垂指针", "性能"]);
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_structured_json_output() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = live_client(api_key);
    let model = chat_model();

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
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zhipu structured output request timed out")
    .unwrap();

    let text = first_visible_content(&response);
    eprintln!("zhipu structured output: {text}");

    let parsed: LocationAnswer = parse_jsonish(&text).unwrap();
    assert_eq!(parsed.city, "Paris");
    assert_eq!(parsed.country, "France");
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_chat_tool_calling() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = live_client(api_key);
    let model = chat_model();

    let response = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu chat tool calling", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("请调用 add_numbers 工具计算 2 + 3，不要直接给出答案。")
                .tool(add_numbers_tool())
                .tool_choice(force_tool_choice("add_numbers"))
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zhipu tool calling request timed out")
    .unwrap();

    let message = &response.choices[0].message;
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    eprintln!(
        "zhipu tool call: name={}, arguments={}",
        tool_call.function.name, tool_call.function.arguments
    );

    assert_eq!(tool_call.function.name, "add_numbers");
    assert_eq!(arguments["a"], 2);
    assert_eq!(arguments["b"], 3);
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_responses_text_or_provider_error_shape() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
        return;
    };

    let client = live_client(api_key);
    let model = responses_model();

    let result = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zhipu responses", 3, || async {
            client
                .responses()
                .create()
                .model(model.clone())
                .input_text("请只回答 OK。")
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zhipu responses request timed out");

    match result {
        Ok(response) => {
            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!("zhipu responses output: {text}");
            assert_contains_any(&text, &["OK", "好", "可以"]);
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::Zhipu);
            eprintln!(
                "zhipu responses api error: status={}, kind={:?}, message={}",
                api.status, api.kind, api.message
            );
            assert!(matches!(
                api.kind,
                ApiErrorKind::BadRequest
                    | ApiErrorKind::NotFound
                    | ApiErrorKind::UnprocessableEntity
                    | ApiErrorKind::Unknown
                    | ApiErrorKind::InternalServer
            ));
        }
    }
}

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY"]
#[serial(provider_live)]
async fn test_live_zhipu_invalid_model_error_shape() {
    let Some(api_key) = env_or_skip("ZHIPU_API_KEY") else {
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
        "zhipu invalid model error: status={}, kind={:?}, message={}",
        api.status, api.kind, api.message
    );
    assert!(matches!(
        api.kind,
        ApiErrorKind::BadRequest
            | ApiErrorKind::NotFound
            | ApiErrorKind::UnprocessableEntity
            | ApiErrorKind::Unknown
    ));
}
