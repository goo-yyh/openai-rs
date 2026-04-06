use std::time::Duration;

use openai_rs::{ApiErrorKind, Client, Error, Model, ProviderKind};
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

    let mut candidates = preferred
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
                .send()
                .await
        })
        .await;
        match probe {
            Ok(_) => {
                eprintln!("zenmux chosen responses model: {candidate}");
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
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_completion_with_discovered_model() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zenmux chat completion request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            eprintln!("skip zenmux chat completion because credential lacks model access: {error}");
            return;
        }
        Err(error) => panic!("zenmux chat completion failed: {error}"),
    };

    let text = first_visible_content(&response);
    eprintln!("zenmux basic output: {text}");

    assert!(!response.choices.is_empty());
    assert_contains_any(&text, &["OK"]);
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_structured_json_output() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zenmux structured output request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            eprintln!(
                "skip zenmux structured output because credential lacks model access: {error}"
            );
            return;
        }
        Err(error) => panic!("zenmux structured output failed: {error}"),
    };

    let text = first_visible_content(&response);
    eprintln!("zenmux structured output: {text}");

    let parsed: LocationAnswer = parse_jsonish(&text).unwrap();
    assert_eq!(parsed.city, "Paris");
    assert_eq!(parsed.country, "France");
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_chat_tool_calling() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
                .tool_choice(force_tool_choice("add_numbers"))
                .send()
                .await
        })
        .await
    })
    .await
    .expect("zenmux tool calling request timed out");

    let response = match response {
        Ok(response) => response,
        Err(error) if should_skip_zenmux_permission(&error) => {
            eprintln!("skip zenmux tool calling because credential lacks model access: {error}");
            return;
        }
        Err(error) => panic!("zenmux tool calling failed: {error}"),
    };

    let message = &response.choices[0].message;
    assert_eq!(message.tool_calls.len(), 1);

    let tool_call = &message.tool_calls[0];
    let arguments = parse_tool_arguments(tool_call);
    eprintln!(
        "zenmux tool call: name={}, arguments={}",
        tool_call.function.name, tool_call.function.arguments
    );

    assert_eq!(tool_call.function.name, "add_numbers");
    assert_eq!(arguments["a"], 2);
    assert_eq!(arguments["b"], 3);
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_text_output() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
                        .send()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses request timed out")
            .unwrap();

            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!("zenmux responses output: {text}");
            assert_contains_any(&text, &["OK"]);
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            eprintln!(
                "zenmux responses model discovery error: status={}, kind={:?}, message={}",
                api.status, api.kind, api.message
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
                return;
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_responses_structured_json_output() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
                        .send()
                        .await
                })
                .await
            })
            .await
            .expect("zenmux responses structured output request timed out")
            .unwrap();

            let text = response
                .output_text()
                .map(|value| sanitize_visible_text(&value))
                .unwrap_or_default();
            eprintln!("zenmux responses structured output: {text}");

            let parsed: LocationAnswer = parse_jsonish(&text).unwrap();
            assert_eq!(parsed.city, "Paris");
            assert_eq!(parsed.country, "France");
        }
        Err(error) => {
            let api = expect_api_error_shape(error, ProviderKind::ZenMux);
            eprintln!(
                "zenmux responses structured discovery error: status={}, kind={:?}, message={}",
                api.status, api.kind, api.message
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
                return;
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires ZENMUX_API_KEY"]
#[serial(provider_live)]
async fn test_live_zenmux_invalid_model_error_shape() {
    let Some(api_key) = env_or_skip("ZENMUX_API_KEY") else {
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
        "zenmux invalid model error: status={}, kind={:?}, message={}",
        api.status, api.kind, api.message
    );
    assert!(matches!(
        api.kind,
        ApiErrorKind::PermissionDenied
            | ApiErrorKind::BadRequest
            | ApiErrorKind::NotFound
            | ApiErrorKind::UnprocessableEntity
            | ApiErrorKind::Unknown
    ));
}
