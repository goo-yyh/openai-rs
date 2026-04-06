use std::io::Cursor;

use bytes::Bytes;
use tokio::io::AsyncWriteExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use openai_rs::{ToFileInput, UploadSource, to_file};

#[tokio::test]
async fn test_should_build_file_from_bytes() {
    let source = to_file(Bytes::from_static(b"hello"), Some("hello.txt"))
        .await
        .unwrap();

    assert_eq!(source.filename(), "hello.txt");
    assert_eq!(source.bytes(), &Bytes::from_static(b"hello"));
}

#[tokio::test]
async fn test_should_fail_when_reader_has_no_filename() {
    let error = to_file(
        ToFileInput::reader(Cursor::new(b"hello".to_vec())),
        None::<String>,
    )
    .await
    .unwrap_err();

    assert!(matches!(error, openai_rs::Error::InvalidConfig(_)));
}

#[tokio::test]
async fn test_should_build_file_from_path() {
    let path = std::env::temp_dir().join(format!("openai-rs-to-file-{}.txt", std::process::id()));
    std::fs::write(&path, b"from-path").unwrap();

    let source = to_file(path.clone(), None::<String>).await.unwrap();
    assert_eq!(
        source.filename(),
        path.file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(source.bytes(), &Bytes::from_static(b"from-path"));

    let _ = std::fs::remove_file(path);
}

#[tokio::test]
async fn test_should_build_file_from_response_and_infer_filename() {
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

    let response = reqwest::get(format!("{}/assets/audio.mp3", server.uri()))
        .await
        .unwrap();
    let source = to_file(response, None::<String>).await.unwrap();

    assert_eq!(source.filename(), "audio.mp3");
    assert_eq!(source.mime_type(), Some("audio/mpeg"));
}

#[tokio::test]
async fn test_should_build_file_from_async_reader() {
    let (reader, mut writer) = tokio::io::duplex(64);
    tokio::spawn(async move {
        writer.write_all(b"async-reader").await.unwrap();
        writer.shutdown().await.unwrap();
    });

    let source = to_file(ToFileInput::async_reader(reader), Some("audio.wav"))
        .await
        .unwrap();

    assert_eq!(source.filename(), "audio.wav");
    assert_eq!(source.bytes(), &Bytes::from_static(b"async-reader"));
}

#[tokio::test]
async fn test_should_accept_existing_upload_source() {
    let source = UploadSource::from_bytes(Bytes::from_static(b"ok"), "old.txt");
    let source = to_file(source, Some("new.txt")).await.unwrap();

    assert_eq!(source.filename(), "new.txt");
    assert_eq!(source.bytes(), &Bytes::from_static(b"ok"));
}

#[test]
fn test_should_export_upload_source_alias() {
    let source = UploadSource::from_bytes(Bytes::from_static(b"ok"), "ok.txt");
    assert_eq!(source.filename(), "ok.txt");
}
