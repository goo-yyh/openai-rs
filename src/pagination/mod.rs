//! 分页相关类型。

use std::collections::BTreeMap;
use std::pin::Pin;

use async_stream::try_stream;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::PageRequestSpec;
use crate::error::{Error, Result};
/// 表示通用列表包裹结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
pub struct ListEnvelope<T> {
    /// 对象类型，通常为 `list`。
    #[serde(default)]
    pub object: String,
    /// 当前页数据。
    #[serde(default)]
    pub data: Vec<T>,
    /// 当前页首个对象 ID。
    pub first_id: Option<String>,
    /// 当前页最后一个对象 ID。
    pub last_id: Option<String>,
    /// 是否还有下一页。
    #[serde(default)]
    pub has_more: bool,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示一个普通页面对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
pub struct Page<T> {
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 页面数据。
    #[serde(default)]
    pub data: Vec<T>,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// 表示基于游标的页面对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPage<T> {
    /// 对象类型。
    #[serde(default)]
    pub object: String,
    /// 当前页数据。
    #[serde(default)]
    pub data: Vec<T>,
    /// 当前页首个对象 ID。
    pub first_id: Option<String>,
    /// 当前页最后一个对象 ID。
    pub last_id: Option<String>,
    /// 是否还有下一页。
    #[serde(default)]
    pub has_more: bool,
    /// 额外字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
    /// 后续分页请求信息。
    #[serde(skip)]
    pub(crate) next: Option<PageRequestSpec>,
}

impl<T> Default for CursorPage<T> {
    fn default() -> Self {
        Self {
            object: String::new(),
            data: Vec::new(),
            first_id: None,
            last_id: None,
            has_more: false,
            extra: BTreeMap::new(),
            next: None,
        }
    }
}

impl<T> From<ListEnvelope<T>> for CursorPage<T> {
    fn from(value: ListEnvelope<T>) -> Self {
        Self {
            object: value.object,
            data: value.data,
            first_id: value.first_id,
            last_id: value.last_id,
            has_more: value.has_more,
            extra: value.extra,
            next: None,
        }
    }
}

impl<T> CursorPage<T>
where
    T: Clone + Send + Sync + serde::de::DeserializeOwned + 'static,
{
    /// 绑定下一页请求元数据。
    pub fn with_next_request(mut self, next: Option<PageRequestSpec>) -> Self {
        self.next = next;
        self
    }

    /// 判断是否存在下一页。
    pub fn has_next_page(&self) -> bool {
        self.has_more && self.next.is_some()
    }

    /// 请求下一页。
    ///
    /// # Errors
    ///
    /// 当当前页没有下一页信息时返回错误。
    pub async fn next_page(&self) -> Result<Self> {
        let next = self
            .next
            .clone()
            .ok_or_else(|| Error::InvalidConfig("当前页面没有下一页游标".into()))?;
        let client = next.client.clone();
        client.fetch_cursor_page(next).await
    }

    /// 把分页对象展开成异步流。
    #[allow(tail_expr_drop_order)]
    pub fn into_stream(self) -> PageStream<T> {
        Box::pin(try_stream! {
            let mut current = Some(self);

            while let Some(page) = current.take() {
                for item in &page.data {
                    yield item.clone();
                }

                if page.has_next_page() {
                    current = Some(page.next_page().await?);
                }
            }
        })
    }
}

/// 分页异步流类型。
pub type PageStream<T> = Pin<Box<dyn Stream<Item = Result<T>> + Send>>;
