use std::time::Duration;

use futures_util::StreamExt;
use openai_rs::{ApiErrorKind, Client, Error, Model, ProviderKind, ResponseRuntimeEvent};
use serde::Deserialize;
use serial_test::serial;

use super::common::{
    LiveCase, LiveTier, add_numbers_tool, assert_contains_any, assert_no_markdown_fence,
    assert_no_think_block, assert_sentence_count_at_most, env_or_skip, expect_api_error_shape,
    first_content, force_tool_choice, multiply_numbers_tool, parse_jsonish, parse_tool_arguments,
    read_cached_model, retry_live, sanitize_visible_text, write_cached_model,
    zenmux_responses_cache_ttl,
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
        .provider(openai_rs::Provider::zenmux())
        .api_key(api_key)
        .timeout(Duration::from_secs(60))
        .max_retries(4)
        .build()
        .unwrap()
}

fn should_skip_zenmux_permission(error: &Error) -> bool {
    matches!(error, Error::Api(api) if api.kind == ApiErrorKind::PermissionDenied)
}

fn preferred_zenmux_text_model(models: &[Model]) -> Option<String> {
    if let Some(model) = std::env::var("ZENMUX_CHAT_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(model);
    }

    let preferred = [
        "openai/gpt-4.1-nano",
        "openai/gpt-4o-mini",
        "openai/gpt-4.1-mini",
        "google/gemini-2.0-flash",
    ];

    preferred
        .iter()
        .find_map(|target| {
            models
                .iter()
                .find(|model| model.id == *target)
                .map(|model| model.id.clone())
        })
        .or_else(|| {
            models
                .iter()
                .find(|model| {
                    let id = model.id.to_ascii_lowercase();
                    model.id.contains('/')
                        && !id.contains("embedding")
                        && !id.contains("rerank")
                        && !id.contains("whisper")
                        && !id.contains("tts")
                        && !id.contains("image")
                        && !id.contains("vision")
                        && !id.contains("audio")
                })
                .map(|model| model.id.clone())
        })
}

async fn resolve_model(client: &Client) -> String {
    if let Some(model) = std::env::var("ZENMUX_CHAT_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return model;
    }

    let page = retry_live("zenmux models.list", 3, || async {
        client.models().list().send().await
    })
    .await
    .unwrap();

    preferred_zenmux_text_model(&page.data).expect("no suitable zenmux chat model")
}

async fn resolve_responses_model(client: &Client) -> Result<String, Error> {
    if let Some(model) = std::env::var("ZENMUX_RESPONSES_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(model);
    }

    if let Some(model) = read_cached_model("zenmux-responses-model", zenmux_responses_cache_ttl()) {
        eprintln!("zenmux cached responses model: {model}");
        return Ok(model);
    }

    let page = retry_live("zenmux models.list", 3, || async {
        client.models().list().send().await
    })
    .await
    .unwrap();

    let preferred = [
        "openai/gpt-4.1-mini",
        "openai/gpt-4.1",
        "openai/gpt-4o-mini",
        "openai/gpt-4.1-nano",
        "openai/gpt-4o",
    ];

    let mut candidates: Vec<String> = preferred
        .iter()
        .filter_map(|target| {
            page.data
                .iter()
                .find(|model| model.id == *target)
                .map(|model| model.id.clone())
        })
        .collect::<Vec<_>>();

    candidates.extend(page.data.iter().filter_map(|model| {
        let id = model.id.to_ascii_lowercase();
        if model.id.contains('/')
            && id.contains("openai/")
            && !id.contains("audio")
            && !id.contains("image")
            && !id.contains("embedding")
            && !id.contains("rerank")
        {
            Some(model.id.clone())
        } else {
            None
        }
    }));

    candidates.sort();
    candidates.dedup();

    let mut last_error = None;

    for candidate in candidates {
        let probe = retry_live("zenmux responses probe", 2, || async {
            client
                .responses()
                .create()
                .model(candidate.clone())
                .input_text("Reply with OK.")
                .max_retries(0)
                .send_with_meta()
                .await
        })
        .await;
        match probe {
            Ok(response) => {
                eprintln!(
                    "zenmux chosen responses model: {} (request_id={})",
                    candidate,
                    response.meta.request_id.as_deref().unwrap_or("-")
                );
                write_cached_model("zenmux-responses-model", &candidate);
                return Ok(candidate);
            }
            Err(Error::Api(api))
                if api.message.contains("/v1/responses")
                    || api.message.contains("No provider available")
                    || matches!(api.kind, ApiErrorKind::BadRequest | ApiErrorKind::NotFound) =>
            {
                eprintln!(
                    "zenmux responses probe rejected model {}: {}",
                    candidate, api.message
                );
                last_error = Some(Error::Api(api));
            }
            Err(error) => {
                eprintln!("zenmux responses probe failed for {}: {}", candidate, error);
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        Error::InvalidConfig(
            "no zenmux model supports /v1/responses in the probed candidate set".into(),
        )
    }))
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_models_list() {
    let Some(case) = LiveCase::begin("zenmux", "models_list", LiveTier::Smoke, None::<String>)
    else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let page = tokio::time::timeout(Duration::from_secs(60), async {
        retry_live("zenmux models.list", 3, || async {
            client.models().list().send().await
        })
        .await
    })
    .await
    .expect("zenmux models list request timed out")
    .unwrap();

    let preview = page
        .data
        .iter()
        .take(5)
        .map(|model| model.id.clone())
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!("zenmux models count={}, preview={preview}", page.data.len());

    assert!(!page.data.is_empty());
    assert!(page.data.iter().all(|model| !model.id.trim().is_empty()));
    case.success(
        None,
        format!("models_count={}; preview={preview}", page.data.len()),
    );
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_completion_with_discovered_model() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "chat_completion_with_discovered_model",
        LiveTier::Smoke,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_id = resolve_model(&client).await;
    eprintln!("zenmux chosen model: {model_id}");

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("zenmux chat completion", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model_id.clone())
                .message_user("请只回复 OK。")
                .send_with_meta()
                .await
        })
        .await
    })
    .await
    .expect("zenmux chat completion request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            let reason = format!("credential lacks model access: {error}");
            eprintln!("skip zenmux chat completion because {reason}");
            case.skip(reason);
            return;
        }
        Err(error) => panic!("zenmux chat completion failed: {error}"),
    };

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "zenmux basic output: model={}, request_id={}, text={text}",
        model_id,
        request_id.as_deref().unwrap_or("-")
    );

    assert!(!response.choices.is_empty());
    assert_no_markdown_fence(&raw_text);
    assert_no_think_block(&raw_text);
    assert_sentence_count_at_most(&text, 1);
    assert_contains_any(&text, &["OK"]);
    case.success(
        request_id.as_deref(),
        format!(
            "model={model_id}; output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_structured_json_output() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "chat_structured_json_output",
        LiveTier::Extended,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_id = resolve_model(&client).await;

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("zenmux chat structured output", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model_id.clone())
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
    .expect("zenmux structured output request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            let reason = format!("credential lacks model access: {error}");
            eprintln!("skip zenmux structured output because {reason}");
            case.skip(reason);
            return;
        }
        Err(error) => panic!("zenmux structured output failed: {error}"),
    };

    let raw_text = first_content(&response);
    let text = sanitize_visible_text(&raw_text);
    let request_id = response.meta.request_id.clone();
    eprintln!(
        "zenmux structured output: model={}, request_id={}, text={text}",
        model_id,
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
            "model={model_id}; structured_output={text}; request_id={}",
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_tool_calling() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "chat_tool_calling",
        LiveTier::Extended,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_id = resolve_model(&client).await;

    let response = tokio::time::timeout(Duration::from_secs(90), async {
        retry_live("zenmux chat tool calling", 3, || async {
            client
                .chat()
                .completions()
                .create()
                .model(model_id.clone())
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
    .expect("zenmux tool calling request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            let reason = format!("credential lacks model access: {error}");
            eprintln!("skip zenmux tool calling because {reason}");
            case.skip(reason);
            return;
        }
        Err(error) => panic!("zenmux tool calling failed: {error}"),
    };

    let message = &response.choices[0].message;
    let request_id = response.meta.request_id.clone();
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    eprintln!(
        "zenmux tool call: model={}, request_id={}, name={}, arguments={}",
        model_id,
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
            "model={model_id}; tool={} args={}; request_id={}",
            tool_call.function.name,
            tool_call.function.arguments,
            request_id.as_deref().unwrap_or("-")
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_text_output() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "responses_text_output",
        LiveTier::Extended,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_result = resolve_responses_model(&client).await;

    match model_result {
        Ok(model_id) => {
            let response = tokio::time::timeout(Duration::from_secs(90), async {
                retry_live("zenmux responses text", 3, || async {
                    client
                        .responses()
                        .create()
                        .model(model_id.clone())
                        .input_text("请只回答 OK。")
                        .send_with_meta()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses request timed out");

            let response = match response {
                Ok(response) => response,
                Err(error) if should_skip_zenmux_permission(&error) => {
                    case.skip("credential lacks responses access");
                    return;
                }
                Err(error) => panic!("zenmux responses text failed: {error}"),
            };

            let request_id = response.meta.request_id.clone();
            let raw_text = response.output_text().unwrap_or_default();
            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!(
                "zenmux responses output: model={}, request_id={}, text={text}",
                model_id,
                request_id.as_deref().unwrap_or("-")
            );
            assert_no_markdown_fence(&raw_text);
            assert_no_think_block(&raw_text);
            assert_sentence_count_at_most(&text, 1);
            assert_contains_any(&text, &["OK"]);
            case.success(
                request_id.as_deref(),
                format!(
                    "model={model_id}; responses_output={text}; request_id={}",
                    request_id.as_deref().unwrap_or("-")
                ),
            );
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            eprintln!(
                "zenmux responses model discovery error: request_id={}, status={}, kind={:?}, message={}",
                api.request_id.as_deref().unwrap_or("-"),
                api.status,
                api.kind,
                api.message
            );
            assert!(matches!(
                api.kind,
                ApiErrorKind::PermissionDenied
                    | ApiErrorKind::BadRequest
                    | ApiErrorKind::NotFound
                    | ApiErrorKind::Unknown
                    | ApiErrorKind::InternalServer
            ));
            if api.kind == ApiErrorKind::PermissionDenied {
                eprintln!("skip zenmux responses text because credential lacks responses access");
                case.skip("credential lacks responses access");
                return;
            }
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
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_structured_json_output() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "responses_structured_json_output",
        LiveTier::Slow,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_result = resolve_responses_model(&client).await;

    match model_result {
        Ok(model_id) => {
            let response = tokio::time::timeout(Duration::from_secs(90), async {
                retry_live("zenmux responses structured output", 3, || async {
                    client
                        .responses()
                        .create()
                        .model(model_id.clone())
                        .input_text(
                            "从字符串 'Paris, France' 中提取 city 和 country，直接返回 JSON 对象，格式固定为 {\"city\":\"Paris\",\"country\":\"France\"}，不要 markdown，不要额外说明。",
                        )
                        .send_with_meta()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses structured output request timed out");

            let response = match response {
                Ok(response) => response,
                Err(error) if should_skip_zenmux_permission(&error) => {
                    case.skip("credential lacks responses access");
                    return;
                }
                Err(error) => panic!("zenmux responses structured output failed: {error}"),
            };

            let request_id = response.meta.request_id.clone();
            let raw_text = response.output_text().unwrap_or_default();
            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!(
                "zenmux responses structured output: model={}, request_id={}, text={text}",
                model_id,
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
                    "model={model_id}; responses_structured_output={text}; request_id={}",
                    request_id.as_deref().unwrap_or("-")
                ),
            );
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            eprintln!(
                "zenmux responses structured discovery error: request_id={}, status={}, kind={:?}, message={}",
                api.request_id.as_deref().unwrap_or("-"),
                api.status,
                api.kind,
                api.message
            );
            assert!(matches!(
                api.kind,
                ApiErrorKind::PermissionDenied
                    | ApiErrorKind::BadRequest
                    | ApiErrorKind::NotFound
                    | ApiErrorKind::Unknown
                    | ApiErrorKind::InternalServer
            ));
            if api.kind == ApiErrorKind::PermissionDenied {
                eprintln!(
                    "skip zenmux responses structured output because credential lacks responses access"
                );
                case.skip("credential lacks responses access");
                return;
            }
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
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_stream_text_output() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "responses_stream_text_output",
        LiveTier::Slow,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_result = resolve_responses_model(&client).await;

    match model_result {
        Ok(model_id) => {
            let stream = tokio::time::timeout(Duration::from_secs(90), async {
                retry_live("zenmux responses stream text", 3, || async {
                    client
                        .responses()
                        .stream()
                        .model(model_id.clone())
                        .input_text("请只回答 OK。")
                        .send_events()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses stream text request timed out");

            let mut stream = match stream {
                Ok(stream) => stream,
                Err(error) if should_skip_zenmux_permission(&error) => {
                    case.skip("credential lacks responses stream access");
                    return;
                }
                Err(error) => panic!("zenmux responses stream text failed: {error}"),
            };

            let request_id = stream.meta().request_id.clone();
            let mut saw_output_delta = false;
            let mut saw_completed = false;
            while let Some(event) = stream.next().await {
                match event.unwrap() {
                    ResponseRuntimeEvent::OutputTextDelta(_) => saw_output_delta = true,
                    ResponseRuntimeEvent::Completed(_) => saw_completed = true,
                    _ => {}
                }
            }

            let text = sanitize_visible_text(stream.output_text());
            let final_response = stream.snapshot();
            eprintln!(
                "zenmux responses stream text: model={}, request_id={}, text={text}",
                model_id,
                request_id.as_deref().unwrap_or("-")
            );

            assert!(
                saw_output_delta,
                "expected at least one output_text.delta event"
            );
            assert!(saw_completed, "expected a completed event");
            assert_sentence_count_at_most(&text, 1);
            assert_contains_any(&text, &["OK"]);
            if let Some(response) = final_response {
                assert_eq!(response.output_text().as_deref(), Some(text.as_str()));
            }
            case.success(
                request_id.as_deref(),
                format!(
                    "model={model_id}; stream_text={text}; saw_output_delta={saw_output_delta}; saw_completed={saw_completed}; request_id={}",
                    request_id.as_deref().unwrap_or("-")
                ),
            );
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            if api.kind == ApiErrorKind::PermissionDenied {
                case.skip("credential lacks responses stream access");
                return;
            }
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
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_stream_function_call_arguments() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "responses_stream_function_call_arguments",
        LiveTier::Slow,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_result = resolve_responses_model(&client).await;

    match model_result {
        Ok(model_id) => {
            let stream = tokio::time::timeout(Duration::from_secs(90), async {
                retry_live("zenmux responses stream tool args", 3, || async {
                    client
                        .responses()
                        .stream()
                        .model(model_id.clone())
                        .input_text("请调用 add_numbers 工具计算 2 + 3，不要直接给出答案。")
                        .tool(add_numbers_tool())
                        .send_events()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses stream tool arguments request timed out");

            let mut stream = match stream {
                Ok(stream) => stream,
                Err(error) if should_skip_zenmux_permission(&error) => {
                    case.skip("credential lacks responses stream access");
                    return;
                }
                Err(error) => panic!("zenmux responses stream tool args failed: {error}"),
            };

            let request_id = stream.meta().request_id.clone();
            let mut saw_args_delta = false;
            let mut saw_completed = false;
            let mut latest_arguments = String::new();
            while let Some(event) = stream.next().await {
                match event.unwrap() {
                    ResponseRuntimeEvent::FunctionCallArgumentsDelta(event) => {
                        saw_args_delta = true;
                        latest_arguments = event.snapshot;
                    }
                    ResponseRuntimeEvent::Completed(_) => saw_completed = true,
                    _ => {}
                }
            }

            if latest_arguments.is_empty()
                && let Some(arguments) = stream.function_arguments().values().next()
            {
                latest_arguments = arguments.clone();
            }

            eprintln!(
                "zenmux responses stream tool args: model={}, request_id={}, arguments={}",
                model_id,
                request_id.as_deref().unwrap_or("-"),
                latest_arguments
            );

            assert!(
                saw_args_delta,
                "expected function_call_arguments.delta events"
            );
            assert!(saw_completed, "expected a completed event");
            let parsed: serde_json::Value = parse_jsonish(&latest_arguments).unwrap();
            assert_eq!(parsed["a"], 2);
            assert_eq!(parsed["b"], 3);
            case.success(
                request_id.as_deref(),
                format!(
                    "model={model_id}; function_arguments={latest_arguments}; saw_completed={saw_completed}; request_id={}",
                    request_id.as_deref().unwrap_or("-")
                ),
            );
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            if api.kind == ApiErrorKind::PermissionDenied {
                case.skip("credential lacks responses stream access");
                return;
            }
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

#[cfg(feature = "tool-runner")]
#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_run_tools_runner() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "chat_run_tools_runner",
        LiveTier::Slow,
        None::<String>,
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);
    let model_id = resolve_model(&client).await;
    let runner = tokio::time::timeout(Duration::from_secs(120), async {
        retry_live("zenmux run_tools", 3, || async {
            client
                .chat()
                .completions()
                .run_tools()
                .model(model_id.clone())
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
    .expect("zenmux run_tools request timed out");

    let runner = match runner {
        Ok(runner) => runner,
        Err(error) if should_skip_zenmux_permission(&error) => {
            case.skip("credential lacks model access");
            return;
        }
        Err(error) => panic!("zenmux run_tools failed: {error}"),
    };

    let final_text = sanitize_visible_text(runner.final_content().unwrap_or_default());
    eprintln!(
        "zenmux run_tools final output: model={}, tool_results={}, text={final_text}",
        model_id,
        runner.tool_results().len()
    );

    assert_eq!(runner.tool_results().len(), 1);
    assert_sentence_count_at_most(&final_text, 1);
    assert_contains_any(&final_text, &["5"]);
    case.success(
        None,
        format!(
            "model={model_id}; tool_results={}; final_content={final_text}",
            runner.tool_results().len()
        ),
    );
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_invalid_model_error_shape() {
    let Some(case) = LiveCase::begin(
        "zenmux",
        "invalid_model_error_shape",
        LiveTier::Smoke,
        Some("definitely-not-a-real-provider/definitely-not-a-real-model"),
    ) else {
        return;
    };
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
        case.skip("ZENMUX_API_KEY missing");
        return;
    };

    let client = live_client(api_key);

    let error = tokio::time::timeout(Duration::from_secs(90), async {
        client
            .responses()
            .create()
            .model("definitely-not-a-real-provider/definitely-not-a-real-model")
            .input_text("hello")
            .max_retries(0)
            .send()
            .await
            .unwrap_err()
    })
    .await
    .expect("zenmux invalid model request timed out");

    let api = expect_api_error_shape(error, ProviderKind::ZenMux);
    eprintln!(
        "zenmux invalid model error: request_id={}, status={}, kind={:?}, message={}",
        api.request_id.as_deref().unwrap_or("-"),
        api.status,
        api.kind,
        api.message
    );
    assert!(matches!(
        api.kind,
        ApiErrorKind::PermissionDenied
            | ApiErrorKind::BadRequest
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
