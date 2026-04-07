use std::time::Duration;

use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{
    AssistantRuntimeEvent, BetaAssistant, BetaThreadMessage, BetaThreadRun, ChatCompletionChunk,
    ChatCompletionMessage, ChatCompletionRuntimeEvent, Client, Model, ResponseRuntimeEvent,
    UploadSource, VectorStore,
};

#[tokio::test]
async fn test_should_send_minimal_chat_completion_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_json(json!({
            "model": "gpt-5.4",
            "messages": [{"role": "user", "content": "你好"}],
            "stream": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_1",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "你好，我是测试返回",
                    "tool_calls": [],
                    "reasoning_details": []
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_user("你好")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "chatcmpl_1");
    assert_eq!(
        response.choices[0].message.content.as_deref(),
        Some("你好，我是测试返回")
    );
}

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
    assert_eq!(chunk.logprobs_content_deltas()[0].values[0]["token"], "hel");
    assert_eq!(chunk.logprobs_refusal_deltas()[0].values[0]["token"], "no");
}

#[tokio::test]
async fn test_should_create_response_with_text_input() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_1",
            "object": "response",
            "model": "gpt-5.4",
            "status": "completed",
            "output": [
                {"type": "output_text", "text": "你好"}
            ]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("你好")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_1");
    assert_eq!(response.output_text().as_deref(), Some("你好"));
}

#[tokio::test]
async fn test_should_serialize_responses_tools_as_flat_objects() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(body_json(json!({
            "model": "gpt-5.4",
            "input": "call tool",
            "stream": false,
            "tools": [{
                "type": "function",
                "name": "add_numbers",
                "description": "Add two integers.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "a": {"type": "integer"},
                        "b": {"type": "integer"}
                    },
                    "required": ["a", "b"]
                }
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_tool_1",
            "object": "response",
            "model": "gpt-5.4",
            "status": "completed",
            "output": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("call tool")
        .tool(openai_rs::resources::ChatToolDefinition {
            tool_type: "function".into(),
            function: openai_rs::resources::ChatToolFunction {
                name: "add_numbers".into(),
                description: Some("Add two integers.".into()),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "a": {"type": "integer"},
                        "b": {"type": "integer"}
                    },
                    "required": ["a", "b"]
                }),
            },
        })
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp_tool_1");
}

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_parse_response_output_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_2",
            "object": "response",
            "model": "gpt-5.4",
            "status": "completed",
            "output": [
                {"type": "output_text", "text": "{\"answer\":\"ok\"}"}
            ]
        })))
        .mount(&server)
        .await;

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
        .responses()
        .parse::<Answer>()
        .model("gpt-5.4")
        .input_text("return json")
        .send()
        .await
        .unwrap();

    assert_eq!(parsed.parsed.answer, "ok");
}

#[tokio::test]
async fn test_should_fetch_next_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "model-1", "object": "model"}],
            "first_id": "model-1",
            "last_id": "model-1",
            "has_more": true
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(query_param("after", "model-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"id": "model-2", "object": "model"}],
            "first_id": "model-2",
            "last_id": "model-2",
            "has_more": false
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let first_page = client.models().list().send().await.unwrap();
    assert!(first_page.has_next_page());
    let next_page = first_page.next_page().await.unwrap();
    assert_eq!(next_page.data[0].id, "model-2");
}

#[test]
fn test_should_keep_full_assistant_message_for_tool_runner_history() {
    let message = ChatCompletionMessage {
        role: "assistant".into(),
        content: Some("text".into()),
        tool_calls: vec![],
        reasoning_content: Some("thinking".into()),
        reasoning_details: vec![json!({"summary":"ok"})],
        ..ChatCompletionMessage::default()
    };

    assert_eq!(message.content.as_deref(), Some("text"));
    assert_eq!(message.reasoning_content.as_deref(), Some("thinking"));
    assert_eq!(message.reasoning_details.len(), 1);
}

#[test]
fn test_should_parse_zenmux_models_list() {
    let model: Model = serde_json::from_value(json!({
        "id": "openai/gpt-5",
        "object": "model",
        "owned_by": "openai"
    }))
    .unwrap();
    assert_eq!(model.id, "openai/gpt-5");
}

#[tokio::test]
async fn test_should_retrieve_beta_assistant_as_typed_object() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/assistants/asst_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "asst_1",
            "object": "assistant",
            "model": "gpt-5.4",
            "name": "helper",
            "description": "beta assistant"
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let assistant: BetaAssistant = client
        .beta()
        .assistants()
        .retrieve("asst_1")
        .send()
        .await
        .unwrap();

    assert_eq!(assistant.id, "asst_1");
    assert_eq!(assistant.name.as_deref(), Some("helper"));
}

#[tokio::test]
async fn test_should_retrieve_vector_store_as_typed_object() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/vector_stores/vs_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "vs_1",
            "object": "vector_store",
            "name": "kb",
            "status": "completed",
            "usage_bytes": 1024
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let vector_store: VectorStore = client
        .vector_stores()
        .retrieve("vs_1")
        .send()
        .await
        .unwrap();

    assert_eq!(vector_store.id, "vs_1");
    assert_eq!(vector_store.name.as_deref(), Some("kb"));
    assert_eq!(vector_store.usage_bytes, Some(1024));
}

#[tokio::test]
async fn test_should_continue_response_stream_by_id_and_aggregate_snapshot() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"in_progress\",\"output\":[]}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[]}}\n\n",
        "event: response.content_part.added\n",
        "data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"hel\"}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"hello\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_1\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]}]}}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("GET"))
        .and(path("/responses/resp_1"))
        .and(query_param("stream", "true"))
        .and(query_param("starting_after", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .stream_response("resp_1")
        .starting_after(7)
        .send()
        .await
        .unwrap()
        .final_response()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(response.id, "resp_1");
    assert_eq!(response.status.as_deref(), Some("completed"));
    assert_eq!(response.output_text().as_deref(), Some("hello"));
}

#[tokio::test]
async fn test_should_create_assistant_stream_and_build_snapshot() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: thread.created\n",
        "data: {\"id\":\"thread_1\",\"object\":\"thread\"}\n\n",
        "event: thread.run.created\n",
        "data: {\"id\":\"run_1\",\"object\":\"thread.run\",\"thread_id\":\"thread_1\",\"status\":\"queued\"}\n\n",
        "event: thread.message.created\n",
        "data: {\"id\":\"msg_1\",\"object\":\"thread.message\",\"thread_id\":\"thread_1\",\"role\":\"assistant\",\"content\":[]}\n\n",
        "event: thread.message.delta\n",
        "data: {\"id\":\"msg_1\",\"object\":\"thread.message.delta\",\"delta\":{\"content\":[{\"index\":0,\"type\":\"text\",\"text\":{\"value\":\"hel\"}}]}}\n\n",
        "event: thread.message.delta\n",
        "data: {\"id\":\"msg_1\",\"object\":\"thread.message.delta\",\"delta\":{\"content\":[{\"index\":0,\"type\":\"text\",\"text\":{\"value\":\"lo\"}}]}}\n\n",
        "event: thread.run.completed\n",
        "data: {\"id\":\"run_1\",\"object\":\"thread.run\",\"thread_id\":\"thread_1\",\"status\":\"completed\"}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/threads/runs"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_json(json!({
            "assistant_id": "asst_1",
            "stream": true
        })))
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
        .beta()
        .threads()
        .create_and_run_stream()
        .body_value(json!({"assistant_id":"asst_1"}))
        .send()
        .await
        .unwrap();

    let mut event_names = Vec::new();
    while let Some(event) = stream.next().await {
        event_names.push(event.unwrap().event);
    }

    let run = stream.snapshot().latest_run::<BetaThreadRun>().unwrap();
    let message = stream
        .snapshot()
        .latest_message::<BetaThreadMessage>()
        .unwrap();

    assert_eq!(
        event_names.last().map(String::as_str),
        Some("thread.run.completed")
    );
    assert_eq!(run.status.as_deref(), Some("completed"));
    assert_eq!(message.content[0]["text"]["value"].as_str(), Some("hello"));
}

#[tokio::test]
async fn test_should_poll_beta_run_until_terminal_state() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/threads/thread_1/runs/run_1"))
        .and(header("openai-beta", "assistants=v2"))
        .and(header("x-stainless-poll-helper", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("openai-poll-after-ms", "1")
                .set_body_json(json!({
                    "id": "run_1",
                    "object": "thread.run",
                    "thread_id": "thread_1",
                    "status": "queued"
                })),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/threads/thread_1/runs/run_1"))
        .and(header("openai-beta", "assistants=v2"))
        .and(header("x-stainless-poll-helper", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "run_1",
            "object": "thread.run",
            "thread_id": "thread_1",
            "status": "completed"
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let run = client
        .beta()
        .threads()
        .runs()
        .poll("thread_1", "run_1", Some(Duration::from_millis(1)))
        .await
        .unwrap();

    assert_eq!(run.status.as_deref(), Some("completed"));
}

#[cfg(feature = "tool-runner")]
#[tokio::test]
async fn test_should_run_tools_with_streaming_runner() {
    let server = MockServer::start().await;
    let tool_stream = concat!(
        "data: {\"id\":\"chatcmpl_stream_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Sha\"}}]}}]}\n\n",
        "data: {\"id\":\"chatcmpl_stream_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"nghai\\\"}\"}}]}}]}\n\n",
        "data: {\"id\":\"chatcmpl_stream_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let final_stream = concat!(
        "data: {\"id\":\"chatcmpl_stream_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"晴\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_stream_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"天\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_stream_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(tool_stream, "text/event-stream"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(final_stream, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .chat()
        .completions()
        .run_tools()
        .model("gpt-5.4")
        .message_user("上海天气怎么样")
        .register_tool(openai_rs::ToolDefinition::new(
            "lookup_weather",
            Some("查询天气"),
            json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string"}
                },
                "required": ["city"]
            }),
            |arguments: serde_json::Value| async move {
                assert_eq!(arguments["city"], "Shanghai");
                Ok(json!({"weather":"sunny"}))
            },
        ))
        .send_streaming()
        .await
        .unwrap();

    assert_eq!(response.choices[0].message.content.as_deref(), Some("晴天"));
}

#[tokio::test]
async fn test_should_encode_dynamic_path_segments() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp/unsafe?id=1",
            "object": "response",
            "status": "completed",
            "output": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .responses()
        .retrieve("resp/unsafe?id=1")
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "resp/unsafe?id=1");
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].url.path(), "/responses/resp%2Funsafe%3Fid%3D1");
}

#[tokio::test]
async fn test_should_encode_nested_dynamic_path_segments() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let _: serde_json::Value = client
        .conversations()
        .items()
        .retrieve("conv/1", "item?2=3")
        .send()
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests[0].url.path(),
        "/conversations/conv%2F1/items/item%3F2%3D3"
    );
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
                assert_eq!(event.values[0]["token"], "hel");
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
                assert_eq!(event.parsed, Some(json!({"city":"Sha"})));
            }
            if event.snapshot == "{\"city\":\"Shanghai\"}" {
                saw_final = true;
                assert_eq!(event.parsed, Some(json!({"city":"Shanghai"})));
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
                    assert_eq!(event.parsed_arguments, Some(json!({"city":"Sha"})));
                }
                if event.snapshot == "{\"city\":\"Shanghai\"}" {
                    assert_eq!(event.parsed_arguments, Some(json!({"city":"Shanghai"})));
                }
            }
            ResponseRuntimeEvent::OutputTextDelta(event) => {
                saw_output_text_delta = true;
                assert_eq!(event.snapshot, "hello");
            }
            ResponseRuntimeEvent::Completed(response) => {
                saw_completed = true;
                assert_eq!(response.output_text().as_deref(), Some("hello"));
            }
            _ => {}
        }
    }

    assert!(saw_arguments_delta);
    assert!(saw_output_text_delta);
    assert!(saw_completed);
}

#[tokio::test]
async fn test_should_stream_audio_transcriptions_over_sse() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"type\":\"transcript.text.delta\",\"delta\":\"ni\"}\n\n",
        "data: {\"type\":\"transcript.text.done\",\"text\":\"ni hao\"}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let file = UploadSource::from_bytes("fake-wav", "sample.wav").with_mime_type("audio/wav");
    let mut stream = client
        .audio()
        .transcriptions()
        .stream()
        .multipart_text("model", "gpt-4o-mini-transcribe")
        .multipart_file("file", file)
        .send_sse()
        .await
        .unwrap();

    let first = stream.next().await.unwrap().unwrap();
    let second = stream.next().await.unwrap().unwrap();
    assert_eq!(first["type"], "transcript.text.delta");
    assert_eq!(second["type"], "transcript.text.done");

    let requests = server.received_requests().await.unwrap();
    let content_type = requests[0]
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(content_type.starts_with("multipart/form-data"));
    assert!(body.contains("name=\"stream\""));
    assert!(body.contains("\r\ntrue\r\n"));
    assert!(body.contains("name=\"model\""));
    assert!(body.contains("gpt-4o-mini-transcribe"));
    assert!(body.contains("filename=\"sample.wav\""));
}

#[tokio::test]
async fn test_should_stream_audio_speech_over_sse() {
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
        .body_value(json!({
            "model": "gpt-4o-mini-tts",
            "voice": "alloy",
            "input": "你好"
        }))
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

#[tokio::test]
async fn test_should_emit_assistant_runtime_events() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: thread.message.created\n",
        "data: {\"id\":\"msg_evt_1\",\"object\":\"thread.message\",\"thread_id\":\"thread_1\",\"role\":\"assistant\",\"content\":[]}\n\n",
        "event: thread.message.delta\n",
        "data: {\"id\":\"msg_evt_1\",\"object\":\"thread.message.delta\",\"delta\":{\"content\":[{\"index\":0,\"type\":\"text\",\"text\":{\"value\":\"hel\"}}]}}\n\n",
        "event: thread.run.step.created\n",
        "data: {\"id\":\"step_evt_1\",\"object\":\"thread.run.step\",\"thread_id\":\"thread_1\",\"status\":\"in_progress\",\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[]}}\n\n",
        "event: thread.run.step.delta\n",
        "data: {\"id\":\"step_evt_1\",\"object\":\"thread.run.step.delta\",\"delta\":{\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"}}]}}}\n\n",
        "event: thread.run.step.completed\n",
        "data: {\"id\":\"step_evt_1\",\"object\":\"thread.run.step\",\"thread_id\":\"thread_1\",\"status\":\"completed\",\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"}}]}}\n\n",
        "event: thread.message.completed\n",
        "data: {\"id\":\"msg_evt_1\",\"object\":\"thread.message\",\"thread_id\":\"thread_1\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":{\"value\":\"hello\"}}]}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/threads/runs"))
        .and(header("openai-beta", "assistants=v2"))
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
        .beta()
        .threads()
        .create_and_run_stream()
        .body_value(json!({"assistant_id":"asst_1"}))
        .send_events()
        .await
        .unwrap();

    let mut saw_message_delta = false;
    let mut saw_tool_call_created = false;
    let mut saw_text_done = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            AssistantRuntimeEvent::MessageDelta(event) => {
                saw_message_delta = true;
                assert_eq!(event.snapshot["content"][0]["text"]["value"], "hel");
            }
            AssistantRuntimeEvent::ToolCallCreated(event) => {
                saw_tool_call_created = true;
                assert_eq!(event.tool_call["function"]["name"], "lookup_weather");
            }
            AssistantRuntimeEvent::TextDone(event) => {
                saw_text_done = true;
                assert_eq!(event.text["text"]["value"], "hello");
            }
            _ => {}
        }
    }

    assert!(saw_message_delta);
    assert!(saw_tool_call_created);
    assert!(saw_text_done);
}

#[cfg(feature = "tool-runner")]
#[tokio::test]
async fn test_should_collect_streaming_runner_trace() {
    let server = MockServer::start().await;
    let tool_stream = concat!(
        "data: {\"id\":\"chatcmpl_runner_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Sha\"}}]}}]}\n\n",
        "data: {\"id\":\"chatcmpl_runner_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"nghai\\\"}\"}}]}}]}\n\n",
        "data: {\"id\":\"chatcmpl_runner_1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let final_stream = concat!(
        "data: {\"id\":\"chatcmpl_runner_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"晴\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_runner_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"天\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_runner_2\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-5.4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(tool_stream, "text/event-stream"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(final_stream, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let runner = client
        .chat()
        .completions()
        .run_tools()
        .model("gpt-5.4")
        .message_user("上海天气怎么样")
        .register_tool(openai_rs::ToolDefinition::new(
            "lookup_weather",
            Some("查询天气"),
            json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string"}
                },
                "required": ["city"]
            }),
            |arguments: serde_json::Value| async move {
                assert_eq!(arguments["city"], "Shanghai");
                Ok(json!({"weather":"sunny"}))
            },
        ))
        .into_streaming_runner()
        .await
        .unwrap();

    assert_eq!(runner.final_content(), Some("晴天"));
    assert_eq!(
        runner.tool_results()[0].tool_call.function.name,
        "lookup_weather"
    );
    assert_eq!(runner.tool_results()[0].output, "{\"weather\":\"sunny\"}");
    assert!(
        runner
            .events()
            .iter()
            .any(|event| matches!(event, ChatCompletionRuntimeEvent::ToolCallArgumentsDone(_)))
    );
}
