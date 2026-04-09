//! Vector Stores 命名空间实现。

use std::collections::BTreeMap;

use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use super::{
    DeleteResponse, JsonRequestBuilder, ListRequestBuilder, VectorStoreFileBatchesResource,
    VectorStoreFilesResource, VectorStoresResource, encode_path_segment,
};
use crate::Page;
use crate::generated::endpoints;

/// Vector store 元数据。
pub type VectorStoreMetadata = BTreeMap<String, String>;

/// Vector store 文件属性。
pub type VectorStoreAttributes = BTreeMap<String, VectorStoreAttributeValue>;

/// 表示 metadata / attributes 中的标量值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum VectorStoreAttributeValue {
    /// 字符串值。
    String(String),
    /// 数字值。
    Number(Number),
    /// 布尔值。
    Bool(bool),
}

/// 表示 vector store 文件计数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreFileCounts {
    /// 已取消文件数量。
    pub cancelled: Option<u64>,
    /// 已完成文件数量。
    pub completed: Option<u64>,
    /// 失败文件数量。
    pub failed: Option<u64>,
    /// 处理中数量。
    pub in_progress: Option<u64>,
    /// 文件总数。
    pub total: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 过期策略。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreExpiresAfter {
    /// 锚点。
    pub anchor: Option<String>,
    /// 过期天数。
    pub days: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示静态分块策略配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreStaticFileChunkingStrategy {
    /// chunk 重叠 token 数。
    pub chunk_overlap_tokens: Option<u64>,
    /// 每个 chunk 的最大 token 数。
    pub max_chunk_size_tokens: Option<u64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示文件分块策略。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VectorStoreFileChunkingStrategy {
    /// 静态分块。
    Static {
        /// 静态分块配置。
        #[serde(rename = "static")]
        configuration: VectorStoreStaticFileChunkingStrategy,
    },
    /// 旧文件或未知策略返回的 other 类型。
    Other,
    /// 向前兼容的未知策略。
    #[serde(other)]
    Unknown,
}

/// 表示 vector store 文件处理错误。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreFileLastError {
    /// 错误码。
    pub code: Option<String>,
    /// 错误描述。
    pub message: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 文件内容项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreFileContent {
    /// 内容文本。
    pub text: Option<String>,
    /// 内容类型。
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 搜索命中的内容片段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreSearchContent {
    /// 返回的文本内容。
    pub text: Option<String>,
    /// 内容类型。
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 搜索命中项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStoreSearchResult {
    /// 文件属性过滤数据。
    pub attributes: Option<VectorStoreAttributes>,
    /// 内容片段。
    #[serde(default)]
    pub content: Vec<VectorStoreSearchContent>,
    /// 文件 ID。
    pub file_id: Option<String>,
    /// 文件名。
    pub filename: Option<String>,
    /// 相似度分数。
    pub score: Option<f64>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示 vector store 对象。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorStore {
    /// 对象 ID。
    pub id: String,
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 创建时间戳。
    pub created_at: Option<u64>,
    /// 描述。
    pub description: Option<String>,
    /// 名称。
    pub name: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 最后活跃时间。
    pub last_active_at: Option<u64>,
    /// 占用字节数。
    pub usage_bytes: Option<u64>,
    /// 文件计数。
    pub file_counts: Option<VectorStoreFileCounts>,
    /// 元数据。
    pub metadata: Option<VectorStoreMetadata>,
    /// 过期策略。
    pub expires_after: Option<VectorStoreExpiresAfter>,
    /// 过期时间。
    pub expires_at: Option<u64>,
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
    /// 创建时间戳。
    pub created_at: Option<u64>,
    /// 关联的 vector store ID。
    pub vector_store_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 最近错误。
    pub last_error: Option<VectorStoreFileLastError>,
    /// 占用字节数。
    pub usage_bytes: Option<u64>,
    /// 属性。
    pub attributes: Option<VectorStoreAttributes>,
    /// 分块策略。
    pub chunking_strategy: Option<VectorStoreFileChunkingStrategy>,
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
    /// 创建时间戳。
    pub created_at: Option<u64>,
    /// 关联的 vector store ID。
    pub vector_store_id: Option<String>,
    /// 状态。
    pub status: Option<String>,
    /// 文件计数。
    pub file_counts: Option<VectorStoreFileCounts>,
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
    pub data: Vec<VectorStoreSearchResult>,
    /// 搜索查询。
    pub search_query: Option<String>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl VectorStoresResource {
    /// 创建 vector store。
    pub fn create(&self) -> JsonRequestBuilder<VectorStore> {
        let endpoint = endpoints::vector_stores::VECTOR_STORES_CREATE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取 vector store。
    pub fn retrieve(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<VectorStore> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_RETRIEVE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 更新 vector store。
    pub fn update(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<VectorStore> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_UPDATE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 列出 vector store。
    pub fn list(&self) -> ListRequestBuilder<VectorStore> {
        let endpoint = endpoints::vector_stores::VECTOR_STORES_LIST;
        vector_store_list(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 删除 vector store。
    pub fn delete(&self, vector_store_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_DELETE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 搜索 vector store。
    pub fn search(
        &self,
        vector_store_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreSearchResponse> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_SEARCH;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
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
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_CREATE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 获取 vector store 文件。
    pub fn retrieve(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFile> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_RETRIEVE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("vector_store_id", &vector_store_id), ("file_id", &file_id)]),
        )
    }

    /// 更新 vector store 文件。
    pub fn update(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFile> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_UPDATE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("vector_store_id", &vector_store_id), ("file_id", &file_id)]),
        )
    }

    /// 列出 vector store 文件。
    pub fn list(&self, vector_store_id: impl Into<String>) -> ListRequestBuilder<VectorStoreFile> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_LIST;
        vector_store_list(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 删除 vector store 文件。
    pub fn delete(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_DELETE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("vector_store_id", &vector_store_id), ("file_id", &file_id)]),
        )
    }

    /// 获取 vector store 文件内容。
    pub fn content(
        &self,
        vector_store_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<Page<VectorStoreFileContent>> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILES_CONTENT;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("vector_store_id", &vector_store_id), ("file_id", &file_id)]),
        )
    }
}

impl VectorStoreFileBatchesResource {
    /// 创建 file batch。
    pub fn create(
        &self,
        vector_store_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILE_BATCHES_CREATE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("vector_store_id", &vector_store_id)]),
        )
    }

    /// 获取 file batch。
    pub fn retrieve(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let batch_id = encode_path_segment(batch_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILE_BATCHES_RETRIEVE;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[
                ("vector_store_id", &vector_store_id),
                ("batch_id", &batch_id),
            ]),
        )
    }

    /// 取消 file batch。
    pub fn cancel(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> JsonRequestBuilder<VectorStoreFileBatch> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let batch_id = encode_path_segment(batch_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILE_BATCHES_CANCEL;
        vector_store_json(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[
                ("vector_store_id", &vector_store_id),
                ("batch_id", &batch_id),
            ]),
        )
    }

    /// 列出 file batch 文件。
    pub fn list_files(
        &self,
        vector_store_id: impl Into<String>,
        batch_id: impl Into<String>,
    ) -> ListRequestBuilder<VectorStoreFile> {
        let vector_store_id = encode_path_segment(vector_store_id.into());
        let batch_id = encode_path_segment(batch_id.into());
        let endpoint = endpoints::vector_stores::VECTOR_STORES_FILE_BATCHES_LIST_FILES;
        vector_store_list(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[
                ("vector_store_id", &vector_store_id),
                ("batch_id", &batch_id),
            ]),
        )
    }
}

fn vector_store_json<T>(
    client: crate::Client,
    endpoint_id: &'static str,
    method: Method,
    path: impl Into<String>,
) -> JsonRequestBuilder<T> {
    JsonRequestBuilder::new(client, endpoint_id, method, path)
        .extra_header("openai-beta", "assistants=v2")
}

fn vector_store_list<T>(
    client: crate::Client,
    endpoint_id: &'static str,
    path: impl Into<String>,
) -> ListRequestBuilder<T> {
    ListRequestBuilder::new(client, endpoint_id, path).extra_header("openai-beta", "assistants=v2")
}
