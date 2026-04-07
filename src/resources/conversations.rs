//! Conversation namespace implementations.

use http::Method;

use super::{
    Conversation, ConversationItem, ConversationItemsResource, ConversationsResource,
    DeleteResponse, JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

impl ConversationsResource {
    /// 创建 conversation。
    pub fn create(&self) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.create",
            Method::POST,
            "/conversations",
        )
    }

    /// 获取 conversation。
    pub fn retrieve(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.retrieve",
            Method::GET,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 更新 conversation。
    pub fn update(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.update",
            Method::POST,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 删除 conversation。
    pub fn delete(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.delete",
            Method::DELETE,
            format!(
                "/conversations/{}",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 返回 items 子资源。
    pub fn items(&self) -> ConversationItemsResource {
        ConversationItemsResource::new(self.client.clone())
    }
}

impl ConversationItemsResource {
    /// 创建 conversation item。
    pub fn create(
        &self,
        conversation_id: impl Into<String>,
    ) -> JsonRequestBuilder<ConversationItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.create",
            Method::POST,
            format!(
                "/conversations/{}/items",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 获取 conversation item。
    pub fn retrieve(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<ConversationItem> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.retrieve",
            Method::GET,
            format!(
                "/conversations/{}/items/{}",
                encode_path_segment(conversation_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }

    /// 列出 conversation items。
    pub fn list(&self, conversation_id: impl Into<String>) -> ListRequestBuilder<ConversationItem> {
        ListRequestBuilder::new(
            self.client.clone(),
            "conversations.items.list",
            format!(
                "/conversations/{}/items",
                encode_path_segment(conversation_id.into())
            ),
        )
    }

    /// 删除 conversation item。
    pub fn delete(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "conversations.items.delete",
            Method::DELETE,
            format!(
                "/conversations/{}/items/{}",
                encode_path_segment(conversation_id.into()),
                encode_path_segment(item_id.into())
            ),
        )
    }
}
