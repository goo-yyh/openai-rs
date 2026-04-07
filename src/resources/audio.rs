//! Audio namespace implementations.

use super::{
    AudioResource, AudioSpeechRequestBuilder, AudioSpeechResource,
    AudioTranscriptionRequestBuilder, AudioTranscriptionsResource, AudioTranslationRequestBuilder,
    AudioTranslationsResource,
};

impl AudioResource {
    /// 返回 speech 子资源。
    pub fn speech(&self) -> AudioSpeechResource {
        AudioSpeechResource::new(self.client.clone())
    }

    /// 返回 transcriptions 子资源。
    pub fn transcriptions(&self) -> AudioTranscriptionsResource {
        AudioTranscriptionsResource::new(self.client.clone())
    }

    /// 返回 translations 子资源。
    pub fn translations(&self) -> AudioTranslationsResource {
        AudioTranslationsResource::new(self.client.clone())
    }
}

impl AudioSpeechResource {
    /// 创建语音合成请求。
    pub fn create(&self) -> AudioSpeechRequestBuilder {
        AudioSpeechRequestBuilder::new(self.client.clone())
    }

    /// 创建 SSE 语音合成请求。
    ///
    /// 该请求会自动在请求体中追加 `stream_format = "sse"`。
    pub fn stream(&self) -> AudioSpeechRequestBuilder {
        AudioSpeechRequestBuilder::stream(self.client.clone())
    }
}

impl AudioTranscriptionsResource {
    /// 创建转写请求。
    pub fn create(&self) -> AudioTranscriptionRequestBuilder {
        AudioTranscriptionRequestBuilder::new(self.client.clone(), false)
    }

    /// 创建流式转写请求。
    ///
    /// 该请求会自动在请求体中追加 `stream = true`。
    pub fn stream(&self) -> AudioTranscriptionRequestBuilder {
        AudioTranscriptionRequestBuilder::new(self.client.clone(), true)
    }
}

impl AudioTranslationsResource {
    /// 创建翻译请求。
    pub fn create(&self) -> AudioTranslationRequestBuilder {
        AudioTranslationRequestBuilder::new(self.client.clone())
    }
}
