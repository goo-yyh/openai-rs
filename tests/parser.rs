use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{ChatCompletionChunk, ChatCompletionRuntimeEvent, Client, ResponseRuntimeEvent};

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_parse_structured_output() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_2",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "{\"name\":\"rust\",\"level\":5}",
                    "tool_calls": [],
                    "reasoning_details": []
                }
            }]
        })))
        .mount(&server)
        .await;

    #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    struct Skill {
        name: String,
        level: u8,
    }

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let parsed = client
        .chat()
        .completions()
        .parse::<Skill>()
        .model("gpt-5.4")
        .message_user("return json")
        .send()
        .await
        .unwrap();

    assert_eq!(parsed.parsed.name, "rust");
    assert_eq!(parsed.parsed.level, 5);
}

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_parse_tool_arguments_when_chat_content_is_empty() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_tool_parse",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "extract_answer",
                            "arguments": "{\"answer\":\"ok\"}"
                        }
                    }],
                    "reasoning_details": []
                }
            }]
        })))
        .mount(&server)
        .await;

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    struct Answer {
        answer: String,
    }

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let parsed = client
        .chat()
        .completions()
        .parse::<Answer>()
        .model("gpt-5.4")
        .message_user("return tool args")
        .send()
        .await
        .unwrap();

    assert_eq!(parsed.parsed.answer, "ok");
}

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_fail_parse_when_finish_reason_is_length() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_length",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "length",
                "message": {
                    "role": "assistant",
                    "content": "{\"answer\":\"partial\"}",
                    "tool_calls": [],
                    "reasoning_details": []
                }
            }]
        })))
        .mount(&server)
        .await;

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    struct Answer {
        answer: String,
    }

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let error = client
        .chat()
        .completions()
        .parse::<Answer>()
        .model("gpt-5.4")
        .message_user("return json")
        .send()
        .await
        .unwrap_err();

    assert!(matches!(error, openai_rs::Error::LengthFinishReason(_)));
}

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_fail_parse_when_finish_reason_is_content_filter() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_content_filter",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "content_filter",
                "message": {
                    "role": "assistant",
                    "content": "{\"answer\":\"blocked\"}",
                    "tool_calls": [],
                    "reasoning_details": []
                }
            }]
        })))
        .mount(&server)
        .await;

    #[allow(dead_code)]
    #[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
    struct Answer {
        answer: String,
    }

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let error = client
        .chat()
        .completions()
        .parse::<Answer>()
        .model("gpt-5.4")
        .message_user("return json")
        .send()
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        openai_rs::Error::ContentFilterFinishReason(_)
    ));
}

#[cfg(feature = "structured-output")]
#[test]
fn test_should_parse_markdown_wrapped_json_payload() {
    #[derive(Debug, serde::Deserialize)]
    struct Answer {
        answer: String,
    }

    let parsed: Answer = openai_rs::parse_json_payload(
        r#"
```json
{"answer":"ok"}
```
        "#,
    )
    .unwrap();

    assert_eq!(parsed.answer, "ok");
}

#[test]
fn test_should_extract_chat_chunk_delta_helpers() {
    let chunk: ChatCompletionChunk = serde_json::from_value(json!({
        "id": "chatcmpl_chunk",
        "object": "chat.completion.chunk",
        "model": "gpt-5.4",
        "choices": [{
            "index": 0,
            "delta": {
                "content": "hel",
                "refusal": "no",
                "tool_calls": [{
                    "index": 0,
                    "function": {
                        "name": "lookup",
                        "arguments": "{\"city\":\"Sha"
                    }
                }]
            },
            "logprobs": {
                "content": [{"token":"hel"}],
                "refusal": [{"token":"no"}]
            }
        }]
    }))
    .unwrap();

    assert_eq!(chunk.content_deltas()[0].delta, "hel");
    assert_eq!(chunk.refusal_deltas()[0].delta, "no");
    assert_eq!(chunk.tool_argument_deltas()[0].delta, "{\"city\":\"Sha");
    assert_eq!(chunk.logprobs_content_deltas()[0].values[0].token, "hel");
    assert_eq!(chunk.logprobs_refusal_deltas()[0].values[0].token, "no");
}

#[tokio::test]
async fn test_should_emit_chat_runtime_events() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"chatcmpl_evt_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"hel\"},\"logprobs\":{\"content\":[{\"token\":\"hel\"}]}}]}\n\n",
        "data: {\"id\":\"chatcmpl_evt_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_evt_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .message_user("hello")
        .send_events()
        .await
        .unwrap();

    let mut saw_content_delta = false;
    let mut saw_content_done = false;
    let mut saw_logprobs_done = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            ChatCompletionRuntimeEvent::ContentDelta(event) => {
                saw_content_delta = true;
                assert!(event.snapshot == "hel" || event.snapshot == "hello");
            }
            ChatCompletionRuntimeEvent::ContentDone(event) => {
                saw_content_done = true;
                assert_eq!(event.content, "hello");
            }
            ChatCompletionRuntimeEvent::LogProbsContentDone(event) => {
                saw_logprobs_done = true;
                assert_eq!(event.values[0].token, "hel");
            }
            _ => {}
        }
    }

    assert!(saw_content_delta);
    assert!(saw_content_done);
    assert!(saw_logprobs_done);
    assert_eq!(
        stream.snapshot().unwrap().choices[0]
            .message
            .content
            .as_deref(),
        Some("hello")
    );
}

#[tokio::test]
async fn test_should_parse_partial_json_in_chat_runtime_events() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"id\":\"chatcmpl_json_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"{\\\"city\\\":\\\"Sha\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_json_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"nghai\\\"}\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_json_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut stream = client
        .chat()
        .completions()
        .stream()
        .model("gpt-5.4")
        .message_user("hello")
        .send_events()
        .await
        .unwrap();

    let mut saw_partial = false;
    let mut saw_final = false;
    while let Some(event) = stream.next().await {
        if let ChatCompletionRuntimeEvent::ContentDelta(event) = event.unwrap() {
            if event.snapshot == "{\"city\":\"Sha" {
                saw_partial = true;
                assert_eq!(event.parsed, Some(json!({"city":"Sha"}).into()));
            }
            if event.snapshot == "{\"city\":\"Shanghai\"}" {
                saw_final = true;
                assert_eq!(event.parsed, Some(json!({"city":"Shanghai"}).into()));
            }
        }
    }

    assert!(saw_partial);
    assert!(saw_final);
}

#[tokio::test]
async fn test_should_emit_response_runtime_events() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_evt_1\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"in_progress\",\"output\":[]}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"fc_1\",\"type\":\"function_call\",\"arguments\":\"\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"delta\":\"{\\\"city\\\":\\\"Sha\"}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"delta\":\"nghai\\\"}\"}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[]}}\n\n",
        "event: response.content_part.added\n",
        "data: {\"type\":\"response.content_part.added\",\"output_index\":1,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":1,\"content_index\":0,\"delta\":\"hello\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_evt_1\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"id\":\"fc_1\",\"type\":\"function_call\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"},{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]}]}}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut stream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("hello")
        .send_events()
        .await
        .unwrap();

    let mut saw_arguments_delta = false;
    let mut saw_output_text_delta = false;
    let mut saw_completed = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            ResponseRuntimeEvent::FunctionCallArgumentsDelta(event) => {
                saw_arguments_delta = true;
                assert_eq!(event.item_id.as_deref(), Some("fc_1"));
                if event.snapshot == "{\"city\":\"Sha" {
                    assert_eq!(event.parsed_arguments, Some(json!({"city":"Sha"}).into()));
                }
                if event.snapshot == "{\"city\":\"Shanghai\"}" {
                    assert_eq!(
                        event.parsed_arguments,
                        Some(json!({"city":"Shanghai"}).into())
                    );
                }
            }
            ResponseRuntimeEvent::OutputTextDelta(event) => {
                saw_output_text_delta = true;
                assert_eq!(event.snapshot, "hello");
            }
            ResponseRuntimeEvent::Completed(response) => {
                saw_completed = true;
                assert_eq!(response.output_text().as_deref(), Some("hello"));
                assert_eq!(
                    response.output[0]
                        .as_function_call()
                        .map(|call| call.arguments.as_str()),
                    Some("{\"city\":\"Shanghai\"}")
                );
            }
            _ => {}
        }
    }

    assert!(saw_arguments_delta);
    assert!(saw_output_text_delta);
    assert!(saw_completed);
}

#[tokio::test]
async fn test_should_stream_audio_speech_over_raw_sse() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.output_audio.delta\n",
        "data: {\"type\":\"response.output_audio.delta\",\"delta\":\"AAAA\"}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/audio/speech"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut stream = client
        .audio()
        .speech()
        .stream()
        .model("gpt-4o-mini-tts")
        .voice("alloy")
        .input("你好")
        .send_raw_sse()
        .await
        .unwrap();

    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.event.as_deref(), Some("response.output_audio.delta"));
    assert!(first.data.contains("\"delta\":\"AAAA\""));

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_eq!(body["stream_format"], "sse");
}
