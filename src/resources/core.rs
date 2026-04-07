//! Core resource implementations that do not need additional namespace files.

use http::Method;
use serde_json::Value;

use super::{
    CompletionsResource, DeleteResponse, EmbeddingResponse, EmbeddingsResource, JsonRequestBuilder,
    ListRequestBuilder, Model, ModelsResource, ModerationsResource, encode_path_segment,
};

impl CompletionsResource {
    /// 创建 completions 请求构建器。
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "completions.create",
            Method::POST,
            "/completions",
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
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "moderations.create",
            Method::POST,
            "/moderations",
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
