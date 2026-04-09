//! Skill namespace implementations.

use http::Method;

use crate::generated::endpoints;

use super::{
    BytesRequestBuilder, DeleteResponse, JsonRequestBuilder, ListRequestBuilder, Skill,
    SkillVersion, SkillVersionsContentResource, SkillVersionsResource, SkillsContentResource,
    SkillsResource, encode_path_segment,
};

impl SkillsResource {
    /// 创建 skill。
    pub fn create(&self) -> JsonRequestBuilder<Skill> {
        let endpoint = endpoints::skills::SKILLS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取 skill。
    pub fn retrieve(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }

    /// 更新 skill。
    pub fn update(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_UPDATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }

    /// 列出 skills。
    pub fn list(&self) -> ListRequestBuilder<Skill> {
        let endpoint = endpoints::skills::SKILLS_LIST;
        ListRequestBuilder::new(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 删除 skill。
    pub fn delete(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> SkillsContentResource {
        SkillsContentResource::new(self.client.clone())
    }

    /// 返回 versions 子资源。
    pub fn versions(&self) -> SkillVersionsResource {
        SkillVersionsResource::new(self.client.clone())
    }
}

impl SkillsContentResource {
    /// 获取 skill 内容。
    pub fn retrieve(&self, skill_id: impl Into<String>) -> BytesRequestBuilder {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_CONTENT_RETRIEVE;
        BytesRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }
}

impl SkillVersionsResource {
    /// 创建 skill version。
    pub fn create(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<SkillVersion> {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_VERSIONS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }

    /// 获取 skill version。
    pub fn retrieve(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<SkillVersion> {
        let skill_id = encode_path_segment(skill_id.into());
        let version_id = encode_path_segment(version_id.into());
        let endpoint = endpoints::skills::SKILLS_VERSIONS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("skill_id", &skill_id), ("version_id", &version_id)]),
        )
    }

    /// 列出 skill versions。
    pub fn list(&self, skill_id: impl Into<String>) -> ListRequestBuilder<SkillVersion> {
        let skill_id = encode_path_segment(skill_id.into());
        let endpoint = endpoints::skills::SKILLS_VERSIONS_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("skill_id", &skill_id)]),
        )
    }

    /// 删除 skill version。
    pub fn delete(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        let skill_id = encode_path_segment(skill_id.into());
        let version_id = encode_path_segment(version_id.into());
        let endpoint = endpoints::skills::SKILLS_VERSIONS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("skill_id", &skill_id), ("version_id", &version_id)]),
        )
    }

    /// 返回 content 子资源。
    pub fn content(&self) -> SkillVersionsContentResource {
        SkillVersionsContentResource::new(self.client.clone())
    }
}

impl SkillVersionsContentResource {
    /// 获取 skill version 内容。
    pub fn retrieve(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> BytesRequestBuilder {
        let skill_id = encode_path_segment(skill_id.into());
        let version_id = encode_path_segment(version_id.into());
        let endpoint = endpoints::skills::SKILLS_VERSIONS_CONTENT_RETRIEVE;
        BytesRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("skill_id", &skill_id), ("version_id", &version_id)]),
        )
    }
}
