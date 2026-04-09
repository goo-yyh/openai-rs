use std::collections::BTreeMap;

use bytes::Bytes;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{
    BatchCreateParams, Client, ContainerCreateParams, ContainerExpiresAfter,
    ContainerFileCreateParams, ConversationCreateParams, ConversationItemCreateParams,
    EvalCreateParams, EvalRunCreateParams, FineTuningHyperparameterValue,
    FineTuningJobHyperparameters, FineTuningJobIntegration, FineTuningWandbIntegration,
    SkillCreateParams, SkillVersionCreateParams, UploadSource, VideoCharacterCreateParams,
    VideoCreateParams,
};

fn test_client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap()
}

#[tokio::test]
async fn test_should_use_typed_image_and_audio_resources() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/images/generations"))
        .and(body_json(json!({
            "model": "gpt-image-1",
            "prompt": "A lighthouse in the fog",
            "size": "1024x1024",
            "quality": "high"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "created": 1,
            "data": [{
                "url": "https://cdn.example.test/image.png",
                "revised_prompt": "A lighthouse in the fog"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/audio/speech"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("audio-body", "audio/mpeg"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "text": "hello from audio",
            "language": "en",
            "duration": 1.25,
            "segments": [{
                "id": 1,
                "start": 0.0,
                "end": 1.25,
                "text": "hello from audio",
                "tokens": [1, 2, 3]
            }],
            "words": [{
                "word": "hello",
                "start": 0.0,
                "end": 0.4,
                "probability": 0.99
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/audio/translations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "text": "hello from translation",
            "language": "en",
            "duration": 1.25,
            "segments": [{
                "id": 1,
                "start": 0.0,
                "end": 1.25,
                "text": "hello from translation"
            }]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);

    let image = client
        .images()
        .generate()
        .model("gpt-image-1")
        .prompt("A lighthouse in the fog")
        .size("1024x1024")
        .quality("high")
        .send()
        .await
        .unwrap();
    assert_eq!(
        image.data[0].url.as_deref(),
        Some("https://cdn.example.test/image.png")
    );

    let speech = client
        .audio()
        .speech()
        .create()
        .model("gpt-4o-mini-tts")
        .voice("alloy")
        .input("hello")
        .send()
        .await
        .unwrap();
    assert_eq!(speech, Bytes::from_static(b"audio-body"));

    let upload = UploadSource::from_bytes("fake-wav", "sample.wav").with_mime_type("audio/wav");
    let transcription = client
        .audio()
        .transcriptions()
        .create()
        .model("gpt-4o-mini-transcribe")
        .language("en")
        .file(upload.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(transcription.text, "hello from audio");
    assert_eq!(transcription.language.as_deref(), Some("en"));
    assert_eq!(
        transcription.segments[0].text.as_deref(),
        Some("hello from audio")
    );
    assert_eq!(transcription.words[0].word.as_deref(), Some("hello"));

    let translation = client
        .audio()
        .translations()
        .create()
        .model("gpt-4o-mini-transcribe")
        .file(upload)
        .send()
        .await
        .unwrap();
    assert_eq!(translation.text, "hello from translation");
    assert_eq!(
        translation.segments[0].text.as_deref(),
        Some("hello from translation")
    );

    let requests = server.received_requests().await.unwrap();
    let speech_request = requests
        .iter()
        .find(|request| request.url.path() == "/audio/speech")
        .unwrap();
    let speech_body: serde_json::Value = speech_request.body_json().unwrap();
    assert_eq!(speech_body["model"], "gpt-4o-mini-tts");
    assert_eq!(speech_body["voice"], "alloy");
    assert_eq!(speech_body["input"], "hello");

    let transcription_request = requests
        .iter()
        .find(|request| request.url.path() == "/audio/transcriptions")
        .unwrap();
    let transcription_body = String::from_utf8_lossy(&transcription_request.body);
    assert!(transcription_body.contains("name=\"model\""));
    assert!(transcription_body.contains("gpt-4o-mini-transcribe"));
    assert!(transcription_body.contains("name=\"language\""));
    assert!(transcription_body.contains("\r\nen\r\n"));
    assert!(transcription_body.contains("filename=\"sample.wav\""));
}

#[tokio::test]
async fn test_should_use_typed_fine_tuning_and_batches_resources() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/fine_tuning/jobs"))
        .and(body_json(json!({
            "model": "gpt-4o-mini",
            "training_file": "file_train_1",
            "hyperparameters": {
                "batch_size": 4,
                "n_epochs": "auto"
            },
            "integrations": [{
                "type": "wandb",
                "wandb": {
                    "project": "sdk-contract",
                    "tags": ["contract"]
                }
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ftjob_1",
            "object": "fine_tuning.job",
            "model": "gpt-4o-mini",
            "status": "running",
            "training_file": "file_train_1",
            "created_at": 1,
            "hyperparameters": {
                "batch_size": 4,
                "n_epochs": "auto"
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fine_tuning/jobs/ftjob_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ftjob_1",
            "object": "fine_tuning.job",
            "model": "gpt-4o-mini",
            "status": "succeeded",
            "training_file": "file_train_1",
            "fine_tuned_model": "ft:gpt-4o-mini:demo",
            "created_at": 1,
            "finished_at": 2,
            "hyperparameters": {
                "batch_size": 4,
                "n_epochs": "auto"
            },
            "error": {
                "code": "server_error",
                "message": "transient",
                "param": null
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fine_tuning/jobs/ftjob_1/events"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "ftevent_1",
                "object": "fine_tuning.job.event",
                "type": "info",
                "level": "info",
                "message": "queued",
                "created_at": 1,
                "data": {
                    "step": 1,
                    "train_loss": 0.25
                }
            }],
            "first_id": "ftevent_1",
            "last_id": "ftevent_1",
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fine_tuning/jobs/ftjob_1/checkpoints"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "ftckpt_1",
                "object": "fine_tuning.job.checkpoint",
                "fine_tuning_job_id": "ftjob_1",
                "fine_tuned_model_checkpoint": "ft:gpt-4o-mini:demo:ckpt",
                "step_number": 42,
                "created_at": 2,
                "metrics": {
                    "step": 42,
                    "train_loss": 0.12,
                    "valid_loss": 0.18
                }
            }],
            "first_id": "ftckpt_1",
            "last_id": "ftckpt_1",
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/batches"))
        .and(body_json(json!({
            "input_file_id": "file_batch_1",
            "endpoint": "/v1/responses",
            "completion_window": "24h"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "batch_1",
            "object": "batch",
            "endpoint": "/v1/responses",
            "status": "validating",
            "input_file_id": "file_batch_1",
            "completion_window": "24h",
            "created_at": 1,
            "request_counts": {
                "completed": 0,
                "failed": 0,
                "total": 2
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/batches/batch_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "batch_1",
            "object": "batch",
            "endpoint": "/v1/responses",
            "status": "completed",
            "input_file_id": "file_batch_1",
            "completion_window": "24h",
            "created_at": 1,
            "completed_at": 2,
            "request_counts": {
                "completed": 2,
                "failed": 0,
                "total": 2
            },
            "errors": {
                "object": "list",
                "data": [{
                    "code": "invalid_request_error",
                    "line": 7,
                    "message": "bad jsonl line",
                    "param": "input"
                }]
            },
            "usage": {
                "input_tokens": 10,
                "output_tokens": 4,
                "total_tokens": 14,
                "input_tokens_details": {
                    "cached_tokens": 2
                },
                "output_tokens_details": {
                    "reasoning_tokens": 1
                }
            }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/batches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "batch_1",
                "object": "batch",
                "endpoint": "/v1/responses",
                "status": "completed",
                "input_file_id": "file_batch_1",
                "completion_window": "24h",
                "created_at": 1,
                "request_counts": {
                    "completed": 2,
                    "failed": 0,
                    "total": 2
                }
            }],
            "first_id": "batch_1",
            "last_id": "batch_1",
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/batches/batch_1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "batch_1",
            "object": "batch",
            "endpoint": "/v1/responses",
            "status": "cancelled",
            "input_file_id": "file_batch_1",
            "completion_window": "24h",
            "created_at": 1,
            "cancelled_at": 3,
            "request_counts": {
                "completed": 1,
                "failed": 0,
                "total": 2
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);

    let job = client
        .fine_tuning()
        .jobs()
        .create()
        .model("gpt-4o-mini")
        .training_file("file_train_1")
        .hyperparameters(FineTuningJobHyperparameters {
            batch_size: Some(FineTuningHyperparameterValue::Integer(4)),
            n_epochs: Some(FineTuningHyperparameterValue::Text("auto".into())),
            ..FineTuningJobHyperparameters::default()
        })
        .integration(FineTuningJobIntegration {
            integration_type: Some("wandb".into()),
            wandb: Some(FineTuningWandbIntegration {
                project: Some("sdk-contract".into()),
                tags: vec!["contract".into()],
                ..FineTuningWandbIntegration::default()
            }),
            ..FineTuningJobIntegration::default()
        })
        .send()
        .await
        .unwrap();
    assert_eq!(job.id, "ftjob_1");
    assert_eq!(
        job.hyperparameters
            .as_ref()
            .and_then(|params| params.batch_size.as_ref()),
        Some(&FineTuningHyperparameterValue::Integer(4))
    );

    let retrieved_job = client
        .fine_tuning()
        .jobs()
        .retrieve("ftjob_1")
        .send()
        .await
        .unwrap();
    assert_eq!(retrieved_job.status.as_deref(), Some("succeeded"));
    assert_eq!(
        retrieved_job.fine_tuned_model.as_deref(),
        Some("ft:gpt-4o-mini:demo")
    );
    assert_eq!(
        retrieved_job
            .hyperparameters
            .as_ref()
            .and_then(|params| params.n_epochs.as_ref()),
        Some(&FineTuningHyperparameterValue::Text("auto".into()))
    );
    assert_eq!(
        retrieved_job
            .error
            .as_ref()
            .and_then(|error| error.code.as_deref()),
        Some("server_error")
    );

    let events = client
        .fine_tuning()
        .jobs()
        .list_events("ftjob_1")
        .limit(20)
        .send()
        .await
        .unwrap();
    assert_eq!(events.data[0].message.as_deref(), Some("queued"));
    assert_eq!(
        events.data[0].data.as_ref().and_then(|data| data.step),
        Some(1)
    );

    let checkpoints = client
        .fine_tuning()
        .jobs()
        .checkpoints()
        .list("ftjob_1")
        .send()
        .await
        .unwrap();
    assert_eq!(checkpoints.data[0].id, "ftckpt_1");
    assert_eq!(
        checkpoints.data[0]
            .metrics
            .as_ref()
            .and_then(|metrics| metrics.valid_loss),
        Some(0.18)
    );

    let batch = client
        .batches()
        .create()
        .json_body(&BatchCreateParams {
            input_file_id: Some("file_batch_1".into()),
            endpoint: Some("/v1/responses".into()),
            completion_window: Some("24h".into()),
            ..BatchCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(batch.id, "batch_1");
    assert_eq!(
        batch.request_counts.as_ref().map(|counts| counts.total),
        Some(2)
    );

    let retrieved_batch = client.batches().retrieve("batch_1").send().await.unwrap();
    assert_eq!(retrieved_batch.status.as_deref(), Some("completed"));
    assert_eq!(
        retrieved_batch
            .request_counts
            .as_ref()
            .map(|counts| counts.completed),
        Some(2)
    );
    assert_eq!(
        retrieved_batch
            .errors
            .as_ref()
            .and_then(|errors| errors.data.first())
            .and_then(|error| error.line),
        Some(7)
    );
    assert_eq!(
        retrieved_batch
            .usage
            .as_ref()
            .map(|usage| usage.total_tokens),
        Some(14)
    );
    assert_eq!(
        retrieved_batch
            .usage
            .as_ref()
            .and_then(|usage| usage.input_tokens_details.as_ref())
            .and_then(|details| details.cached_tokens),
        Some(2)
    );

    let batches = client.batches().list().limit(10).send().await.unwrap();
    assert_eq!(batches.data[0].status.as_deref(), Some("completed"));
    assert_eq!(
        batches.data[0]
            .request_counts
            .as_ref()
            .map(|counts| counts.total),
        Some(2)
    );

    let cancelled_batch = client.batches().cancel("batch_1").send().await.unwrap();
    assert_eq!(cancelled_batch.status.as_deref(), Some("cancelled"));
    assert_eq!(cancelled_batch.cancelled_at, Some(3));
}

#[tokio::test]
async fn test_should_use_typed_conversation_and_eval_resources() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/conversations"))
        .and(body_json(json!({
            "name": "support",
            "metadata": {"team": "ops"}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "conv_1",
            "object": "conversation",
            "name": "support",
            "metadata": {"team": "ops"},
            "created_at": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/conversations/conv_1/items"))
        .and(body_json(json!({
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "hello"
            }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "item_1",
            "object": "conversation.item",
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "hello"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/conversations/conv_1/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{
                "id": "item_1",
                "object": "conversation.item",
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "hello"
                }]
            }],
            "first_id": "item_1",
            "last_id": "item_1",
            "has_more": false
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/evals"))
        .and(body_json(json!({
            "name": "support-eval",
            "data_source": {
                "type": "conversation",
                "conversation_id": "conv_1"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "eval_1",
            "object": "eval",
            "name": "support-eval",
            "status": "active",
            "created_at": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/evals/eval_1/runs"))
        .and(body_json(json!({
            "input": {
                "conversation_id": "conv_1"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "run_1",
            "object": "eval.run",
            "eval_id": "eval_1",
            "status": "queued",
            "created_at": 2
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/evals/eval_1/runs/run_1/output_items/out_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "out_1",
            "object": "eval.output_item",
            "status": "completed",
            "output": {
                "type": "output_text",
                "score": 0.98
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);

    let conversation = client
        .conversations()
        .create()
        .json_body(&ConversationCreateParams {
            name: Some("support".into()),
            metadata: BTreeMap::from([("team".into(), "ops".into())]),
            ..ConversationCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(conversation.id, "conv_1");

    let item = client
        .conversations()
        .items()
        .create("conv_1")
        .json_body(&ConversationItemCreateParams {
            item_type: Some("message".into()),
            role: Some("user".into()),
            content: vec![
                json!({
                    "type": "input_text",
                    "text": "hello"
                })
                .into(),
            ],
            ..ConversationItemCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(item.role.as_deref(), Some("user"));
    assert_eq!(item.content[0].kind(), Some("input_text"));

    let items = client
        .conversations()
        .items()
        .list("conv_1")
        .send()
        .await
        .unwrap();
    assert_eq!(items.data[0].id, "item_1");

    let eval = client
        .evals()
        .create()
        .json_body(&EvalCreateParams {
            name: Some("support-eval".into()),
            data_source: Some(
                json!({
                    "type": "conversation",
                    "conversation_id": "conv_1"
                })
                .into(),
            ),
            ..EvalCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(eval.id, "eval_1");

    let run = client
        .evals()
        .runs()
        .create("eval_1")
        .json_body(&EvalRunCreateParams {
            input: Some(
                json!({
                    "conversation_id": "conv_1"
                })
                .into(),
            ),
            ..EvalRunCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(run.status.as_deref(), Some("queued"));

    let output_item = client
        .evals()
        .runs()
        .output_items()
        .retrieve("eval_1", "run_1", "out_1")
        .send()
        .await
        .unwrap();
    assert_eq!(output_item.status.as_deref(), Some("completed"));
    assert_eq!(
        output_item.output.as_ref().and_then(|output| output.kind()),
        Some("output_text")
    );
}

#[tokio::test]
async fn test_should_use_typed_container_skill_and_video_resources() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/containers"))
        .and(body_json(json!({
            "name": "sandbox",
            "expires_after": {
                "anchor": "last_active_at",
                "minutes": 60
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cont_1",
            "object": "container",
            "name": "sandbox",
            "status": "active",
            "created_at": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/containers/cont_1/files"))
        .and(body_json(json!({
            "file_id": "file_1",
            "path": "/workspace/input.txt"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cfile_1",
            "object": "container.file",
            "container_id": "cont_1",
            "file_id": "file_1",
            "filename": "input.txt",
            "status": "ready"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/skills"))
        .and(body_json(json!({
            "name": "writer",
            "instructions": "Write concise release notes."
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "skill_1",
            "object": "skill",
            "name": "writer",
            "status": "active"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/skills/skill_1/versions"))
        .and(body_json(json!({
            "description": "initial version",
            "content": {
                "instructions": "Write concise release notes."
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ver_1",
            "object": "skill.version",
            "skill_id": "skill_1",
            "status": "ready"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/videos"))
        .and(body_json(json!({
            "model": "sora-1",
            "prompt": "A fox running through snow",
            "duration": "5s"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "video_1",
            "object": "video",
            "model": "sora-1",
            "prompt": "A fox running through snow",
            "status": "queued",
            "created_at": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/videos/characters"))
        .and(body_json(json!({
            "name": "fox",
            "image": "file_img_1"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "char_1",
            "object": "video.character",
            "name": "fox",
            "status": "ready"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server);

    let container = client
        .containers()
        .create()
        .json_body(&ContainerCreateParams {
            name: Some("sandbox".into()),
            expires_after: Some(ContainerExpiresAfter {
                anchor: Some("last_active_at".into()),
                minutes: Some(60),
                ..ContainerExpiresAfter::default()
            }),
            ..ContainerCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(container.id, "cont_1");

    let container_file = client
        .containers()
        .files()
        .create("cont_1")
        .json_body(&ContainerFileCreateParams {
            file_id: Some("file_1".into()),
            path: Some("/workspace/input.txt".into()),
            ..ContainerFileCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(container_file.filename.as_deref(), Some("input.txt"));

    let skill = client
        .skills()
        .create()
        .json_body(&SkillCreateParams {
            name: Some("writer".into()),
            instructions: Some("Write concise release notes.".into()),
            ..SkillCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(skill.id, "skill_1");

    let version = client
        .skills()
        .versions()
        .create("skill_1")
        .json_body(&SkillVersionCreateParams {
            description: Some("initial version".into()),
            content: Some(
                json!({
                    "instructions": "Write concise release notes."
                })
                .into(),
            ),
            ..SkillVersionCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(version.id, "ver_1");

    let video = client
        .videos()
        .create()
        .json_body(&VideoCreateParams {
            model: Some("sora-1".into()),
            prompt: Some("A fox running through snow".into()),
            duration: Some("5s".into()),
            ..VideoCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(video.id, "video_1");

    let character = client
        .videos()
        .create_character()
        .json_body(&VideoCharacterCreateParams {
            name: Some("fox".into()),
            image: Some("file_img_1".into()),
            ..VideoCharacterCreateParams::default()
        })
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(character.id, "char_1");
}
