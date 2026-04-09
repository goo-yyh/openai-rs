//! Conversation namespace implementations.

use http::Method;

use crate::generated::endpoints;

use super::{
    Conversation, ConversationItem, ConversationItemsResource, ConversationsResource,
    DeleteResponse, JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

impl ConversationsResource {
    /// 创建 conversation。
    pub fn create(&self) -> JsonRequestBuilder<Conversation> {
        let endpoint = endpoints::conversations::CONVERSATIONS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.template,
        )
    }

    /// 获取 conversation。
    pub fn retrieve(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("conversation_id", &conversation_id)]),
        )
    }

    /// 更新 conversation。
    pub fn update(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<Conversation> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_UPDATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("conversation_id", &conversation_id)]),
        )
    }

    /// 删除 conversation。
    pub fn delete(&self, conversation_id: impl Into<String>) -> JsonRequestBuilder<DeleteResponse> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("conversation_id", &conversation_id)]),
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
        let conversation_id = encode_path_segment(conversation_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_ITEMS_CREATE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::POST,
            endpoint.render(&[("conversation_id", &conversation_id)]),
        )
    }

    /// 获取 conversation item。
    pub fn retrieve(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<ConversationItem> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let item_id = encode_path_segment(item_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_ITEMS_RETRIEVE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::GET,
            endpoint.render(&[("conversation_id", &conversation_id), ("item_id", &item_id)]),
        )
    }

    /// 列出 conversation items。
    pub fn list(&self, conversation_id: impl Into<String>) -> ListRequestBuilder<ConversationItem> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_ITEMS_LIST;
        ListRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            endpoint.render(&[("conversation_id", &conversation_id)]),
        )
    }

    /// 删除 conversation item。
    pub fn delete(
        &self,
        conversation_id: impl Into<String>,
        item_id: impl Into<String>,
    ) -> JsonRequestBuilder<DeleteResponse> {
        let conversation_id = encode_path_segment(conversation_id.into());
        let item_id = encode_path_segment(item_id.into());
        let endpoint = endpoints::conversations::CONVERSATIONS_ITEMS_DELETE;
        JsonRequestBuilder::new(
            self.client.clone(),
            endpoint.id,
            Method::DELETE,
            endpoint.render(&[("conversation_id", &conversation_id), ("item_id", &item_id)]),
        )
    }
}
