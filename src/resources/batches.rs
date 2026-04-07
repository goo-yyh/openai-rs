//! Batch namespace implementations.

use http::Method;

use super::{
    Batch, BatchCreateRequestBuilder, BatchesResource, JsonRequestBuilder, ListRequestBuilder,
    encode_path_segment,
};

impl BatchesResource {
    /// 创建 batch。
    pub fn create(&self) -> BatchCreateRequestBuilder {
        BatchCreateRequestBuilder::new(self.client.clone())
    }

    /// 获取 batch。
    pub fn retrieve(&self, batch_id: impl Into<String>) -> JsonRequestBuilder<Batch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "batches.retrieve",
            Method::GET,
            format!("/batches/{}", encode_path_segment(batch_id.into())),
        )
    }

    /// 列出 batches。
    pub fn list(&self) -> ListRequestBuilder<Batch> {
        ListRequestBuilder::new(self.client.clone(), "batches.list", "/batches")
    }

    /// 取消 batch。
    pub fn cancel(&self, batch_id: impl Into<String>) -> JsonRequestBuilder<Batch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "batches.cancel",
            Method::POST,
            format!("/batches/{}/cancel", encode_path_segment(batch_id.into())),
        )
    }
}
