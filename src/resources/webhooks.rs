//! Webhook helper implementations for the resources facade.

use std::time::Duration;

use crate::error::Result;
use crate::webhooks::{HeaderLookup, WebhookVerifier};

use super::WebhooksResource;

impl WebhooksResource {
    fn verifier(&self) -> WebhookVerifier {
        WebhookVerifier::new(self.client.inner.options.webhook_secret.clone())
    }

    /// 校验 Webhook 签名。
    ///
    /// # Errors
    ///
    /// 当签名不合法时返回错误。
    pub fn verify_signature<H>(
        &self,
        payload: &str,
        headers: &H,
        secret: Option<&str>,
        tolerance: Duration,
    ) -> Result<()>
    where
        H: HeaderLookup,
    {
        self.verifier()
            .verify_signature(payload, headers, secret, tolerance)
    }

    /// 校验签名并解包事件。
    ///
    /// # Errors
    ///
    /// 当签名校验失败或 JSON 解析失败时返回错误。
    pub fn unwrap<H, T>(
        &self,
        payload: &str,
        headers: &H,
        secret: Option<&str>,
        tolerance: Duration,
    ) -> Result<T>
    where
        H: HeaderLookup,
        T: serde::de::DeserializeOwned,
    {
        self.verifier().unwrap(payload, headers, secret, tolerance)
    }
}
