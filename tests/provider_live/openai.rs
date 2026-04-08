use std::time::Duration;

use openai_rs::{ApiErrorKind, Client, ProviderKind};
use serde::Deserialize;
use serial_test::serial;

#[cfg(feature = "tool-runner")]
use super::common::add_numbers_runner_tool;
use super::common::{
    LiveCase, LiveTier, add_numbers_tool, assert_contains_any, assert_no_markdown_fence,
    assert_no_think_block, assert_sentence_count_at_most, env_or_skip, expect_api_error_shape,
    first_content, force_tool_choice, parse_jsonish, parse_tool_arguments, retry_live,
    sanitize_visible_text,
};

#[derive(Debug, Deserialize)]
struct OwnerAnswer {
    language: String,
    feature: String,
}

fn live_client(api_key: String) -> Client {
    Client::builder()
        .api_key(api_key)
        .timeout(Duration::from_secs(60))
        .max_retries(3)
        .build()
        .unwrap()
}

fn live_client_no_retry(api_key: String) -> Client {
    Client::builder()
        .api_key(api_key)
        .timeout(Duration::from_secs(60))
        .max_retries(0)
        .build()
        .unwrap()
}

fn chat_model() -> String {
    std::env::var("OPENAI_CHAT_MODEL").unwrap_or_else(|_| "gpt-5.4-mini".into())
}

fn responses_model() -> String {
    std::env::var("OPENAI_RESPONSES_MODEL").unwrap_or_else(|_| "gpt-5.4-mini".into())
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_chat_completion_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "chat_completion_basic",
        LiveTier::Smoke,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("openai chat basic", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("Explain Rust ownership in one sentence.")
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("openai basic chat request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "openai basic output: request_id={}, text={text}",
        request_id.as_deref().unwrap_or("-")
    );

    assert!(!response.choices.is_empty());
    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    assert_sentence_count_at_most(&text, 1);
    assert_contains_any(
        &text.to_ascii_lowercase(),
        &["ownership", "borrow", "memory"],
    );
    case.success(
        request_id.as_deref(),
        format!(
            "output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_chat_completion_stream_basic() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "chat_completion_stream_basic",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let (request_id, content) = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("openai chat stream", 3, || async {
            let stream = client
                .chat()
                .completions()
                .stream()
                .model(model.clone())
                .message_user("Reply with one short sentence about why Rust avoids data races.")
                .send()
                .await?;
            let request_id = stream.meta().request_id.clone();
            let content = stream.final_content().await?;
            Ok((request_id, content.unwrap_or_default()))
        })
        .await
    })
    .await
    .expect("openai streaming chat request timed out")
    .unwrap();

    let raw_content = content.clone();
    let content = sanitize_visible_text(&content);
    eprintln!(
        "openai stream output: request_id={}, text={content}",
        request_id.as_deref().unwrap_or("-")
    );
    assert_no_markdown_fence(&raw_content);
    assert_no_think_block(&raw_content);
    assert_sentence_count_at_most(&content, 1);
    assert_contains_any(
        &content.to_ascii_lowercase(),
        &["data race", "memory", "ownership", "borrow"],
    );
    case.success(
        request_id.as_deref(),
        format!(
            "stream_output={content}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_chat_structured_json_output() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "chat_structured_json_output",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("openai chat structured output", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .temperature(0.0)
                .message_user(
                    "Extract JSON from 'Rust, ownership'. Return only {\"language\":\"Rust\",\"feature\":\"ownership\"}.",
                )
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("openai structured output request timed out")
    .unwrap();

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "openai structured output: request_id={}, text={text}",
        request_id.as_deref().unwrap_or("-")
    );

    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    let parsed: OwnerAnswer = parse_jsonish(&text).unwrap();
    assert_eq!(parsed.language, "Rust");
    assert_eq!(parsed.feature, "ownership");
    case.success(
        request_id.as_deref(),
        format!(
            "structured_output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_chat_tool_calling() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "chat_tool_calling",
        LiveTier::Extended,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("openai chat tool calling", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model.clone())
                .message_user("Call add_numbers to compute 2 + 3. Do not answer directly.")
                .tool(add_numbers_tool())
                .tool_choice(force_tool_choice("add_numbers"))
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("openai tool calling request timed out")
    .unwrap();

    let request_id = response.meta.request_id.clone();
    let message = &response.choices[0].message;
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    eprintln!(
        "openai tool call: request_id={}, tool={}, arguments={arguments}",
        request_id.as_deref().unwrap_or("-"),
        tool_call.function.name
    );

    assert_eq!(tool_call.function.name, "add_numbers");
    assert_eq!(arguments["a"].as_i64(), Some(2));
    assert_eq!(arguments["b"].as_i64(), Some(3));
    case.success(
        request_id.as_deref(),
        format!(
            "tool_name={}; arguments={arguments}; request_id={}",
            tool_call.function.name,
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_responses_text() {
    let model = responses_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "responses_text",
        LiveTier::Smoke,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("openai responses", 3, || async {
            client
                .responses()
                .create()
                .model(model.clone())
                .input_text("Reply with only OK.")
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("openai responses request timed out")
    .unwrap();

    let request_id = response.meta.request_id.clone();
    let raw_text = response.output_text().unwrap_or_default();
    let text = response
        .output_text()
        .map(|value| sanitize_visible_text(&value))
        .unwrap_or_default();
    eprintln!(
        "openai responses output: request_id={}, text={text}",
        request_id.as_deref().unwrap_or("-")
    );

    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    assert_contains_any(&text, &["OK"]);
    case.success(
        request_id.as_deref(),
        format!(
            "responses_output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_invalid_model_error_shape() {
    let Some(case) = LiveCase::begin(
        "openai",
        "invalid_model_error_shape",
        LiveTier::Smoke,
        Some("definitely-not-a-real-openai-model"),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client_no_retry(api_key);

    let error = tokio::time::timeout(Duration::from_secs(90), async {
        let result = client
            .chat()
            .completions()
            .create()
            .model("definitely-not-a-real-openai-model")
            .message_user("hello")
            .send()
            .await;
        result.unwrap_err()
    })
    .await
    .expect("openai invalid model request timed out");

    let api = expect_api_error_shape(error, ProviderKind::OpenAI);
    eprintln!(
        "openai invalid model error: request_id={}, status={}, kind={:?}, message={}",
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
#[ignore = "requires OPENAI_API_KEY"]
#[serial(provider_live)]
async fn test_live_openai_chat_run_tools_runner() {
    let model = chat_model();
    let Some(case) = LiveCase::begin(
        "openai",
        "chat_run_tools_runner",
        LiveTier::Slow,
        Some(model.clone()),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("OPENAI_API_KEY") else {
        case.skip("OPENAI_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let runner = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("openai run_tools", 3, || async {
            client
                .chat()
                .completions()
                .run_tools()
                .model(model.clone())
                .message_user(
                    "You must call add_numbers to compute 2 + 3. After the tool result, reply with only 5.",
                )
                .register_tool(add_numbers_runner_tool())
                .max_rounds(4)
                .into_runner()
                .await
        })
        .await
    })
    .await
    .expect("openai run_tools request timed out")
    .unwrap();

    let final_text = sanitize_visible_text(runner.final_content().unwrap_or_default());
    eprintln!(
        "openai run_tools final output: tool_results={}, text={final_text}",
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
