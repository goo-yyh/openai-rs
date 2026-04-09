use std::fs;
use std::path::PathBuf;

use serde::de::DeserializeOwned;

use openai_rs::{
    Batch, BetaRealtimeSession, ChatCompletion, ChatKitSession, Completion, EmbeddingResponse,
    GraderRunResponse, ModerationCreateResponse, RealtimeClientSecretCreateResponse, Response,
};

fn load_fixture<T>(name: &str) -> T
where
    T: DeserializeOwned,
{
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("spec_fixtures")
        .join(name);
    let contents = fs::read_to_string(path).unwrap();
    serde_json::from_str(&contents).unwrap()
}

#[test]
fn test_should_deserialize_spec_fixtures_into_typed_models() {
    let batch: Batch = load_fixture("batch.json");
    let chat_completion: ChatCompletion = load_fixture("chat_completion.json");
    let completion: Completion = load_fixture("completion.json");
    let moderation: ModerationCreateResponse = load_fixture("moderation.json");
    let embedding: EmbeddingResponse = load_fixture("embedding.json");
    let realtime_secret: RealtimeClientSecretCreateResponse =
        load_fixture("realtime_client_secret.json");
    let grader_run: GraderRunResponse = load_fixture("grader_run.json");
    let chatkit_session: ChatKitSession = load_fixture("chatkit_session.json");
    let beta_realtime_session: BetaRealtimeSession = load_fixture("beta_realtime_session.json");
    let response: Response = load_fixture("response.json");

    assert_eq!(batch.id, "batch_spec_1");
    assert_eq!(
        batch
            .request_counts
            .as_ref()
            .map(|counts| (counts.completed, counts.failed, counts.total)),
        Some((2, 1, 3))
    );
    assert_eq!(
        batch
            .usage
            .as_ref()
            .and_then(|usage| usage.input_tokens_details.as_ref())
            .and_then(|details| details.cached_tokens),
        Some(4)
    );
    assert_eq!(chat_completion.id, "chatcmpl_spec_1");
    assert_eq!(
        chat_completion.choices[0].message.content.as_deref(),
        Some("fixture assistant reply")
    );
    assert_eq!(
        chat_completion.choices[0].message.reasoning_details[0]
            .as_raw()
            .get("summary")
            .and_then(serde_json::Value::as_str),
        Some("concise")
    );
    assert_eq!(
        chat_completion.choices[0]
            .logprobs
            .as_ref()
            .and_then(|logprobs| logprobs.content.first())
            .map(|entry| entry.token.as_str()),
        Some("fixture")
    );
    assert_eq!(
        chat_completion
            .usage
            .as_ref()
            .and_then(|usage| usage.completion_tokens_details.as_ref())
            .and_then(|details| details.reasoning_tokens),
        Some(1)
    );
    assert_eq!(completion.id, "cmpl_spec_1");
    assert_eq!(completion.choices[0].text, "fixture completion");
    assert_eq!(moderation.id, "modr_spec_1");
    assert!(moderation.results[0].flagged);
    assert_eq!(embedding.data[0].embedding.len(), 3);
    assert_eq!(realtime_secret.secret_value(), Some("ek_fixture_1"));
    assert_eq!(grader_run.reward, Some(0.75));
    assert_eq!(chatkit_session.id, "cksess_spec_1");
    assert_eq!(beta_realtime_session.id.as_deref(), Some("sess_spec_1"));
    assert_eq!(response.output_text().as_deref(), Some("fixture response"));
    assert_eq!(response.usage.unwrap().total_tokens, 11);
    let output_text = response
        .output
        .iter()
        .find_map(|item| item.as_message())
        .and_then(|message| message.content.first())
        .and_then(|part| part.as_output_text())
        .unwrap();
    assert_eq!(output_text.annotations[0].kind(), Some("url_citation"));
    assert_eq!(
        output_text
            .logprobs
            .as_ref()
            .and_then(|entries| entries.first())
            .map(|entry| entry.token.as_str()),
        Some("fixture")
    );
}
