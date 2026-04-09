use futures_util::StreamExt;
use insta::{assert_debug_snapshot, assert_snapshot};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[cfg(feature = "realtime")]
use openai_rs::RealtimeServerEvent;
#[cfg(feature = "responses-ws")]
use openai_rs::ResponsesServerEvent;
use openai_rs::{AssistantRuntimeEvent, Client, ResponseRuntimeEvent};

#[tokio::test]
async fn test_should_snapshot_chat_completion_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "chatcmpl_snapshot",
            "object": "chat.completion",
            "created": 1,
            "model": "gpt-5.4",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": "ok"
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

    let _ = client
        .chat()
        .completions()
        .create()
        .model("gpt-5.4")
        .message_system("你是一个测试助手")
        .message_user("请返回一句话")
        .temperature(0.2)
        .extra_body("metadata", json!({"suite":"snapshot"}))
        .send()
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let body: serde_json::Value = requests[0].body_json().unwrap();
    assert_snapshot!(
        "chat_completion_request_body",
        serde_json::to_string_pretty(&body).unwrap()
    );
}

#[tokio::test]
async fn test_should_snapshot_api_error_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-request-id", "req_snapshot")
                .set_body_json(json!({
                    "error": {
                        "message": "too many requests"
                    }
                })),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .max_retries(0)
        .build()
        .unwrap();

    let error = client
        .responses()
        .create()
        .model("gpt-5.4")
        .input_text("hello")
        .send()
        .await
        .unwrap_err();

    assert_debug_snapshot!("api_error_mapping", error);
}

#[tokio::test]
async fn test_should_snapshot_response_stream_aggregation() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hel\"}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"lo\"}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"text\":\"hello\"}\n\n",
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
        .send()
        .await
        .unwrap();

    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }

    assert_snapshot!(
        "response_stream_aggregation",
        serde_json::to_string_pretty(&json!({
            "events": events,
            "output_text": stream.output_text(),
        }))
        .unwrap()
    );
}

#[tokio::test]
async fn test_should_snapshot_send_with_meta_response_meta() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req_meta_snapshot")
                .set_body_json(json!({
                    "id": "resp_meta_snapshot",
                    "object": "response",
                    "status": "completed",
                    "output": [{"type":"output_text","text":"ok"}]
                })),
        )
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
        .input_text("hello")
        .send_with_meta()
        .await
        .unwrap();
    let url = url::Url::parse(&response.meta.url).unwrap();

    assert_snapshot!(
        "send_with_meta_response_meta",
        serde_json::to_string_pretty(&json!({
            "id": response.id,
            "status": response.meta.status.as_u16(),
            "request_id": response.meta.request_id,
            "provider": format!("{:?}", response.meta.provider),
            "attempts": response.meta.attempts,
            "path": url.path(),
        }))
        .unwrap()
    );
}

#[tokio::test]
async fn test_should_snapshot_out_of_order_response_runtime_contract() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_phase2\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"in_progress\",\"output\":[]}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"hel\"}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[]}}\n\n",
        "event: response.content_part.added\n",
        "data: {\"type\":\"response.content_part.added\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\"}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":1,\"item\":{\"id\":\"fc_1\",\"type\":\"function_call\",\"arguments\":\"\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":1,\"item_id\":\"fc_1\",\"delta\":\"{\\\"city\\\":\\\"Sha\"}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":1,\"item_id\":\"fc_1\",\"delta\":\"nghai\\\"}\"}\n\n",
        "event: response.output_text.done\n",
        "data: {\"type\":\"response.output_text.done\",\"output_index\":0,\"content_index\":0,\"text\":\"hello\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_phase2\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]},{\"id\":\"fc_1\",\"type\":\"function_call\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"}]}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_phase2\",\"object\":\"response\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"output\":[{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]},{\"id\":\"fc_1\",\"type\":\"function_call\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"}]}}\n\n",
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

    let mut events_stream = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("hello")
        .send_events()
        .await
        .unwrap();

    let mut events = Vec::new();
    let mut completed_count = 0usize;
    while let Some(event) = events_stream.next().await {
        match event.unwrap() {
            ResponseRuntimeEvent::OutputItemAdded {
                output_index,
                snapshot,
                ..
            } => events.push(json!({
                "type": "response.output_item.added",
                "output_index": output_index,
                "output_text": snapshot.output_text(),
            })),
            ResponseRuntimeEvent::ContentPartAdded {
                output_index,
                content_index,
                snapshot,
                ..
            } => events.push(json!({
                "type": "response.content_part.added",
                "output_index": output_index,
                "content_index": content_index,
                "output_text": snapshot.output_text(),
            })),
            ResponseRuntimeEvent::OutputTextDelta(event) => events.push(json!({
                "type": event.event_type,
                "text": event.text,
                "snapshot": event.snapshot,
            })),
            ResponseRuntimeEvent::OutputTextDone(event) => events.push(json!({
                "type": event.event_type,
                "text": event.text,
                "snapshot": event.snapshot,
            })),
            ResponseRuntimeEvent::FunctionCallArgumentsDelta(event) => events.push(json!({
                "type": "response.function_call_arguments.delta",
                "item_id": event.item_id,
                "delta": event.delta,
                "snapshot": event.snapshot,
                "parsed_arguments": event.parsed_arguments,
            })),
            ResponseRuntimeEvent::Completed(response) => {
                completed_count += 1;
                events.push(json!({
                    "type": "response.completed",
                    "output_text": response.output_text(),
                }));
            }
            _ => {}
        }
    }

    let event_stream_final = events_stream.snapshot().unwrap();
    let plain_stream_final = client
        .responses()
        .stream()
        .model("gpt-5.4")
        .input_text("hello")
        .send()
        .await
        .unwrap()
        .final_response()
        .await
        .unwrap()
        .unwrap();
    let event_stream_function_arguments = event_stream_final.output[1]
        .as_function_call()
        .map(|call| call.arguments.clone());
    let plain_stream_function_arguments = plain_stream_final.output[1]
        .as_function_call()
        .map(|call| call.arguments.clone());

    assert_snapshot!(
        "response_runtime_out_of_order_contract",
        serde_json::to_string_pretty(&json!({
            "events": events,
            "completed_count": completed_count,
            "event_stream_output_text": event_stream_final.output_text(),
            "plain_stream_output_text": plain_stream_final.output_text(),
            "event_stream_function_arguments": event_stream_function_arguments,
            "plain_stream_function_arguments": plain_stream_function_arguments,
        }))
        .unwrap()
    );
}

#[tokio::test]
async fn test_should_snapshot_assistant_runtime_final_snapshot_contract() {
    let server = MockServer::start().await;
    let body = concat!(
        "event: thread.message.delta\n",
        "data: {\"id\":\"msg_phase2\",\"object\":\"thread.message.delta\",\"delta\":{\"content\":[{\"index\":0,\"type\":\"text\",\"text\":{\"value\":\"hel\"}}]}}\n\n",
        "event: thread.run.step.delta\n",
        "data: {\"id\":\"step_phase2\",\"object\":\"thread.run.step.delta\",\"delta\":{\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Sha\"}}]}}}\n\n",
        "event: thread.run.step.delta\n",
        "data: {\"id\":\"step_phase2\",\"object\":\"thread.run.step.delta\",\"delta\":{\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"arguments\":\"nghai\\\"}\"}}]}}}\n\n",
        "event: thread.message.completed\n",
        "data: {\"id\":\"msg_phase2\",\"object\":\"thread.message\",\"thread_id\":\"thread_1\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":{\"value\":\"hello\"}}]}\n\n",
        "event: thread.run.step.completed\n",
        "data: {\"id\":\"step_phase2\",\"object\":\"thread.run.step\",\"thread_id\":\"thread_1\",\"status\":\"completed\",\"step_details\":{\"type\":\"tool_calls\",\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"name\":\"lookup_weather\",\"arguments\":\"{\\\"city\\\":\\\"Shanghai\\\"}\"}}]}}\n\n",
        "event: thread.run.completed\n",
        "data: {\"id\":\"run_phase2\",\"object\":\"thread.run\",\"thread_id\":\"thread_1\",\"status\":\"completed\"}\n\n",
        "data: [DONE]\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/threads/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "text/event-stream"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let mut events_stream = client
        .beta()
        .threads()
        .create_and_run_stream()
        .body_value(json!({"assistant_id":"asst_1"}))
        .send_events()
        .await
        .unwrap();

    let mut event_kinds = Vec::new();
    while let Some(event) = events_stream.next().await {
        let label = match event.unwrap() {
            AssistantRuntimeEvent::Event(event) => format!("event:{}", event.event),
            AssistantRuntimeEvent::MessageDelta(_) => "runtime:message_delta".into(),
            AssistantRuntimeEvent::MessageDone(_) => "runtime:message_done".into(),
            AssistantRuntimeEvent::RunStepDelta(_) => "runtime:run_step_delta".into(),
            AssistantRuntimeEvent::RunStepDone(_) => "runtime:run_step_done".into(),
            AssistantRuntimeEvent::ToolCallCreated(_) => "runtime:tool_call_created".into(),
            AssistantRuntimeEvent::ToolCallDelta(_) => "runtime:tool_call_delta".into(),
            AssistantRuntimeEvent::ToolCallDone(_) => "runtime:tool_call_done".into(),
            AssistantRuntimeEvent::TextCreated(_) => "runtime:text_created".into(),
            AssistantRuntimeEvent::TextDelta(_) => "runtime:text_delta".into(),
            AssistantRuntimeEvent::TextDone(_) => "runtime:text_done".into(),
            other => format!("runtime:{other:?}"),
        };
        event_kinds.push(label);
    }

    let event_stream_snapshot = events_stream.final_snapshot().await.unwrap();
    let plain_stream_snapshot = client
        .beta()
        .threads()
        .create_and_run_stream()
        .body_value(json!({"assistant_id":"asst_1"}))
        .send()
        .await
        .unwrap()
        .final_snapshot()
        .await
        .unwrap();

    assert_snapshot!(
        "assistant_runtime_final_snapshot_contract",
        serde_json::to_string_pretty(&json!({
            "event_kinds": event_kinds,
            "event_stream_message_text": event_stream_snapshot.latest_message_raw()
                .and_then(|message| message.get("content"))
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.first())
                .and_then(|part| part.get("text"))
                .and_then(|text| text.get("value"))
                .and_then(serde_json::Value::as_str),
            "plain_stream_message_text": plain_stream_snapshot.latest_message_raw()
                .and_then(|message| message.get("content"))
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.first())
                .and_then(|part| part.get("text"))
                .and_then(|text| text.get("value"))
                .and_then(serde_json::Value::as_str),
            "event_stream_tool_arguments": event_stream_snapshot.latest_run_step_raw()
                .and_then(|step| step.get("step_details"))
                .and_then(|details| details.get("tool_calls"))
                .and_then(serde_json::Value::as_array)
                .and_then(|tool_calls| tool_calls.first())
                .and_then(|tool_call| tool_call.get("function"))
                .and_then(|function| function.get("arguments"))
                .and_then(serde_json::Value::as_str),
            "plain_stream_tool_arguments": plain_stream_snapshot.latest_run_step_raw()
                .and_then(|step| step.get("step_details"))
                .and_then(|details| details.get("tool_calls"))
                .and_then(serde_json::Value::as_array)
                .and_then(|tool_calls| tool_calls.first())
                .and_then(|tool_call| tool_call.get("function"))
                .and_then(|function| function.get("arguments"))
                .and_then(serde_json::Value::as_str),
            "event_stream_run_status": event_stream_snapshot.latest_run_raw()
                .and_then(|run| run.get("status"))
                .and_then(serde_json::Value::as_str),
            "plain_stream_run_status": plain_stream_snapshot.latest_run_raw()
                .and_then(|run| run.get("status"))
                .and_then(serde_json::Value::as_str),
        }))
        .unwrap()
    );
}

#[cfg(feature = "structured-output")]
#[tokio::test]
async fn test_should_snapshot_content_filter_finish_reason_error() {
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

    assert_debug_snapshot!("content_filter_finish_reason_error", error);
}

#[cfg(feature = "responses-ws")]
#[test]
fn test_should_snapshot_responses_websocket_event_decode() {
    let event: ResponsesServerEvent = serde_json::from_value(json!({
        "type": "response.output_text.delta",
        "response_id": "resp_1",
        "item_id": "item_1",
        "delta": "hello"
    }))
    .unwrap();

    assert_debug_snapshot!("responses_websocket_event_decode", event);
}

#[cfg(feature = "realtime")]
#[test]
fn test_should_snapshot_realtime_websocket_event_decode() {
    let event: RealtimeServerEvent = serde_json::from_value(json!({
        "type": "session.created",
        "session": {
            "id": "sess_1",
            "object": "realtime.session"
        }
    }))
    .unwrap();

    assert_debug_snapshot!("realtime_websocket_event_decode", event);
}
