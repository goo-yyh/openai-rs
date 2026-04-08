//! Upload namespace implementations.

use std::collections::BTreeMap;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::generated::endpoints;

use super::{
    JsonRequestBuilder, UploadObject, UploadPartsResource, UploadsResource, encode_path_segment,
};

/// 表示 upload part 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadPart {
    /// part ID。
    pub id: String,
    /// 创建时间。
    pub created_at: Option<u64>,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 所属 upload ID。
    pub upload_id: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

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
    pub fn create(&self, upload_id: impl Into<String>) -> JsonRequestBuilder<UploadPart> {
        let endpoint = endpoints::uploads::UPLOADS_PARTS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("upload_id", &encode_path_segment(upload_id.into()))]),
        )
    }
}
