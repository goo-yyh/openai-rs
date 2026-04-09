use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::ops::Deref;

/// 通用的原始 JSON 载荷包装器。
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(transparent)]
pub struct JsonPayload(Value);

impl fmt::Debug for JsonPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl JsonPayload {
    /// 返回未经解释的原始 JSON 值。
    pub fn as_raw(&self) -> &Value {
        &self.0
    }

    /// 消费包装器并返回原始 JSON 值。
    pub fn into_raw(self) -> Value {
        self.0
    }

    /// 返回载荷中的 `type` 字段，若存在且为字符串。
    pub fn kind(&self) -> Option<&str> {
        self.0.get("type").and_then(Value::as_str)
    }

    /// 返回载荷中指定 key 的原始 JSON 值。
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }
}

impl From<Value> for JsonPayload {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl From<JsonPayload> for Value {
    fn from(value: JsonPayload) -> Self {
        value.0
    }
}

impl AsRef<Value> for JsonPayload {
    fn as_ref(&self) -> &Value {
        self.as_raw()
    }
}

impl Deref for JsonPayload {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        self.as_raw()
    }
}

impl PartialEq<Value> for JsonPayload {
    fn eq(&self, other: &Value) -> bool {
        self.as_raw() == other
    }
}

impl PartialEq<JsonPayload> for Value {
    fn eq(&self, other: &JsonPayload) -> bool {
        self == other.as_raw()
    }
}
