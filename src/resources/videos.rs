//! Video namespace implementations.

use http::Method;

use crate::generated::endpoints;

use super::{
    BytesRequestBuilder, DeleteResponse, JsonRequestBuilder, ListRequestBuilder, Video,
    VideoCharacter, VideosResource, encode_path_segment,
};

impl VideosResource {
    /// 创建视频。
    pub fn create(&self) -> JsonRequestBuilder<Video> {
        let endpoint = endpoints::videos::VIDEOS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取视频。
    pub fn retrieve(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }

    /// 列出视频。
    pub fn list(&self) -> ListRequestBuilder<Video> {
        let endpoint = endpoints::videos::VIDEOS_LIST;
        ListRequestBuilder::new(self.client.clone(), endpoint.id, endpoint.template)
    }

    /// 删除视频。
    pub fn delete(&self, video_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }

    /// 编辑视频。
    pub fn edit(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_EDIT;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }

    /// 扩展视频。
    pub fn extend(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_EXTEND;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }

    /// 创建角色。
    pub fn create_character(&self) -> JsonRequestBuilder<VideoCharacter> {
        let endpoint = endpoints::videos::VIDEOS_CREATE_CHARACTER;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取角色。
    pub fn get_character(
        &self,
        character_id: impl Into<String>,
    ) -> JsonRequestBuilder<VideoCharacter> {
        let character_id = encode_path_segment(character_id.into());
        let endpoint = endpoints::videos::VIDEOS_GET_CHARACTER;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("character_id", &character_id)]),
        )
    }

    /// 下载视频内容。
    pub fn download_content(&self, video_id: impl Into<String>) -> BytesRequestBuilder {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_DOWNLOAD_CONTENT;
        BytesRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }

    /// 混剪视频。
    pub fn remix(&self, video_id: impl Into<String>) -> JsonRequestBuilder<Video> {
        let video_id = encode_path_segment(video_id.into());
        let endpoint = endpoints::videos::VIDEOS_REMIX;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("video_id", &video_id)]),
        )
    }
}
