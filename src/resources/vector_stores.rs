//! Vector Stores 命名空间实现。

use std::collections::BTreeMap;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    BytesRequestBuilder, DeleteResponse, JsonRequestBuilder, ListRequestBuilder,
    VectorStoreFileBatchesResource, VectorStoreFilesResource, VectorStoresResource,
    encode_path_segment,
};

/// 表示 vector store 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStore {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 名称。
    pub name: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 占用字节数。
    pub usage_bytes: Option<u64>,
    /// 文件计数。
    pub file_counts: Option<Value>,
    /// 元数据。
    pub metadata: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 文件对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreFile {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 关联的 vector store ID。
    pub vector_store_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 占用字节数。
    pub usage_bytes: Option<u64>,
    /// 属性。
    pub attributes: Option<Value>,
    /// 分块策略。
    pub chunking_strategy: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store file batch 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreFileBatch {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 关联的 vector store ID。
    pub vector_store_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 文件计数。
    pub file_counts: Option<Value>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 搜索返回值。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreSearchResponse {
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 搜索结果。
    #[serde(default)]
    pub data: Vec<Value>,
    /// 搜索查询。
    pub search_query: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl VectorStoresResource {
    /// 创建 vector store。
    pub fn create(&self) -> JsonRequestBuilder<VectorStore> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.create",
            Method::POST,
            "/vector_stores",
        )
    }

    /// 获取 vector store。
    pub fn retrieve(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<VectorStore> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.retrieve",
            Method::GET,
            format!(
                "/vector_stores/{}",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 更新 vector store。
    pub fn update(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<VectorStore> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.update",
            Method::POST,
            format!(
                "/vector_stores/{}",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 列出 vector store。
    pub fn list(&self) -> ListRequestBuilder<VectorStore> {
        ListRequestBuilder::new(self.client.clone(), "vector_stores.list", "/vector_stores")
    }

    /// 删除 vector store。
    pub fn delete(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.delete",
            Method::DELETE,
            format!(
                "/vector_stores/{}",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 搜索 vector store。
    pub fn search(
        &self,
        vector_store_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreSearchResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.search",
            Method::POST,
            format!(
                "/vector_stores/{}/search",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 返回 files 子资源。
    pub fn files(&self) -> VectorStoreFilesResource {
        VectorStoreFilesResource::new(self.client.clone())
    }

    /// 返回 file_batches 子资源。
    pub fn file_batches(&self) -> VectorStoreFileBatchesResource {
        VectorStoreFileBatchesResource::new(self.client.clone())
    }
}

impl VectorStoreFilesResource {
    /// 创建 vector store 文件。
    pub fn create(
        &self,
        vector_store_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.create",
            Method::POST,
            format!(
                "/vector_stores/{}/files",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 获取 vector store 文件。
    pub fn retrieve(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.retrieve",
            Method::GET,
            format!(
                "/vector_stores/{}/files/{}",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 更新 vector store 文件。
    pub fn update(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.update",
            Method::POST,
            format!(
                "/vector_stores/{}/files/{}",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 列出 vector store 文件。
    pub fn list(&self, vector_store_id: impl Into<String>) -> ListRequestBuilder<VectorStoreFile> {
        ListRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.list",
            format!(
                "/vector_stores/{}/files",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 删除 vector store 文件。
    pub fn delete(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.delete",
            Method::DELETE,
            format!(
                "/vector_stores/{}/files/{}",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 获取 vector store 文件内容。
    pub fn content(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "vector_stores.files.content",
            Method::GET,
            format!(
                "/vector_stores/{}/files/{}/content",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }
}

impl VectorStoreFileBatchesResource {
    /// 创建 file batch。
    pub fn create(
        &self,
        vector_store_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.file_batches.create",
            Method::POST,
            format!(
                "/vector_stores/{}/file_batches",
                encode_path_segment(vector_store_id.into())
            ),
        )
    }

    /// 获取 file batch。
    pub fn retrieve(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.file_batches.retrieve",
            Method::GET,
            format!(
                "/vector_stores/{}/file_batches/{}",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(batch_id.into())
            ),
        )
    }

    /// 取消 file batch。
    pub fn cancel(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "vector_stores.file_batches.cancel",
            Method::POST,
            format!(
                "/vector_stores/{}/file_batches/{}/cancel",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(batch_id.into())
            ),
        )
    }

    /// 列出 file batch 文件。
    pub fn list_files(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> ListRequestBuilder<VectorStoreFile> {
        ListRequestBuilder::new(
            self.client.clone(),
            "vector_stores.file_batches.list_files",
            format!(
                "/vector_stores/{}/file_batches/{}/files",
                encode_path_segment(vector_store_id.into()),
                encode_path_segment(batch_id.into())
            ),
        )
    }
}
