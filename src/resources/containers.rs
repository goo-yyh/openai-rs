//! Container namespace implementations.

use http::Method;

use crate::generated::endpoints;

use super::{
    BytesRequestBuilder, Container, ContainerFile, ContainerFilesContentResource,
    ContainerFilesResource, ContainersResource, DeleteResponse, JsonRequestBuilder,
    ListRequestBuilder, encode_path_segment,
};

impl ContainersResource {
    /// 创建 container。
    pub fn create(&self) -> JsonRequestBuilder<Container> {
        let endpoint = endpoints::containers::CONTAINERS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取 container。
    pub fn retrieve(&self, container_id: impl Into<String>) -> JsonRequestBuilder<Container> {
        let container_id = encode_path_segment(container_id.into());
        let endpoint = endpoints::containers::CONTAINERS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("container_id", &container_id)]),
        )
    }

    /// 列出 containers。
    pub fn list(&self) -> ListRequestBuilder<Container> {
        let endpoint = endpoints::containers::CONTAINERS_LIST;
        ListRequestBuilder::new(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 删除 container。
    pub fn delete(&self, container_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let container_id = encode_path_segment(container_id.into());
        let endpoint = endpoints::containers::CONTAINERS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("container_id", &container_id)]),
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
        let container_id = encode_path_segment(container_id.into());
        let endpoint = endpoints::containers::CONTAINERS_FILES_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("container_id", &container_id)]),
        )
    }

    /// 获取 container file。
    pub fn retrieve(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<ContainerFile> {
        let container_id = encode_path_segment(container_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::containers::CONTAINERS_FILES_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("container_id", &container_id), ("file_id", &file_id)]),
        )
    }

    /// 列出 container files。
    pub fn list(&self, container_id: impl Into<String>) -> ListRequestBuilder<ContainerFile> {
        let container_id = encode_path_segment(container_id.into());
        let endpoint = endpoints::containers::CONTAINERS_FILES_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("container_id", &container_id)]),
        )
    }

    /// 删除 container file。
    pub fn delete(
        &self,
        container_id: impl Into<String>,
        file_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        let container_id = encode_path_segment(container_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::containers::CONTAINERS_FILES_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("container_id", &container_id), ("file_id", &file_id)]),
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
        let container_id = encode_path_segment(container_id.into());
        let file_id = encode_path_segment(file_id.into());
        let endpoint = endpoints::containers::CONTAINERS_FILES_CONTENT_RETRIEVE;
        BytesRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("container_id", &container_id), ("file_id", &file_id)]),
        )
    }
}
