use std::fs;
use std::path::PathBuf;

use serde::de::DeserializeOwned;

use openai_rs::{
    BetaRealtimeSession, ChatKitSession, Completion, GraderRunResponse, ModerationCreateResponse,
    RealtimeClientSecretCreateResponse,
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
    let completion: Completion = load_fixture("completion.json");
    let moderation: ModerationCreateResponse = load_fixture("moderation.json");
    let realtime_secret: RealtimeClientSecretCreateResponse =
        load_fixture("realtime_client_secret.json");
    let grader_run: GraderRunResponse = load_fixture("grader_run.json");
    let chatkit_session: ChatKitSession = load_fixture("chatkit_session.json");
    let beta_realtime_session: BetaRealtimeSession = load_fixture("beta_realtime_session.json");

    assert_eq!(completion.id, "cmpl_spec_1");
    assert_eq!(completion.choices[0].text, "fixture completion");
    assert_eq!(moderation.id, "modr_spec_1");
    assert!(moderation.results[0].flagged);
    assert_eq!(realtime_secret.secret_value(), Some("ek_fixture_1"));
    assert_eq!(grader_run.reward, Some(0.75));
    assert_eq!(chatkit_session.id, "cksess_spec_1");
    assert_eq!(beta_realtime_session.id.as_deref(), Some("sess_spec_1"));
}
