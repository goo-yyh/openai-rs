use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{
    BetaRealtimeSession, ChatKitSession, ChatKitThread, ChatKitThreadItem, Client, Completion,
    GraderRunResponse, GraderValidateResponse, ModerationCreateResponse, Page, UploadPart,
    UploadSource, VectorStoreFileChunkingStrategy, VectorStoreFileContent,
    VectorStoreSearchResponse,
};

#[tokio::test]
async fn test_should_deserialize_typed_completion_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/completions"))
        .and(body_json(json!({
            "model": "gpt-3.5-turbo-instruct",
            "prompt": "hello"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cmpl_1",
            "object": "text_completion",
            "created": 1,
            "model": "gpt-3.5-turbo-instruct",
            "choices": [{
                "index": 0,
                "finish_reason": "stop",
                "text": "hello world",
                "logprobs": null
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 2,
                "total_tokens": 3
            }
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let completion: Completion = client
        .completions()
        .create()
        .body_value(json!({
            "model": "gpt-3.5-turbo-instruct",
            "prompt": "hello"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(completion.id, "cmpl_1");
    assert_eq!(completion.choices[0].text, "hello world");
    assert_eq!(completion.usage.unwrap().total_tokens, 3);
}

#[tokio::test]
async fn test_should_deserialize_typed_moderation_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/moderations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "modr_1",
            "model": "omni-moderation-latest",
            "results": [{
                "categories": {"violence": true, "sexual": false},
                "category_applied_input_types": {
                    "violence": ["text"],
                    "sexual": ["text"]
                },
                "category_scores": {"violence": 0.93, "sexual": 0.01},
                "flagged": true
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

    let response: ModerationCreateResponse = client
        .moderations()
        .create()
        .body_value(json!({"input": "violent text"}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "modr_1");
    assert!(response.results[0].flagged);
    assert_eq!(response.results[0].categories.get("violence"), Some(&true));
}

#[tokio::test]
async fn test_should_deserialize_realtime_client_secret_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/realtime/client_secrets"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "client_secret": {
                "expires_at": 1700000000,
                "value": "ek_test_123"
            },
            "type": "realtime",
            "session": {
                "model": "gpt-realtime"
            }
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
        .realtime()
        .client_secrets()
        .create()
        .body_value(json!({"session": {"type": "realtime"}}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.secret_value(), Some("ek_test_123"));
    assert_eq!(response.session_type.as_deref(), Some("realtime"));
}

#[tokio::test]
async fn test_should_treat_realtime_call_actions_as_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/realtime/calls/call_1/hangup"))
        .and(header("accept", "*/*"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let response = client
        .realtime()
        .calls()
        .hangup("call_1")
        .send_with_meta()
        .await
        .unwrap();

    assert_eq!(response.meta.status.as_u16(), 204);
}

#[tokio::test]
async fn test_should_deserialize_upload_part_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/uploads/upl_1/parts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "part_1",
            "object": "upload.part",
            "created_at": 1,
            "upload_id": "upl_1"
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let part: UploadPart = client
        .uploads()
        .parts()
        .create("upl_1")
        .multipart_file("data", UploadSource::from_bytes("chunk-1", "part.bin"))
        .send()
        .await
        .unwrap();

    assert_eq!(part.id, "part_1");
    assert_eq!(part.upload_id.as_deref(), Some("upl_1"));
}

#[tokio::test]
async fn test_should_deserialize_typed_grader_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/fine_tuning/alpha/graders/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "metadata": {
                "name": "string-check",
                "type": "string_check",
                "execution_time": 0.2,
                "scores": {"exact_match": 1.0},
                "token_usage": 12,
                "errors": {
                    "formula_parse_error": false,
                    "invalid_variable_error": false,
                    "model_grader_parse_error": false,
                    "model_grader_refusal_error": false,
                    "model_grader_server_error": false,
                    "other_error": false,
                    "python_grader_runtime_error": false,
                    "python_grader_server_error": false,
                    "sample_parse_error": false,
                    "truncated_observation_error": false,
                    "unresponsive_reward_error": false
                }
            },
            "reward": 1.0,
            "sub_rewards": {"exact_match": 1.0},
            "model_grader_token_usage_per_model": {}
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/fine_tuning/alpha/graders/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "grader": {"type": "string_check", "name": "string-check"}
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let run: GraderRunResponse = client
        .fine_tuning()
        .alpha()
        .graders()
        .run()
        .body_value(json!({
            "grader": {"type": "string_check", "name": "string-check"},
            "model_sample": "ok"
        }))
        .send()
        .await
        .unwrap();
    let validate: GraderValidateResponse = client
        .fine_tuning()
        .alpha()
        .graders()
        .validate()
        .body_value(json!({
            "grader": {"type": "string_check", "name": "string-check"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        run.metadata.unwrap().grader_type.as_deref(),
        Some("string_check")
    );
    assert_eq!(validate.grader.unwrap()["type"], "string_check");
}

#[tokio::test]
async fn test_should_use_chatkit_beta_header_and_typed_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chatkit/sessions"))
        .and(header("openai-beta", "chatkit_beta=v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cksess_1",
            "object": "chatkit.session",
            "client_secret": "ck_secret_1",
            "expires_at": 1700000000,
            "max_requests_per_1_minute": 10,
            "status": "active",
            "user": "u_1"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/chatkit/threads/cthr_1"))
        .and(header("openai-beta", "chatkit_beta=v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cthr_1",
            "object": "chatkit.thread",
            "created_at": 1,
            "status": {"type": "active"},
            "title": "demo",
            "user": "u_1"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/chatkit/threads/cthr_1/items"))
        .and(header("openai-beta", "chatkit_beta=v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "cthi_1",
                "object": "chatkit.thread_item",
                "thread_id": "cthr_1",
                "created_at": 1,
                "type": "chatkit.assistant_message",
                "content": [{"type": "output_text", "text": "hello"}]
            }],
            "first_id": "cthi_1",
            "last_id": "cthi_1",
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

    let session: ChatKitSession = client
        .beta()
        .chatkit()
        .sessions()
        .create()
        .body_value(json!({
            "user": "u_1",
            "workflow": {"id": "wf_1"}
        }))
        .send()
        .await
        .unwrap();
    let thread: ChatKitThread = client
        .beta()
        .chatkit()
        .threads()
        .retrieve("cthr_1")
        .send()
        .await
        .unwrap();
    let items = client
        .beta()
        .chatkit()
        .threads()
        .list_items("cthr_1")
        .send()
        .await
        .unwrap();

    assert_eq!(session.id, "cksess_1");
    assert_eq!(thread.title.as_deref(), Some("demo"));
    assert_eq!(items.data.len(), 1);
    let item: &ChatKitThreadItem = &items.data[0];
    assert_eq!(item.thread_id.as_deref(), Some("cthr_1"));
}

#[tokio::test]
async fn test_should_deserialize_beta_realtime_session_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/realtime/sessions"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "sess_1",
            "type": "realtime",
            "model": "gpt-4o-realtime-preview",
            "modalities": ["text", "audio"],
            "client_secret": {
                "expires_at": 1700000000,
                "value": "ek_beta_1"
            }
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let session: BetaRealtimeSession = client
        .beta()
        .realtime()
        .sessions()
        .create()
        .body_value(json!({"model": "gpt-4o-realtime-preview"}))
        .send()
        .await
        .unwrap();

    assert_eq!(session.id.as_deref(), Some("sess_1"));
    assert_eq!(session.client_secret.unwrap().value, "ek_beta_1");
}

#[tokio::test]
async fn test_should_use_vector_store_beta_header_and_typed_payloads() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/vector_stores/vs_1"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "vs_1",
            "object": "vector_store",
            "created_at": 1,
            "name": "kb",
            "status": "completed",
            "usage_bytes": 1024,
            "last_active_at": 2,
            "file_counts": {
                "completed": 1,
                "failed": 0,
                "in_progress": 0,
                "cancelled": 0,
                "total": 1
            },
            "metadata": {
                "scope": "fixture"
            },
            "expires_after": {
                "anchor": "last_active_at",
                "days": 7
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/vector_stores/vs_1/files/file_1"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "file_1",
            "object": "vector_store.file",
            "created_at": 1,
            "vector_store_id": "vs_1",
            "status": "completed",
            "usage_bytes": 512,
            "attributes": {
                "lang": "zh",
                "priority": 1,
                "published": true
            },
            "chunking_strategy": {
                "type": "static",
                "static": {
                    "max_chunk_size_tokens": 800,
                    "chunk_overlap_tokens": 400
                }
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/vector_stores/vs_1/files/file_1/content"))
        .and(header("openai-beta", "assistants=v2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "type": "text",
                "text": "chunk 1"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/vector_stores/vs_1/search"))
        .and(header("openai-beta", "assistants=v2"))
        .and(body_json(json!({
            "query": "hello"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "search_query": "hello",
            "data": [{
                "file_id": "file_1",
                "filename": "kb.md",
                "score": 0.98,
                "attributes": {
                    "lang": "zh",
                    "priority": 1,
                    "published": true
                },
                "content": [{
                    "type": "text",
                    "text": "chunk 1"
                }]
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

    let vector_store = client
        .vector_stores()
        .retrieve("vs_1")
        .send()
        .await
        .unwrap();
    let vector_store_file = client
        .vector_stores()
        .files()
        .retrieve("vs_1", "file_1")
        .send()
        .await
        .unwrap();
    let content_page: Page<VectorStoreFileContent> = client
        .vector_stores()
        .files()
        .content("vs_1", "file_1")
        .send()
        .await
        .unwrap();
    let search_response: VectorStoreSearchResponse = client
        .vector_stores()
        .search("vs_1")
        .body_value(json!({"query": "hello"}))
        .send()
        .await
        .unwrap();

    assert_eq!(vector_store.file_counts.unwrap().completed, Some(1));
    assert_eq!(
        vector_store
            .metadata
            .unwrap()
            .get("scope")
            .map(String::as_str),
        Some("fixture")
    );
    match vector_store_file.chunking_strategy.unwrap() {
        VectorStoreFileChunkingStrategy::Static { configuration } => {
            assert_eq!(configuration.max_chunk_size_tokens, Some(800));
            assert_eq!(configuration.chunk_overlap_tokens, Some(400));
        }
        other => panic!("unexpected chunking strategy: {other:?}"),
    }
    assert_eq!(content_page.data[0].content_type.as_deref(), Some("text"));
    assert_eq!(content_page.data[0].text.as_deref(), Some("chunk 1"));
    assert_eq!(search_response.search_query.as_deref(), Some("hello"));
    assert_eq!(search_response.data[0].filename.as_deref(), Some("kb.md"));
    assert_eq!(
        search_response.data[0].content[0].text.as_deref(),
        Some("chunk 1")
    );
}
