//! Batch namespace implementations.

use http::Method;

use crate::generated::endpoints;

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
        let endpoint = endpoints::batches::BATCHES_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("batch_id", &encode_path_segment(batch_id.into()))]),
        )
    }

    /// 列出 batches。
    pub fn list(&self) -> ListRequestBuilder<Batch> {
        let endpoint = endpoints::batches::BATCHES_LIST;
        ListRequestBuilder::new(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 取消 batch。
    pub fn cancel(&self, batch_id: impl Into<String>) -> JsonRequestBuilder<Batch> {
        let endpoint = endpoints::batches::BATCHES_CANCEL;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("batch_id", &encode_path_segment(batch_id.into()))]),
        )
    }
}
