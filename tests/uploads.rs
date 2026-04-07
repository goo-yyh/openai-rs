use std::io::Cursor;

use bytes::Bytes;
use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{Client, ToFileInput, UploadSource, to_file};

#[tokio::test]
async fn test_should_build_file_from_reader_when_filename_provided() {
    let source = to_file(
        ToFileInput::reader(Cursor::new(b"reader-body".to_vec())),
        Some("reader.txt"),
    )
    .await
    .unwrap();

    assert_eq!(source.filename(), "reader.txt");
    assert_eq!(source.bytes(), &Bytes::from_static(b"reader-body"));
}

#[tokio::test]
async fn test_should_require_filename_for_bytes_input() {
    let error = to_file(Bytes::from_static(b"hello"), None::<String>)
        .await
        .unwrap_err();

    assert!(matches!(error, openai_rs::Error::InvalidConfig(_)));
}

#[tokio::test]
async fn test_should_override_filename_for_response_input() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/assets/audio.mp3"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .set_body_raw("fake-audio", "audio/mpeg"),
        )
        .mount(&server)
        .await;

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("{}/assets/audio.mp3", server.uri()))
        .send()
        .await
        .unwrap();
    let source = to_file(response, Some("override.wav")).await.unwrap();

    assert_eq!(source.filename(), "override.wav");
    assert_eq!(source.mime_type(), Some("audio/mpeg"));
}

#[tokio::test]
async fn test_should_merge_json_body_and_multipart_file_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "file_1",
            "object": "file",
            "filename": "input.jsonl",
            "purpose": "batch"
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("sk-test")
        .base_url(server.uri())
        .disable_proxy_for_local_base_url(true)
        .build()
        .unwrap();

    let file = UploadSource::from_bytes(Bytes::from_static(b"{\"ok\":true}\n"), "input.jsonl")
        .with_mime_type("application/jsonl");
    let response = client
        .files()
        .create()
        .body_value(json!({
            "purpose": "batch",
            "metadata": {"suite": "phase1"},
            "tags": ["alpha", "beta"]
        }))
        .multipart_file("file", file)
        .send()
        .await
        .unwrap();

    assert_eq!(response.id, "file_1");

    let requests = server.received_requests().await.unwrap();
    let content_type = requests[0]
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(content_type.starts_with("multipart/form-data"));
    assert!(body.contains("name=\"purpose\""));
    assert!(body.contains("\r\nbatch\r\n"));
    assert!(body.contains("name=\"metadata[suite]\""));
    assert!(body.contains("\r\nphase1\r\n"));
    assert!(body.contains("name=\"tags[0]\""));
    assert!(body.contains("\r\nalpha\r\n"));
    assert!(body.contains("name=\"tags[1]\""));
    assert!(body.contains("\r\nbeta\r\n"));
    assert!(body.contains("name=\"file\""));
    assert!(body.contains("filename=\"input.jsonl\""));
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
