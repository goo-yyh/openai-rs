//! Image namespace implementations.

use http::Method;

use super::{
    ImageGenerateRequestBuilder, ImageGenerationResponse, ImagesResource, JsonRequestBuilder,
};

impl ImagesResource {
    /// 创建图像生成请求。
    pub fn generate(&self) -> ImageGenerateRequestBuilder {
        ImageGenerateRequestBuilder::new(self.client.clone())
    }

    /// 创建图像编辑请求。
    pub fn edit(&self) -> JsonRequestBuilder<ImageGenerationResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "images.edit",
            Method::POST,
            "/images/edits",
        )
    }

    /// 创建图像变体请求。
    pub fn create_variation(&self) -> JsonRequestBuilder<ImageGenerationResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "images.create_variation",
            Method::POST,
            "/images/variations",
        )
    }
}
