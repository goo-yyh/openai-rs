use std::io::Cursor;

use bytes::Bytes;
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

#[test]
fn test_should_export_upload_source_alias() {
    let source = UploadSource::from_bytes(Bytes::from_static(b"ok"), "ok.txt");
    assert_eq!(source.filename(), "ok.txt");
}
