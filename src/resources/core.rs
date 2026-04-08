//! Core resource implementations that do not need additional namespace files.

use std::collections::BTreeMap;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::generated::endpoints;

use super::{
    CompletionsResource, DeleteResponse, EmbeddingResponse, EmbeddingsResource, JsonRequestBuilder,
    ListRequestBuilder, Model, ModelsResource, ModerationsResource, encode_path_segment,
};

/// 表示 legacy completions 接口返回值。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Completion {
    /// 补全 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 创建时间。
    pub created: Option<i64>,
    /// 模型 ID。
    #[serde(default)]
    pub model: String,
    /// 候选项。
    #[serde(default)]
    pub choices: Vec<CompletionChoice>,
    /// 用量统计。
    pub usage: Option<CompletionUsage>,
    /// 系统指纹。
    pub system_fingerprint: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 legacy completions 候选项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionChoice {
    /// 结束原因。
    pub finish_reason: Option<String>,
    /// 候选索引。
    pub index: u32,
    /// token 级 logprobs。
    pub logprobs: Option<CompletionLogProbs>,
    /// 生成文本。
    #[serde(default)]
    pub text: String,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 legacy completions 的 token 级 logprobs。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionLogProbs {
    /// token 偏移量。
    #[serde(default)]
    pub text_offset: Vec<i64>,
    /// token logprob。
    #[serde(default)]
    pub token_logprobs: Vec<f64>,
    /// token 列表。
    #[serde(default)]
    pub tokens: Vec<String>,
    /// top logprobs。
    #[serde(default)]
    pub top_logprobs: Vec<BTreeMap<String, f64>>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 legacy completions 的用量统计。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionUsage {
    /// completion token 数。
    pub completion_tokens: u64,
    /// prompt token 数。
    pub prompt_tokens: u64,
    /// 总 token 数。
    pub total_tokens: u64,
    /// prompt token 明细。
    pub prompt_tokens_details: Option<Value>,
    /// completion token 明细。
    pub completion_tokens_details: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 moderation 接口返回值。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModerationCreateResponse {
    /// 请求 ID。
    pub id: String,
    /// 模型 ID。
    #[serde(default)]
    pub model: String,
    /// 分类结果列表。
    #[serde(default)]
    pub results: Vec<ModerationResult>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示单个 moderation 结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModerationResult {
    /// 命中的分类布尔值。
    #[serde(default)]
    pub categories: BTreeMap<String, bool>,
    /// 分类对应的输入类型。
    #[serde(default)]
    pub category_applied_input_types: BTreeMap<String, Vec<String>>,
    /// 分类分数。
    #[serde(default)]
    pub category_scores: BTreeMap<String, f64>,
    /// 是否命中任一分类。
    #[serde(default)]
    pub flagged: bool,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl CompletionsResource {
    /// 创建 completions 请求构建器。
    pub fn create(&self) -> JsonRequestBuilder<Completion> {
        let endpoint = endpoints::core::COMPLETIONS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }
}

impl EmbeddingsResource {
    /// 创建 embeddings 请求构建器。
    pub fn create(&self) -> JsonRequestBuilder<EmbeddingResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "embeddings.create",
            Method::POST,
            "/embeddings",
        )
    }
}

impl ModerationsResource {
    /// 创建 moderation 请求。
    pub fn create(&self) -> JsonRequestBuilder<ModerationCreateResponse> {
        let endpoint = endpoints::core::MODERATIONS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }
}

impl ModelsResource {
    /// 列出模型。
    pub fn list(&self) -> ListRequestBuilder<Model> {
        ListRequestBuilder::new(self.client.clone(), "models.list", "/models")
    }

    /// 获取单个模型。
    pub fn retrieve(&self, model_id: impl Into<String>) -> JsonRequestBuilder<Model> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "models.retrieve",
            Method::GET,
            format!("/models/{}", encode_path_segment(model_id.into())),
        )
    }

    /// 删除模型。
    pub fn delete(&self, model_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "models.delete",
            Method::DELETE,
            format!("/models/{}", encode_path_segment(model_id.into())),
        )
    }
}
