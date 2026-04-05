//! Webhook 校验。

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::error::{Error, Result, WebhookVerificationError};

type HmacSha256 = Hmac<Sha256>;

/// 表示可查询 Header 的对象。
pub trait HeaderLookup {
    /// 读取指定名称的 Header。
    fn get_header(&self, name: &str) -> Option<String>;
}

impl HeaderLookup for http::HeaderMap {
    fn get_header(&self, name: &str) -> Option<String> {
        self.get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }
}

impl HeaderLookup for BTreeMap<String, String> {
    fn get_header(&self, name: &str) -> Option<String> {
        self.get(name).cloned()
    }
}

impl<const N: usize> HeaderLookup for [(&str, &str); N] {
    fn get_header(&self, name: &str) -> Option<String> {
        self.iter()
            .find_map(|(key, value)| (*key == name).then(|| (*value).to_owned()))
    }
}

/// 表示通用 Webhook 事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// 事件 ID。
    pub id: String,
    /// 对象类型。
    pub object: Option<String>,
    /// 创建时间。
    pub created_at: i64,
    /// 事件类型。
    #[serde(rename = "type")]
    pub event_type: String,
    /// 事件数据。
    pub data: Value,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 Webhook 验签器。
#[derive(Clone)]
pub struct WebhookVerifier {
    secret: Option<SecretString>,
}

impl fmt::Debug for WebhookVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebhookVerifier")
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl WebhookVerifier {
    /// 创建新的 Webhook 验签器。
    pub fn new(secret: Option<SecretString>) -> Self {
        Self { secret }
    }

    /// 验证签名是否有效。
    ///
    /// # Errors
    ///
    /// 当 Header 缺失、时间戳异常或签名不匹配时返回错误。
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
        let secret = secret
            .map(str::to_owned)
            .or_else(|| {
                self.secret
                    .as_ref()
                    .map(|value| value.expose_secret().to_owned())
            })
            .ok_or_else(|| {
                Error::WebhookVerification(WebhookVerificationError::new("Webhook secret 未配置"))
            })?;

        let signature_header = required_header(headers, "webhook-signature")?;
        let timestamp = required_header(headers, "webhook-timestamp")?;
        let webhook_id = required_header(headers, "webhook-id")?;

        let timestamp = timestamp.parse::<u64>().map_err(|_| {
            Error::WebhookVerification(WebhookVerificationError::new(
                "Invalid webhook timestamp format",
            ))
        })?;
        validate_timestamp(timestamp, tolerance)?;

        let signed_payload = format!("{webhook_id}.{timestamp}.{payload}");
        let expected = compute_signature(&secret, signed_payload.as_bytes())?;

        let valid = signature_header.split(' ').any(|part| {
            let signature = part.strip_prefix("v1,").unwrap_or(part);
            base64::engine::general_purpose::STANDARD
                .decode(signature)
                .ok()
                .is_some_and(|candidate| candidate.ct_eq(&expected).into())
        });

        if !valid {
            return Err(Error::WebhookVerification(WebhookVerificationError::new(
                "The given webhook signature does not match the expected signature",
            )));
        }

        Ok(())
    }

    /// 先验签，再反序列化事件对象。
    ///
    /// # Errors
    ///
    /// 当签名校验失败或 JSON 反序列化失败时返回错误。
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
        self.verify_signature(payload, headers, secret, tolerance)?;
        serde_json::from_str(payload).map_err(|error| {
            Error::Serialization(crate::SerializationError::new(format!(
                "Webhook 负载解析失败: {error}"
            )))
        })
    }
}

fn required_header<H>(headers: &H, name: &str) -> Result<String>
where
    H: HeaderLookup,
{
    headers.get_header(name).ok_or_else(|| {
        Error::WebhookVerification(WebhookVerificationError::new(format!(
            "Missing required header: {name}"
        )))
    })
}

fn validate_timestamp(timestamp: u64, tolerance: Duration) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            Error::WebhookVerification(WebhookVerificationError::new(error.to_string()))
        })?
        .as_secs();
    let tolerance = tolerance.as_secs();

    if now.saturating_sub(timestamp) > tolerance {
        return Err(Error::WebhookVerification(WebhookVerificationError::new(
            "Webhook timestamp is too old",
        )));
    }

    if timestamp > now.saturating_add(tolerance) {
        return Err(Error::WebhookVerification(WebhookVerificationError::new(
            "Webhook timestamp is too new",
        )));
    }

    Ok(())
}

fn compute_signature(secret: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let key = if let Some(secret) = secret.strip_prefix("whsec_") {
        base64::engine::general_purpose::STANDARD
            .decode(secret)
            .map_err(|error| {
                Error::WebhookVerification(WebhookVerificationError::new(format!(
                    "Webhook secret 非法: {error}"
                )))
            })?
    } else {
        secret.as_bytes().to_vec()
    };

    let mut mac = HmacSha256::new_from_slice(&key).map_err(|error| {
        Error::WebhookVerification(WebhookVerificationError::new(format!(
            "创建 HMAC 失败: {error}"
        )))
    })?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::WebhookVerifier;
    use std::collections::BTreeMap;
    use std::time::Duration;

    fn test_payload() -> &'static str {
        r#"{"id": "evt_685c059ae3a481909bdc86819b066fb6", "object": "event", "created_at": 1750861210, "type": "response.completed", "data": {"id": "resp_123"}}"#
    }

    fn test_headers() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "webhook-signature".into(),
                "v1,gUAg4R2hWouRZqRQG4uJypNS8YK885G838+EHb4nKBY=".into(),
            ),
            ("webhook-timestamp".into(), "1750861210".into()),
            (
                "webhook-id".into(),
                "wh_685c059ae39c8190af8c71ed1022a24d".into(),
            ),
        ])
    }

    fn test_secret() -> &'static str {
        "whsec_RdvaYFYUXuIFuEbvZHwMfYFhUf7aMYjYcmM24+Aj40c="
    }

    #[test]
    fn test_should_verify_valid_signature() {
        let verifier = WebhookVerifier::new(None);
        verifier
            .verify_signature(
                test_payload(),
                &test_headers(),
                Some(test_secret()),
                Duration::from_secs(60 * 60 * 24 * 3650),
            )
            .unwrap();
    }

    #[test]
    fn test_should_reject_invalid_signature() {
        let verifier = WebhookVerifier::new(None);
        let error = verifier
            .verify_signature(
                test_payload(),
                &test_headers(),
                Some("whsec_Zm9v"),
                Duration::from_secs(60 * 60 * 24 * 3650),
            )
            .unwrap_err();
        assert!(matches!(error, crate::Error::WebhookVerification(_)));
    }

    #[test]
    fn test_should_unwrap_payload_after_verification() {
        let verifier = WebhookVerifier::new(None);
        let event: crate::webhooks::WebhookEvent = verifier
            .unwrap(
                test_payload(),
                &test_headers(),
                Some(test_secret()),
                Duration::from_secs(60 * 60 * 24 * 3650),
            )
            .unwrap();
        assert_eq!(event.id, "evt_685c059ae3a481909bdc86819b066fb6");
        assert_eq!(event.event_type, "response.completed");
    }
}
