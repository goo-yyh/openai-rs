//! Skill namespace implementations.

use http::Method;

use super::{
    BytesRequestBuilder, DeleteResponse, JsonRequestBuilder, ListRequestBuilder, Skill,
    SkillVersion, SkillVersionsContentResource, SkillVersionsResource, SkillsContentResource,
    SkillsResource, encode_path_segment,
};

impl SkillsResource {
    /// 创建 skill。
    pub fn create(&self) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.create",
            Method::POST,
            "/skills",
        )
    }

    /// 获取 skill。
    pub fn retrieve(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.retrieve",
            Method::GET,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
        )
    }

    /// 更新 skill。
    pub fn update(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<Skill> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.update",
            Method::POST,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
        )
    }

    /// 列出 skills。
    pub fn list(&self) -> ListRequestBuilder<Skill> {
        ListRequestBuilder::new(self.client.clone(), "skills.list", "/skills")
    }

    /// 删除 skill。
    pub fn delete(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.delete",
            Method::DELETE,
            format!("/skills/{}", encode_path_segment(skill_id.into())),
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
        BytesRequestBuilder::new(
            self.client.clone(),
            "skills.content.retrieve",
            Method::GET,
            format!("/skills/{}/content", encode_path_segment(skill_id.into())),
        )
    }
}

impl SkillVersionsResource {
    /// 创建 skill version。
    pub fn create(&self, skill_id: impl Into<String>) -> JsonRequestBuilder<SkillVersion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.create",
            Method::POST,
            format!("/skills/{}/versions", encode_path_segment(skill_id.into())),
        )
    }

    /// 获取 skill version。
    pub fn retrieve(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<SkillVersion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.retrieve",
            Method::GET,
            format!(
                "/skills/{}/versions/{}",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
        )
    }

    /// 列出 skill versions。
    pub fn list(&self, skill_id: impl Into<String>) -> ListRequestBuilder<SkillVersion> {
        ListRequestBuilder::new(
            self.client.clone(),
            "skills.versions.list",
            format!("/skills/{}/versions", encode_path_segment(skill_id.into())),
        )
    }

    /// 删除 skill version。
    pub fn delete(
        &self,
        skill_id: impl Into<String>,
        version_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "skills.versions.delete",
            Method::DELETE,
            format!(
                "/skills/{}/versions/{}",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
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
        BytesRequestBuilder::new(
            self.client.clone(),
            "skills.versions.content.retrieve",
            Method::GET,
            format!(
                "/skills/{}/versions/{}/content",
                encode_path_segment(skill_id.into()),
                encode_path_segment(version_id.into())
            ),
        )
    }
}
