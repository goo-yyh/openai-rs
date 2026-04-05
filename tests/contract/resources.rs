use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{ChatCompletionMessage, Client, Model};

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
