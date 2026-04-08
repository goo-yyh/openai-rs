use std::sync::Arc;
use std::time::Duration;

use http::HeaderMap;
use openai_rs::{WebhookEvent, WebhookVerifier};

fn webhook(verifier: &WebhookVerifier, headers: &HeaderMap, body: &str) -> WebhookEvent {
    verifier
        .unwrap(
            body,
            headers,
            Some("whsec_RdvaYFYUXuIFuEbvZHwMfYFhUf7aMYjYcmM24+Aj40c="),
            Duration::from_secs(60 * 60 * 24 * 3650),
        )
        .unwrap()
}

fn fixture_payload() -> &'static str {
    r#"{"id": "evt_685c059ae3a481909bdc86819b066fb6", "object": "event", "created_at": 1750861210, "type": "response.completed", "data": {"id": "resp_123"}}"#
}

fn fixture_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "webhook-signature",
        "v1,gUAg4R2hWouRZqRQG4uJypNS8YK885G838+EHb4nKBY="
            .parse()
            .unwrap(),
    );
    headers.insert("webhook-timestamp", "1750861210".parse().unwrap());
    headers.insert(
        "webhook-id",
        "wh_685c059ae39c8190af8c71ed1022a24d".parse().unwrap(),
    );
    headers
}

fn main() {
    let verifier = Arc::new(WebhookVerifier::new(None));
    let event = webhook(&verifier, &fixture_headers(), fixture_payload());
    assert_eq!(event.id, "evt_685c059ae3a481909bdc86819b066fb6");
    assert_eq!(event.event_type, "response.completed");
}
