//! File namespace implementations.

use http::Method;

use super::{
    BytesRequestBuilder, DeleteResponse, FileObject, FilesResource, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment,
};

impl FilesResource {
    /// 创建文件上传请求。
    pub fn create(&self) -> JsonRequestBuilder<FileObject> {
        JsonRequestBuilder::new(self.client.clone(), "files.create", Method::POST, "/files")
    }

    /// 获取文件对象。
    pub fn retrieve(&self, file_id: impl Into<String>) -> JsonRequestBuilder<FileObject> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "files.retrieve",
            Method::GET,
            format!("/files/{}", encode_path_segment(file_id.into())),
        )
    }

    /// 列出文件。
    pub fn list(&self) -> ListRequestBuilder<FileObject> {
        ListRequestBuilder::new(self.client.clone(), "files.list", "/files")
    }

    /// 删除文件。
    pub fn delete(&self, file_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "files.delete",
            Method::DELETE,
            format!("/files/{}", encode_path_segment(file_id.into())),
        )
    }

    /// 获取文件内容。
    pub fn content(&self, file_id: impl Into<String>) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "files.content",
            Method::GET,
            format!("/files/{}/content", encode_path_segment(file_id.into())),
        )
    }
}
