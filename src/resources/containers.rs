//! Container namespace implementations.

use http::Method;

use super::{
    BytesRequestBuilder, Container, ContainerFile, ContainerFilesContentResource,
    ContainerFilesResource, ContainersResource, DeleteResponse, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment,
};

impl ContainersResource {
    /// 创建 container。
    pub fn create(&self) -> JsonRequestBuilder<Container> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.create",
            Method::POST,
            "/containers",
        )
    }

    /// 获取 container。
    pub fn retrieve(&self, container_id: impl Into<String>) -> JsonRequestBuilder<Container> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.retrieve",
            Method::GET,
            format!("/containers/{}", encode_path_segment(container_id.into())),
        )
    }

    /// 列出 containers。
    pub fn list(&self) -> ListRequestBuilder<Container> {
        ListRequestBuilder::new(self.client.clone(), "containers.list", "/containers")
    }

    /// 删除 container。
    pub fn delete(&self, container_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.delete",
            Method::DELETE,
            format!("/containers/{}", encode_path_segment(container_id.into())),
        )
    }

    /// 返回 files 子资源。
    pub fn files(&self) -> ContainerFilesResource {
        ContainerFilesResource::new(self.client.clone())
    }
}

impl ContainerFilesResource {
    /// 创建 container file。
    pub fn create(&self, container_id: impl Into<String>) -> JsonRequestBuilder<ContainerFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.create",
            Method::POST,
            format!(
                "/containers/{}/files",
                encode_path_segment(container_id.into())
            ),
        )
    }

    /// 获取 container file。
    pub fn retrieve(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<ContainerFile> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.retrieve",
            Method::GET,
            format!(
                "/containers/{}/files/{}",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 列出 container files。
    pub fn list(&self, container_id: impl Into<String>) -> ListRequestBuilder<ContainerFile> {
        ListRequestBuilder::new(
            self.client.clone(),
            "containers.files.list",
            format!(
                "/containers/{}/files",
                encode_path_segment(container_id.into())
            ),
        )
    }

    /// 删除 container file。
    pub fn delete(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "containers.files.delete",
            Method::DELETE,
            format!(
                "/containers/{}/files/{}",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> ContainerFilesContentResource {
        ContainerFilesContentResource::new(self.client.clone())
    }
}

impl ContainerFilesContentResource {
    /// 获取 container file 内容。
    pub fn retrieve(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "containers.files.content.retrieve",
            Method::GET,
            format!(
                "/containers/{}/files/{}/content",
                encode_path_segment(container_id.into()),
                encode_path_segment(file_id.into())
            ),
        )
    }
}
