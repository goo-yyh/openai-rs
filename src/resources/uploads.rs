//! Upload namespace implementations.

use http::Method;
use serde_json::Value;

use super::{
    JsonRequestBuilder, UploadObject, UploadPartsResource, UploadsResource, encode_path_segment,
};

impl UploadsResource {
    /// 创建 upload。
    pub fn create(&self) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.create",
            Method::POST,
            "/uploads",
        )
    }

    /// 取消 upload。
    pub fn cancel(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.cancel",
            Method::POST,
            format!("/uploads/{}/cancel", encode_path_segment(upload_id.into())),
        )
    }

    /// 完成 upload。
    pub fn complete(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<UploadObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.complete",
            Method::POST,
            format!(
                "/uploads/{}/complete",
                encode_path_segment(upload_id.into())
            ),
        )
    }

    /// 返回 parts 子资源。
    pub fn parts(&self) -> UploadPartsResource {
        UploadPartsResource::new(self.client.clone())
    }
}

impl UploadPartsResource {
    /// 创建 upload part。
    pub fn create(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "uploads.parts.create",
            Method::POST,
            format!("/uploads/{}/parts", encode_path_segment(upload_id.into())),
        )
    }
}
