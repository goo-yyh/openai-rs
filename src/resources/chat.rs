//! Chat 命名空间实现。

use http::Method;
use serde_json::Value;

#[cfg(feature = "structured-output")]
use super::ChatCompletionParseRequestBuilder;
#[cfg(feature = "tool-runner")]
use super::ChatCompletionRunToolsRequestBuilder;
use super::{
    ChatCompletion, ChatCompletionCreateRequestBuilder, ChatCompletionMessagesResource,
    ChatCompletionStreamRequestBuilder, ChatCompletionsResource, ChatResource, DeleteResponse,
    JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

impl ChatResource {
    /// 返回聊天补全资源。
    pub fn completions(&self) -> ChatCompletionsResource {
        ChatCompletionsResource::new(self.client.clone())
    }
}

impl ChatCompletionsResource {
    /// 创建聊天补全请求构建器。
    pub fn create(&self) -> ChatCompletionCreateRequestBuilder {
        ChatCompletionCreateRequestBuilder::new(self.client.clone())
    }

    /// 创建聊天补全流式请求构建器。
    pub fn stream(&self) -> ChatCompletionStreamRequestBuilder {
        ChatCompletionStreamRequestBuilder::new(self.client.clone())
    }

    /// 创建结构化解析请求构建器。
    #[cfg(feature = "structured-output")]
    #[cfg_attr(docsrs, doc(cfg(feature = "structured-output")))]
    pub fn parse<T>(&self) -> ChatCompletionParseRequestBuilder<T> {
        ChatCompletionParseRequestBuilder::new(self.client.clone())
    }

    /// 创建工具运行构建器。
    #[cfg(feature = "tool-runner")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tool-runner")))]
    pub fn run_tools(&self) -> ChatCompletionRunToolsRequestBuilder {
        ChatCompletionRunToolsRequestBuilder::new(self.client.clone())
    }

    /// 根据 ID 获取聊天补全对象。
    pub fn retrieve(&self, id: impl Into<String>) -> JsonRequestBuilder<ChatCompletion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.retrieve",
            Method::GET,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 更新聊天补全对象。
    pub fn update(&self, id: impl Into<String>) -> JsonRequestBuilder<ChatCompletion> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.update",
            Method::POST,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 列出聊天补全对象。
    pub fn list(&self) -> ListRequestBuilder<ChatCompletion> {
        ListRequestBuilder::new(
            self.client.clone(),
            "chat.completions.list",
            "/chat/completions",
        )
    }

    /// 删除聊天补全对象。
    pub fn delete(&self, id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "chat.completions.delete",
            Method::DELETE,
            format!("/chat/completions/{}", encode_path_segment(id.into())),
        )
    }

    /// 返回聊天补全消息子资源。
    pub fn messages(&self) -> ChatCompletionMessagesResource {
        ChatCompletionMessagesResource::new(self.client.clone())
    }
}

impl ChatCompletionMessagesResource {
    /// 列出某个聊天补全下的消息。
    pub fn list(&self, completion_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "chat.completions.messages.list",
            format!(
                "/chat/completions/{}/messages",
                encode_path_segment(completion_id.into())
            ),
        )
    }
}
