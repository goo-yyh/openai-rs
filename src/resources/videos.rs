//! Video namespace implementations.

use http::Method;

use super::{
    BytesRequestBuilder, DeleteResponse, JsonRequestBuilder, ListRequestBuilder, Video,
    VideoCharacter, VideosResource, encode_path_segment,
};

impl VideosResource {
    /// 创建视频。
    pub fn create(&self) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.create",
            Method::POST,
            "/videos",
        )
    }

    /// 获取视频。
    pub fn retrieve(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.retrieve",
            Method::GET,
            format!("/videos/{}", encode_path_segment(video_id.into())),
        )
    }

    /// 列出视频。
    pub fn list(&self) -> ListRequestBuilder<Video> {
        ListRequestBuilder::new(self.client.clone(), "videos.list", "/videos")
    }

    /// 删除视频。
    pub fn delete(&self, video_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.delete",
            Method::DELETE,
            format!("/videos/{}", encode_path_segment(video_id.into())),
        )
    }

    /// 编辑视频。
    pub fn edit(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.edit",
            Method::POST,
            format!("/videos/{}/edit", encode_path_segment(video_id.into())),
        )
    }

    /// 扩展视频。
    pub fn extend(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.extend",
            Method::POST,
            format!("/videos/{}/extend", encode_path_segment(video_id.into())),
        )
    }

    /// 创建角色。
    pub fn create_character(&self) -> JsonRequestBuilder<VideoCharacter> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.create_character",
            Method::POST,
            "/videos/characters",
        )
    }

    /// 获取角色。
    pub fn get_character(
        &self,
        character_id: impl Into<String>,
    ) -> JsonRequestBuilder<VideoCharacter> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.get_character",
            Method::GET,
            format!(
                "/videos/characters/{}",
                encode_path_segment(character_id.into())
            ),
        )
    }

    /// 下载视频内容。
    pub fn download_content(&self, video_id: impl Into<String>) -> BytesRequestBuilder {
        BytesRequestBuilder::new(
            self.client.clone(),
            "videos.download_content",
            Method::GET,
            format!("/videos/{}/content", encode_path_segment(video_id.into())),
        )
    }

    /// 混剪视频。
    pub fn remix(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "videos.remix",
            Method::POST,
            format!("/videos/{}/remix", encode_path_segment(video_id.into())),
        )
    }
}
