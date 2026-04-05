//! SSE、行解码与流式聚合。

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Result, SerializationError, StreamError};
use crate::resources::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionChunkDelta, ChatCompletionMessage,
    ChatCompletionToolCall, Response,
};
use crate::response_meta::ResponseMeta;

/// 用于把字节流切分为逻辑行。
#[derive(Debug, Default, Clone)]
pub struct LineDecoder {
    buffer: Vec<u8>,
}

impl LineDecoder {
    /// 向解码器推入一个新分片，并返回已经完整的行。
    ///
    /// # Errors
    ///
    /// 当 UTF-8 解码失败时返回错误。
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>> {
        self.buffer.extend_from_slice(chunk);
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut index = 0usize;

        while index < self.buffer.len() {
            match self.buffer[index] {
                b'\n' => {
                    let end = if index > start && self.buffer[index - 1] == b'\r' {
                        index - 1
                    } else {
                        index
                    };
                    lines.push(bytes_to_string(&self.buffer[start..end])?);
                    start = index + 1;
                }
                b'\r' => {
                    let end = index;
                    if index + 1 < self.buffer.len() && self.buffer[index + 1] == b'\n' {
                        index += 1;
                        lines.push(bytes_to_string(&self.buffer[start..end])?);
                        start = index + 1;
                    } else {
                        lines.push(bytes_to_string(&self.buffer[start..end])?);
                        start = index + 1;
                    }
                }
                _ => {}
            }
            index += 1;
        }

        if start > 0 {
            self.buffer.drain(0..start);
        }

        Ok(lines)
    }

    /// 在输入结束时刷新最后一行。
    ///
    /// # Errors
    ///
    /// 当 UTF-8 解码失败时返回错误。
    pub fn finish(&mut self) -> Result<Option<String>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        let line = if self.buffer.last() == Some(&b'\r') {
            let length = self.buffer.len() - 1;
            bytes_to_string(&self.buffer[..length])?
        } else {
            bytes_to_string(&self.buffer)?
        };
        self.buffer.clear();
        Ok(Some(line))
    }
}

fn bytes_to_string(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|error| {
        SerializationError::new(format!("SSE 行解码失败，收到非法 UTF-8: {error}")).into()
    })
}

/// 表示一个标准 SSE 事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// 事件名。
    pub event: Option<String>,
    /// 数据体。
    pub data: String,
    /// 事件 ID。
    pub id: Option<String>,
    /// 服务端建议的重连时间。
    pub retry: Option<u64>,
}

#[derive(Debug, Default)]
struct PendingSseEvent {
    event: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u64>,
}

impl PendingSseEvent {
    fn push_line(&mut self, line: &str) -> Result<Option<SseEvent>> {
        if line.is_empty() {
            if self.event.is_none()
                && self.data.is_empty()
                && self.id.is_none()
                && self.retry.is_none()
            {
                return Ok(None);
            }

            let event = SseEvent {
                event: self.event.take(),
                data: self.data.join("\n"),
                id: self.id.take(),
                retry: self.retry.take(),
            };
            self.data.clear();
            return Ok(Some(event));
        }

        if line.starts_with(':') {
            return Ok(None);
        }

        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => (line, ""),
        };

        match field {
            "event" => self.event = Some(value.to_owned()),
            "data" => self.data.push(value.to_owned()),
            "id" => self.id = Some(value.to_owned()),
            "retry" => {
                self.retry = value.parse::<u64>().ok();
            }
            _ => {}
        }

        Ok(None)
    }

    fn flush(&mut self) -> Option<SseEvent> {
        if self.event.is_none() && self.data.is_empty() && self.id.is_none() && self.retry.is_none()
        {
            return None;
        }

        let event = SseEvent {
            event: self.event.take(),
            data: self.data.join("\n"),
            id: self.id.take(),
            retry: self.retry.take(),
        };
        self.data.clear();
        Some(event)
    }
}

/// 表示原始 SSE 流。
pub struct RawSseStream {
    inner: Pin<Box<dyn Stream<Item = Result<SseEvent>> + Send>>,
    meta: ResponseMeta,
}

impl RawSseStream {
    /// 从 `reqwest::Response` 创建原始 SSE 流。
    #[allow(clippy::collapsible_if, tail_expr_drop_order)]
    pub fn new(response: reqwest::Response, meta: ResponseMeta) -> Self {
        let stream = try_stream! {
            let mut decoder = LineDecoder::default();
            let mut pending = PendingSseEvent::default();
            let mut byte_stream = response.bytes_stream();

            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|error| StreamError::new(format!("读取 SSE 数据流失败: {error}")))?;
                for line in decoder.push(&chunk)? {
                    if let Some(event) = pending.push_line(&line)? {
                        yield event;
                    }
                }
            }

            if let Some(line) = decoder.finish()? {
                if let Some(event) = pending.push_line(&line)? {
                    yield event;
                }
            }

            if let Some(event) = pending.flush() {
                yield event;
            }
        };

        Self {
            inner: Box::pin(stream),
            meta,
        }
    }

    /// 返回流对应的响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }

    /// 将原始 SSE 流转换为 JSON 事件流。
    #[allow(tail_expr_drop_order)]
    pub fn into_typed<T>(self) -> SseStream<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let meta = self.meta.clone();
        let stream = try_stream! {
            let mut raw = self;
            while let Some(event) = raw.next().await {
                let event = event?;
                if event.data == "[DONE]" {
                    break;
                }
                let item = serde_json::from_str::<T>(&event.data).map_err(|error| {
                    StreamError::new(format!("解析 SSE JSON 事件失败: {error}; payload={}", event.data))
                })?;
                yield item;
            }
        };

        SseStream {
            inner: Box::pin(stream),
            meta,
        }
    }
}

impl fmt::Debug for RawSseStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawSseStream")
            .field("meta", &self.meta)
            .finish()
    }
}

impl Stream for RawSseStream {
    type Item = Result<SseEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

/// 表示一个类型化后的 SSE 流。
pub struct SseStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T>> + Send>>,
    meta: ResponseMeta,
}

impl<T> SseStream<T> {
    /// 返回流对应的响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }
}

impl<T> Stream for SseStream<T> {
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

impl<T> fmt::Debug for SseStream<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SseStream")
            .field("meta", &self.meta)
            .finish()
    }
}

/// 表示聊天补全的流式包装器。
#[derive(Debug)]
pub struct ChatCompletionStream {
    inner: SseStream<ChatCompletionChunk>,
    accumulator: ChatCompletionAccumulator,
}

impl ChatCompletionStream {
    /// 创建新的聊天补全流。
    pub fn new(inner: SseStream<ChatCompletionChunk>) -> Self {
        Self {
            inner,
            accumulator: ChatCompletionAccumulator::default(),
        }
    }

    /// 获取截至目前的聚合快照。
    pub fn snapshot(&self) -> Option<ChatCompletion> {
        self.accumulator.snapshot()
    }

    /// 消费整个流并返回最终快照。
    pub async fn into_final_response(mut self) -> Result<Option<ChatCompletion>> {
        while let Some(chunk) = self.next().await {
            chunk?;
        }
        Ok(self.snapshot())
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
    }

    /// 消费整个流并返回最终聊天补全对象。
    pub async fn final_chat_completion(self) -> Result<Option<ChatCompletion>> {
        self.into_final_response().await
    }

    /// 消费整个流并返回通过 finish_reason 校验的最终聊天补全对象。
    pub async fn final_chat_completion_checked(self) -> Result<Option<ChatCompletion>> {
        let response = self.into_final_response().await?;
        if let Some(response) = &response {
            response.ensure_not_truncated()?;
        }
        Ok(response)
    }

    /// 消费整个流并返回首个 choice 的最终消息。
    pub async fn final_message(self) -> Result<Option<ChatCompletionMessage>> {
        Ok(self.into_final_response().await?.and_then(|response| {
            response
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message)
        }))
    }

    /// 消费整个流并返回首个 choice 的最终文本内容。
    pub async fn final_content(self) -> Result<Option<String>> {
        Ok(self
            .final_message()
            .await?
            .and_then(|message| message.content))
    }

    /// 消费整个流并返回首个 choice 的最终工具调用集合。
    pub async fn final_tool_calls(self) -> Result<Option<Vec<ChatCompletionToolCall>>> {
        Ok(self
            .final_message()
            .await?
            .map(|message| message.tool_calls)
            .filter(|tool_calls| !tool_calls.is_empty()))
    }

    /// 把原始 chunk 流转换为带有高层语义事件的运行时流。
    pub fn events(self) -> ChatCompletionEventStream {
        ChatCompletionEventStream::new(self)
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                this.accumulator.apply(&chunk);
                Poll::Ready(Some(Ok(chunk)))
            }
            other => other,
        }
    }
}

/// 表示聊天文本增量和当前聚合快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatContentSnapshotEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 本次文本增量。
    pub delta: String,
    /// 当前累计文本。
    pub snapshot: String,
    /// 如果当前文本是合法 JSON，则提供解析结果。
    pub parsed: Option<Value>,
}

/// 表示聊天文本完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatContentDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 完整文本。
    pub content: String,
    /// 如果完整文本是合法 JSON，则提供解析结果。
    pub parsed: Option<Value>,
}

/// 表示拒绝回答文本增量和当前聚合快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatRefusalSnapshotEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 本次拒绝文本增量。
    pub delta: String,
    /// 当前累计拒绝文本。
    pub snapshot: String,
}

/// 表示拒绝回答完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatRefusalDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 完整拒绝文本。
    pub refusal: String,
}

/// 表示工具参数增量和当前聚合快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatToolArgumentsSnapshotEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 工具调用索引。
    pub tool_call_index: u32,
    /// 工具名称。
    pub name: String,
    /// 本次参数增量。
    pub arguments_delta: String,
    /// 当前累计参数字符串。
    pub arguments: String,
    /// 如果当前参数是合法 JSON，则提供解析结果。
    pub parsed_arguments: Option<Value>,
}

/// 表示工具参数完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatToolArgumentsDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 工具调用索引。
    pub tool_call_index: u32,
    /// 工具名称。
    pub name: String,
    /// 完整参数字符串。
    pub arguments: String,
    /// 如果完整参数是合法 JSON，则提供解析结果。
    pub parsed_arguments: Option<Value>,
}

/// 表示 token logprobs 增量和当前聚合快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLogProbsSnapshotEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 本次 logprobs 增量。
    pub values: Vec<Value>,
    /// 当前累计 logprobs。
    pub snapshot: Vec<Value>,
}

/// 表示 token logprobs 完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLogProbsDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 完整 logprobs。
    pub values: Vec<Value>,
}

/// 表示聊天流在运行时派生出的高层事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatCompletionRuntimeEvent {
    /// 原始 chunk 与当前补全快照。
    Chunk {
        /// 原始 chunk。
        chunk: ChatCompletionChunk,
        /// 当前累计快照。
        snapshot: ChatCompletion,
    },
    /// 文本内容增量。
    ContentDelta(ChatContentSnapshotEvent),
    /// 文本内容完成。
    ContentDone(ChatContentDoneEvent),
    /// 拒绝回答增量。
    RefusalDelta(ChatRefusalSnapshotEvent),
    /// 拒绝回答完成。
    RefusalDone(ChatRefusalDoneEvent),
    /// 工具参数增量。
    ToolCallArgumentsDelta(ChatToolArgumentsSnapshotEvent),
    /// 工具参数完成。
    ToolCallArgumentsDone(ChatToolArgumentsDoneEvent),
    /// 内容 token logprobs 增量。
    LogProbsContentDelta(ChatLogProbsSnapshotEvent),
    /// 内容 token logprobs 完成。
    LogProbsContentDone(ChatLogProbsDoneEvent),
    /// 拒绝 token logprobs 增量。
    LogProbsRefusalDelta(ChatLogProbsSnapshotEvent),
    /// 拒绝 token logprobs 完成。
    LogProbsRefusalDone(ChatLogProbsDoneEvent),
}

#[derive(Debug, Default, Clone)]
struct ChatChoiceEventState {
    content_done: bool,
    refusal_done: bool,
    logprobs_content_done: bool,
    logprobs_refusal_done: bool,
    current_tool_call_index: Option<u32>,
    done_tool_calls: HashSet<u32>,
}

/// 表示带高层语义事件的聊天流。
#[derive(Debug)]
pub struct ChatCompletionEventStream {
    inner: ChatCompletionStream,
    queue: VecDeque<ChatCompletionRuntimeEvent>,
    choice_states: HashMap<u32, ChatChoiceEventState>,
}

impl ChatCompletionEventStream {
    fn new(inner: ChatCompletionStream) -> Self {
        Self {
            inner,
            queue: VecDeque::new(),
            choice_states: HashMap::new(),
        }
    }

    /// 返回当前累计快照。
    pub fn snapshot(&self) -> Option<ChatCompletion> {
        self.inner.snapshot()
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
    }

    /// 消费整个事件流并返回最终聊天补全对象。
    pub async fn final_chat_completion(mut self) -> Result<Option<ChatCompletion>> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.snapshot())
    }

    /// 消费整个事件流并返回通过 finish_reason 校验的最终聊天补全对象。
    pub async fn final_chat_completion_checked(self) -> Result<Option<ChatCompletion>> {
        let response = self.final_chat_completion().await?;
        if let Some(response) = &response {
            response.ensure_not_truncated()?;
        }
        Ok(response)
    }

    /// 消费整个事件流并返回首个 choice 的最终消息。
    pub async fn final_message(self) -> Result<Option<ChatCompletionMessage>> {
        Ok(self.final_chat_completion().await?.and_then(|response| {
            response
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message)
        }))
    }

    /// 消费整个事件流并返回首个 choice 的最终文本内容。
    pub async fn final_content(self) -> Result<Option<String>> {
        Ok(self
            .final_message()
            .await?
            .and_then(|message| message.content))
    }

    /// 消费整个事件流并返回首个 choice 的最终工具调用集合。
    pub async fn final_tool_calls(self) -> Result<Option<Vec<ChatCompletionToolCall>>> {
        Ok(self
            .final_message()
            .await?
            .map(|message| message.tool_calls)
            .filter(|tool_calls| !tool_calls.is_empty()))
    }

    fn enqueue_events(&mut self, chunk: &ChatCompletionChunk, snapshot: &ChatCompletion) {
        self.queue.push_back(ChatCompletionRuntimeEvent::Chunk {
            chunk: chunk.clone(),
            snapshot: snapshot.clone(),
        });

        for choice in &chunk.choices {
            let Some(snapshot_choice) = snapshot
                .choices
                .iter()
                .find(|item| item.index == choice.index)
            else {
                continue;
            };
            let state = self
                .choice_states
                .get(&choice.index)
                .cloned()
                .unwrap_or_default();
            let (events, state) = derive_chat_choice_events(choice, snapshot_choice, state);
            self.choice_states.insert(choice.index, state);
            self.queue.extend(events);
        }
    }
}

impl Stream for ChatCompletionEventStream {
    type Item = Result<ChatCompletionRuntimeEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(event) = this.queue.pop_front() {
            return Poll::Ready(Some(Ok(event)));
        }

        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if let Some(snapshot) = this.inner.snapshot() {
                    this.enqueue_events(&chunk, &snapshot);
                }
                Poll::Ready(this.queue.pop_front().map(Ok))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// 表示 Responses API 的流式包装器。
#[derive(Debug)]
pub struct ResponseStream {
    inner: SseStream<Value>,
    accumulator: ResponseAccumulator,
}

impl ResponseStream {
    /// 创建新的 Responses 流。
    pub fn new(inner: SseStream<Value>) -> Self {
        Self {
            inner,
            accumulator: ResponseAccumulator::default(),
        }
    }

    /// 获取当前聚合出的输出文本。
    pub fn output_text(&self) -> &str {
        &self.accumulator.output_text
    }

    /// 获取已聚合的函数调用参数。
    pub fn function_arguments(&self) -> &HashMap<String, String> {
        &self.accumulator.function_arguments
    }

    /// 获取截至目前聚合出的响应快照。
    pub fn snapshot(&self) -> Option<Response> {
        self.accumulator.response.clone()
    }

    /// 消费整个流并返回最终文本快照。
    pub async fn into_output_text(mut self) -> Result<String> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.accumulator.output_text)
    }

    /// 消费整个流并返回最终响应快照。
    pub async fn final_response(mut self) -> Result<Option<Response>> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.accumulator.response)
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
    }

    /// 把原始事件流转换为带高层语义的运行时流。
    pub fn events(self) -> ResponseEventStream {
        ResponseEventStream::new(self)
    }
}

impl Stream for ResponseStream {
    type Item = Result<Value>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                this.accumulator.apply(&event);
                Poll::Ready(Some(Ok(event)))
            }
            other => other,
        }
    }
}

/// 表示输出文本增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseOutputTextEvent {
    /// 原始事件类型。
    pub event_type: String,
    /// 输出项索引。
    pub output_index: usize,
    /// 内容索引。
    pub content_index: usize,
    /// 文本增量或最终文本。
    pub text: String,
    /// 当前累计文本。
    pub snapshot: String,
}

/// 表示函数调用参数增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseFunctionCallArgumentsEvent {
    /// 输出项索引。
    pub output_index: usize,
    /// 关联的 item ID。
    pub item_id: Option<String>,
    /// 参数增量。
    pub delta: String,
    /// 当前累计参数字符串。
    pub snapshot: String,
    /// 如果当前参数是合法 JSON，则提供解析结果。
    pub parsed_arguments: Option<Value>,
}

/// 表示 Responses 流在运行时派生出的高层事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseRuntimeEvent {
    /// 未专门派生的原始事件。
    Raw(Value),
    /// 响应已创建。
    ResponseCreated(Response),
    /// 输出项已追加。
    OutputItemAdded {
        /// 输出项索引。
        output_index: usize,
        /// 新增输出项。
        item: Value,
        /// 当前响应快照。
        snapshot: Response,
    },
    /// 输出内容片段已追加。
    ContentPartAdded {
        /// 输出项索引。
        output_index: usize,
        /// 内容索引。
        content_index: usize,
        /// 新增内容片段。
        part: Value,
        /// 当前响应快照。
        snapshot: Response,
    },
    /// 输出文本增量。
    OutputTextDelta(ResponseOutputTextEvent),
    /// 输出文本完成。
    OutputTextDone(ResponseOutputTextEvent),
    /// 函数调用参数增量。
    FunctionCallArgumentsDelta(ResponseFunctionCallArgumentsEvent),
    /// 响应完成。
    Completed(Response),
}

/// 表示带高层语义事件的 Responses 流。
#[derive(Debug)]
pub struct ResponseEventStream {
    inner: ResponseStream,
}

impl ResponseEventStream {
    fn new(inner: ResponseStream) -> Self {
        Self { inner }
    }

    /// 返回当前累计文本。
    pub fn output_text(&self) -> &str {
        self.inner.output_text()
    }

    /// 返回当前聚合的函数参数。
    pub fn function_arguments(&self) -> &HashMap<String, String> {
        self.inner.function_arguments()
    }

    /// 返回当前响应快照。
    pub fn snapshot(&self) -> Option<Response> {
        self.inner.snapshot()
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
    }

    /// 消费整个事件流并返回最终响应快照。
    pub async fn final_response(mut self) -> Result<Option<Response>> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.snapshot())
    }
}

impl Stream for ResponseEventStream {
    type Item = Result<ResponseRuntimeEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let snapshot = this.inner.snapshot();
                Poll::Ready(Some(Ok(derive_response_runtime_event(event, snapshot))))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// 表示 Assistants/Beta Threads SSE 事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantStreamEvent {
    /// SSE 事件名。
    pub event: String,
    /// 事件对应的 JSON 负载。
    pub data: Value,
}

impl AssistantStreamEvent {
    /// 判断当前事件是否为错误事件。
    pub fn is_error(&self) -> bool {
        self.event == "error"
    }

    /// 把事件负载解析为指定类型。
    pub fn data_as<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.data.clone()).map_err(|error| {
            SerializationError::new(format!(
                "Assistants 流事件反序列化失败: event={}, error={error}",
                self.event
            ))
            .into()
        })
    }
}

/// 表示 Assistants 流运行时累积出的快照。
#[derive(Debug, Clone, Default)]
pub struct AssistantStreamSnapshot {
    thread: Option<Value>,
    runs: BTreeMap<String, Value>,
    messages: BTreeMap<String, Value>,
    run_steps: BTreeMap<String, Value>,
    latest_run_id: Option<String>,
    latest_message_id: Option<String>,
    latest_run_step_id: Option<String>,
}

impl AssistantStreamSnapshot {
    /// 返回最新的 thread 原始快照。
    pub fn thread_raw(&self) -> Option<&Value> {
        self.thread.as_ref()
    }

    /// 返回最新的 thread 快照。
    pub fn thread<T>(&self) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.thread
            .as_ref()
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// 返回指定 run 的原始快照。
    pub fn run_raw(&self, run_id: &str) -> Option<&Value> {
        self.runs.get(run_id)
    }

    /// 返回最新 run 的原始快照。
    pub fn latest_run_raw(&self) -> Option<&Value> {
        self.latest_run_id
            .as_deref()
            .and_then(|run_id| self.runs.get(run_id))
    }

    /// 返回指定 message 的原始快照。
    pub fn message_raw(&self, message_id: &str) -> Option<&Value> {
        self.messages.get(message_id)
    }

    /// 返回最新 message 的原始快照。
    pub fn latest_message_raw(&self) -> Option<&Value> {
        self.latest_message_id
            .as_deref()
            .and_then(|message_id| self.messages.get(message_id))
    }

    /// 返回指定 run 的结构化快照。
    pub fn run<T>(&self, run_id: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.run_raw(run_id)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// 返回最新 run 的结构化快照。
    pub fn latest_run<T>(&self) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.latest_run_raw()
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// 返回指定 message 的结构化快照。
    pub fn message<T>(&self, message_id: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.messages
            .get(message_id)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// 返回最新 message 的结构化快照。
    pub fn latest_message<T>(&self) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.latest_message_id
            .as_deref()
            .and_then(|message_id| self.message(message_id))
    }

    /// 返回指定 run step 的原始快照。
    pub fn run_step_raw(&self, step_id: &str) -> Option<&Value> {
        self.run_steps.get(step_id)
    }

    /// 返回最新 run step 的原始快照。
    pub fn latest_run_step_raw(&self) -> Option<&Value> {
        self.latest_run_step_id
            .as_deref()
            .and_then(|step_id| self.run_steps.get(step_id))
    }

    /// 返回指定 run step 的结构化快照。
    pub fn run_step<T>(&self, step_id: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.run_steps
            .get(step_id)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }

    /// 返回最新 run step 的结构化快照。
    pub fn latest_run_step<T>(&self) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.latest_run_step_id
            .as_deref()
            .and_then(|step_id| self.run_step(step_id))
    }
}

/// 表示 Assistants API 的流式包装器。
pub struct AssistantStream {
    inner: Pin<Box<dyn Stream<Item = Result<AssistantStreamEvent>> + Send>>,
    meta: ResponseMeta,
    snapshot: AssistantStreamSnapshot,
}

impl AssistantStream {
    /// 从原始 SSE 流创建 Assistants 流。
    #[allow(tail_expr_drop_order)]
    pub fn new(raw: RawSseStream) -> Self {
        let meta = raw.meta().clone();
        let stream = try_stream! {
            let mut raw = raw;
            while let Some(event) = raw.next().await {
                let event = event?;
                if event.data == "[DONE]" {
                    break;
                }

                let data = serde_json::from_str::<Value>(&event.data).map_err(|error| {
                    StreamError::new(format!(
                        "解析 Assistants SSE 事件失败: event={:?}, error={error}, payload={}",
                        event.event,
                        event.data
                    ))
                })?;
                let event_name = event
                    .event
                    .or_else(|| data.get("event").and_then(Value::as_str).map(str::to_owned))
                    .or_else(|| data.get("type").and_then(Value::as_str).map(str::to_owned))
                    .unwrap_or_else(|| "message".into());

                yield AssistantStreamEvent {
                    event: event_name,
                    data,
                };
            }
        };

        Self {
            inner: Box::pin(stream),
            meta,
            snapshot: AssistantStreamSnapshot::default(),
        }
    }

    /// 返回截至目前的快照。
    pub fn snapshot(&self) -> &AssistantStreamSnapshot {
        &self.snapshot
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        &self.meta
    }

    /// 消费整个流并返回最终快照。
    pub async fn final_snapshot(mut self) -> Result<AssistantStreamSnapshot> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.snapshot)
    }

    /// 把原始 Assistants 事件流转换为带高层派生语义的运行时流。
    pub fn events(self) -> AssistantEventStream {
        AssistantEventStream::new(self)
    }
}

impl fmt::Debug for AssistantStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssistantStream")
            .field("meta", &self.meta)
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

impl Stream for AssistantStream {
    type Item = Result<AssistantStreamEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                this.snapshot.apply(&event);
                Poll::Ready(Some(Ok(event)))
            }
            other => other,
        }
    }
}

/// 表示 message 创建事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessageCreatedEvent {
    /// message 快照。
    pub message: Value,
}

/// 表示 message 增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessageDeltaEvent {
    /// message 增量。
    pub delta: Value,
    /// 当前 message 快照。
    pub snapshot: Value,
}

/// 表示 message 完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessageDoneEvent {
    /// message 快照。
    pub message: Value,
}

/// 表示 run step 创建事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantRunStepCreatedEvent {
    /// run step 快照。
    pub run_step: Value,
}

/// 表示 run step 增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantRunStepDeltaEvent {
    /// run step 增量。
    pub delta: Value,
    /// 当前 run step 快照。
    pub snapshot: Value,
}

/// 表示 run step 完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantRunStepDoneEvent {
    /// run step 快照。
    pub run_step: Value,
}

/// 表示工具调用创建事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantToolCallCreatedEvent {
    /// run step ID。
    pub run_step_id: Option<String>,
    /// 工具调用索引。
    pub tool_call_index: usize,
    /// 工具调用快照。
    pub tool_call: Value,
}

/// 表示工具调用增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantToolCallDeltaEvent {
    /// run step ID。
    pub run_step_id: Option<String>,
    /// 工具调用索引。
    pub tool_call_index: usize,
    /// 工具调用增量。
    pub delta: Value,
    /// 当前工具调用快照。
    pub snapshot: Value,
}

/// 表示工具调用完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantToolCallDoneEvent {
    /// run step ID。
    pub run_step_id: Option<String>,
    /// 工具调用索引。
    pub tool_call_index: usize,
    /// 工具调用快照。
    pub tool_call: Value,
}

/// 表示文本内容创建事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantTextCreatedEvent {
    /// message ID。
    pub message_id: Option<String>,
    /// 内容索引。
    pub content_index: usize,
    /// 文本内容快照。
    pub text: Value,
}

/// 表示文本内容增量事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantTextDeltaEvent {
    /// message ID。
    pub message_id: Option<String>,
    /// 内容索引。
    pub content_index: usize,
    /// 文本增量。
    pub delta: Value,
    /// 当前文本内容快照。
    pub snapshot: Value,
}

/// 表示文本内容完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantTextDoneEvent {
    /// message ID。
    pub message_id: Option<String>,
    /// 内容索引。
    pub content_index: usize,
    /// 当前文本内容快照。
    pub text: Value,
    /// 当前 message 快照。
    pub message: Value,
}

/// 表示图片文件完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantImageFileDoneEvent {
    /// message ID。
    pub message_id: Option<String>,
    /// 内容索引。
    pub content_index: usize,
    /// 图片文件内容。
    pub image_file: Value,
    /// 当前 message 快照。
    pub message: Value,
}

/// 表示 Assistants 流在运行时派生出的高层事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssistantRuntimeEvent {
    /// 原始 Assistants SSE 事件。
    Event(AssistantStreamEvent),
    /// message 创建。
    MessageCreated(AssistantMessageCreatedEvent),
    /// message 增量。
    MessageDelta(AssistantMessageDeltaEvent),
    /// message 完成。
    MessageDone(AssistantMessageDoneEvent),
    /// run step 创建。
    RunStepCreated(AssistantRunStepCreatedEvent),
    /// run step 增量。
    RunStepDelta(AssistantRunStepDeltaEvent),
    /// run step 完成。
    RunStepDone(AssistantRunStepDoneEvent),
    /// 工具调用创建。
    ToolCallCreated(AssistantToolCallCreatedEvent),
    /// 工具调用增量。
    ToolCallDelta(AssistantToolCallDeltaEvent),
    /// 工具调用完成。
    ToolCallDone(AssistantToolCallDoneEvent),
    /// 文本内容创建。
    TextCreated(AssistantTextCreatedEvent),
    /// 文本内容增量。
    TextDelta(AssistantTextDeltaEvent),
    /// 文本内容完成。
    TextDone(AssistantTextDoneEvent),
    /// 图片文件完成。
    ImageFileDone(AssistantImageFileDoneEvent),
}

/// 表示带高层派生事件的 Assistants 流。
#[derive(Debug)]
pub struct AssistantEventStream {
    inner: AssistantStream,
    queue: VecDeque<AssistantRuntimeEvent>,
    seen_message_texts: HashMap<String, HashSet<usize>>,
    seen_step_tool_calls: HashMap<String, HashSet<usize>>,
}

impl AssistantEventStream {
    fn new(inner: AssistantStream) -> Self {
        Self {
            inner,
            queue: VecDeque::new(),
            seen_message_texts: HashMap::new(),
            seen_step_tool_calls: HashMap::new(),
        }
    }

    /// 返回当前累计快照。
    pub fn snapshot(&self) -> &AssistantStreamSnapshot {
        self.inner.snapshot()
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
    }

    /// 消费整个事件流并返回最终快照。
    pub async fn final_snapshot(mut self) -> Result<AssistantStreamSnapshot> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.inner.snapshot)
    }

    fn enqueue_events(&mut self, event: &AssistantStreamEvent) {
        self.queue
            .push_back(AssistantRuntimeEvent::Event(event.clone()));

        match event.event.as_str() {
            "thread.message.created" => {
                self.queue.push_back(AssistantRuntimeEvent::MessageCreated(
                    AssistantMessageCreatedEvent {
                        message: event.data.clone(),
                    },
                ));
                self.enqueue_text_created_from_message(&event.data);
            }
            "thread.message.delta" => {
                let message_id = event
                    .data
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let snapshot = message_id
                    .as_deref()
                    .and_then(|id| self.inner.snapshot().message_raw(id))
                    .cloned()
                    .unwrap_or_else(|| event.data.clone());
                self.queue.push_back(AssistantRuntimeEvent::MessageDelta(
                    AssistantMessageDeltaEvent {
                        delta: event.data.get("delta").cloned().unwrap_or(Value::Null),
                        snapshot: snapshot.clone(),
                    },
                ));
                self.enqueue_text_delta(&message_id, event, &snapshot);
            }
            "thread.message.completed" | "thread.message.incomplete" => {
                let message = event
                    .data
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| self.inner.snapshot().message_raw(id))
                    .cloned()
                    .unwrap_or_else(|| event.data.clone());
                self.queue.push_back(AssistantRuntimeEvent::MessageDone(
                    AssistantMessageDoneEvent {
                        message: message.clone(),
                    },
                ));
                self.enqueue_message_done_content(&message);
            }
            "thread.run.step.created" => {
                self.queue.push_back(AssistantRuntimeEvent::RunStepCreated(
                    AssistantRunStepCreatedEvent {
                        run_step: event.data.clone(),
                    },
                ));
            }
            "thread.run.step.delta" => {
                let step_id = event
                    .data
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let snapshot = step_id
                    .as_deref()
                    .and_then(|id| self.inner.snapshot().run_step_raw(id))
                    .cloned()
                    .unwrap_or_else(|| event.data.clone());
                self.queue.push_back(AssistantRuntimeEvent::RunStepDelta(
                    AssistantRunStepDeltaEvent {
                        delta: event.data.get("delta").cloned().unwrap_or(Value::Null),
                        snapshot: snapshot.clone(),
                    },
                ));
                self.enqueue_tool_call_delta(&step_id, event, &snapshot);
            }
            "thread.run.step.completed"
            | "thread.run.step.failed"
            | "thread.run.step.cancelled"
            | "thread.run.step.expired" => {
                let run_step = event
                    .data
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| self.inner.snapshot().run_step_raw(id))
                    .cloned()
                    .unwrap_or_else(|| event.data.clone());
                self.queue.push_back(AssistantRuntimeEvent::RunStepDone(
                    AssistantRunStepDoneEvent {
                        run_step: run_step.clone(),
                    },
                ));
                self.enqueue_tool_call_done(&run_step);
            }
            _ => {}
        }
    }

    fn enqueue_text_created_from_message(&mut self, message: &Value) {
        let message_id = message.get("id").and_then(Value::as_str).map(str::to_owned);
        let Some(content) = message.get("content").and_then(Value::as_array) else {
            return;
        };
        for (index, part) in content.iter().enumerate() {
            if part.get("type").and_then(Value::as_str) == Some("text") {
                self.mark_message_text_seen(&message_id, index);
                self.queue.push_back(AssistantRuntimeEvent::TextCreated(
                    AssistantTextCreatedEvent {
                        message_id: message_id.clone(),
                        content_index: index,
                        text: part.clone(),
                    },
                ));
            }
        }
    }

    fn enqueue_text_delta(
        &mut self,
        message_id: &Option<String>,
        event: &AssistantStreamEvent,
        snapshot: &Value,
    ) {
        let Some(content_deltas) = event
            .data
            .get("delta")
            .and_then(|value| value.get("content"))
            .and_then(Value::as_array)
        else {
            return;
        };

        let snapshot_content = snapshot
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for content_delta in content_deltas {
            let index = content_delta
                .get("index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_default();
            if content_delta.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }

            if !self.message_text_seen(message_id, index)
                && let Some(snapshot_part) = snapshot_content.get(index)
            {
                self.mark_message_text_seen(message_id, index);
                self.queue.push_back(AssistantRuntimeEvent::TextCreated(
                    AssistantTextCreatedEvent {
                        message_id: message_id.clone(),
                        content_index: index,
                        text: snapshot_part.clone(),
                    },
                ));
            }

            if let Some(snapshot_part) = snapshot_content.get(index) {
                self.queue
                    .push_back(AssistantRuntimeEvent::TextDelta(AssistantTextDeltaEvent {
                        message_id: message_id.clone(),
                        content_index: index,
                        delta: content_delta.clone(),
                        snapshot: snapshot_part.clone(),
                    }));
            }
        }
    }

    fn enqueue_message_done_content(&mut self, message: &Value) {
        let message_id = message.get("id").and_then(Value::as_str).map(str::to_owned);
        let Some(content) = message.get("content").and_then(Value::as_array) else {
            return;
        };
        for (index, part) in content.iter().enumerate() {
            match part.get("type").and_then(Value::as_str) {
                Some("text") => {
                    self.mark_message_text_seen(&message_id, index);
                    self.queue
                        .push_back(AssistantRuntimeEvent::TextDone(AssistantTextDoneEvent {
                            message_id: message_id.clone(),
                            content_index: index,
                            text: part.clone(),
                            message: message.clone(),
                        }));
                }
                Some("image_file") => {
                    self.queue.push_back(AssistantRuntimeEvent::ImageFileDone(
                        AssistantImageFileDoneEvent {
                            message_id: message_id.clone(),
                            content_index: index,
                            image_file: part.clone(),
                            message: message.clone(),
                        },
                    ));
                }
                _ => {}
            }
        }
    }

    fn enqueue_tool_call_delta(
        &mut self,
        step_id: &Option<String>,
        event: &AssistantStreamEvent,
        snapshot: &Value,
    ) {
        let Some(tool_call_deltas) = event
            .data
            .get("delta")
            .and_then(|value| value.get("step_details"))
            .and_then(|value| value.get("tool_calls"))
            .and_then(Value::as_array)
        else {
            return;
        };
        let snapshot_calls = snapshot
            .get("step_details")
            .and_then(|value| value.get("tool_calls"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for tool_call_delta in tool_call_deltas {
            let index = tool_call_delta
                .get("index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_default();
            if !self.step_tool_call_seen(step_id, index)
                && let Some(snapshot_call) = snapshot_calls.get(index)
            {
                self.mark_step_tool_call_seen(step_id, index);
                self.queue.push_back(AssistantRuntimeEvent::ToolCallCreated(
                    AssistantToolCallCreatedEvent {
                        run_step_id: step_id.clone(),
                        tool_call_index: index,
                        tool_call: snapshot_call.clone(),
                    },
                ));
            }
            if let Some(snapshot_call) = snapshot_calls.get(index) {
                self.queue.push_back(AssistantRuntimeEvent::ToolCallDelta(
                    AssistantToolCallDeltaEvent {
                        run_step_id: step_id.clone(),
                        tool_call_index: index,
                        delta: tool_call_delta.clone(),
                        snapshot: snapshot_call.clone(),
                    },
                ));
            }
        }
    }

    fn enqueue_tool_call_done(&mut self, run_step: &Value) {
        let step_id = run_step
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let Some(tool_calls) = run_step
            .get("step_details")
            .and_then(|value| value.get("tool_calls"))
            .and_then(Value::as_array)
        else {
            return;
        };
        for (index, tool_call) in tool_calls.iter().enumerate() {
            self.mark_step_tool_call_seen(&step_id, index);
            self.queue.push_back(AssistantRuntimeEvent::ToolCallDone(
                AssistantToolCallDoneEvent {
                    run_step_id: step_id.clone(),
                    tool_call_index: index,
                    tool_call: tool_call.clone(),
                },
            ));
        }
    }

    fn message_text_seen(&self, message_id: &Option<String>, index: usize) -> bool {
        message_id
            .as_deref()
            .and_then(|id| self.seen_message_texts.get(id))
            .is_some_and(|set| set.contains(&index))
    }

    fn mark_message_text_seen(&mut self, message_id: &Option<String>, index: usize) {
        let Some(message_id) = message_id else {
            return;
        };
        self.seen_message_texts
            .entry(message_id.clone())
            .or_default()
            .insert(index);
    }

    fn step_tool_call_seen(&self, step_id: &Option<String>, index: usize) -> bool {
        step_id
            .as_deref()
            .and_then(|id| self.seen_step_tool_calls.get(id))
            .is_some_and(|set| set.contains(&index))
    }

    fn mark_step_tool_call_seen(&mut self, step_id: &Option<String>, index: usize) {
        let Some(step_id) = step_id else {
            return;
        };
        self.seen_step_tool_calls
            .entry(step_id.clone())
            .or_default()
            .insert(index);
    }
}

impl Stream for AssistantEventStream {
    type Item = Result<AssistantRuntimeEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(event) = this.queue.pop_front() {
            return Poll::Ready(Some(Ok(event)));
        }

        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                this.enqueue_events(&event);
                Poll::Ready(this.queue.pop_front().map(Ok))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ChatCompletionAccumulator {
    id: Option<String>,
    model: Option<String>,
    created: Option<i64>,
    object: String,
    choices: HashMap<u32, AccumulatedChoice>,
}

#[derive(Debug, Default, Clone)]
struct AccumulatedChoice {
    role: Option<String>,
    content: String,
    refusal: String,
    reasoning_content: String,
    finish_reason: Option<String>,
    tool_calls: HashMap<u32, ChatCompletionToolCall>,
    logprobs: Option<Value>,
}

impl ChatCompletionAccumulator {
    fn apply(&mut self, chunk: &ChatCompletionChunk) {
        self.id = Some(chunk.id.clone());
        self.model = Some(chunk.model.clone());
        self.created = chunk.created.or(self.created);
        self.object = if chunk.object.is_empty() {
            "chat.completion".into()
        } else {
            chunk.object.clone()
        };

        for choice in &chunk.choices {
            let state = self.choices.entry(choice.index).or_default();
            state.finish_reason = choice.finish_reason.clone().or(state.finish_reason.clone());
            if let Some(logprobs) = &choice.logprobs {
                merge_logprobs(&mut state.logprobs, logprobs);
            }
            apply_delta(state, &choice.delta);
        }
    }

    fn snapshot(&self) -> Option<ChatCompletion> {
        let id = self.id.clone()?;
        let model = self.model.clone().unwrap_or_default();
        let mut choices: Vec<_> = self
            .choices
            .iter()
            .map(|(index, choice)| {
                let mut extra = BTreeMap::new();
                if !choice.reasoning_content.is_empty() {
                    extra.insert(
                        "reasoning_content".into(),
                        Value::String(choice.reasoning_content.clone()),
                    );
                }

                crate::resources::ChatCompletionChoice {
                    index: *index,
                    finish_reason: choice.finish_reason.clone(),
                    message: ChatCompletionMessage {
                        role: choice.role.clone().unwrap_or_else(|| "assistant".into()),
                        content: (!choice.content.is_empty()).then(|| choice.content.clone()),
                        name: None,
                        tool_call_id: None,
                        tool_calls: {
                            let mut tool_calls = choice
                                .tool_calls
                                .iter()
                                .map(|(tool_call_index, tool_call)| {
                                    (*tool_call_index, tool_call.clone())
                                })
                                .collect::<Vec<_>>();
                            tool_calls.sort_by_key(|(tool_call_index, _)| *tool_call_index);
                            tool_calls
                                .into_iter()
                                .map(|(_, tool_call)| tool_call)
                                .collect()
                        },
                        refusal: (!choice.refusal.is_empty()).then(|| choice.refusal.clone()),
                        reasoning_content: (!choice.reasoning_content.is_empty())
                            .then(|| choice.reasoning_content.clone()),
                        reasoning_details: Vec::new(),
                        extra,
                    },
                    logprobs: choice.logprobs.clone(),
                    extra: BTreeMap::new(),
                }
            })
            .collect();
        choices.sort_by_key(|choice| choice.index);

        Some(ChatCompletion {
            id,
            object: self.object.clone(),
            created: self.created,
            model,
            choices,
            usage: None,
            extra: BTreeMap::new(),
        })
    }
}

fn apply_delta(state: &mut AccumulatedChoice, delta: &ChatCompletionChunkDelta) {
    if let Some(role) = &delta.role {
        state.role = Some(role.clone());
    }
    if let Some(content) = &delta.content {
        state.content.push_str(content);
    }
    if let Some(refusal) = &delta.refusal {
        state.refusal.push_str(refusal);
    }
    if let Some(reasoning_content) = &delta.reasoning_content {
        state.reasoning_content.push_str(reasoning_content);
    }

    for tool_call in &delta.tool_calls {
        let index = tool_call.index.unwrap_or_default();
        let entry = state
            .tool_calls
            .entry(index)
            .or_insert_with(|| ChatCompletionToolCall {
                id: tool_call.id.clone().unwrap_or_default(),
                call_type: tool_call
                    .call_type
                    .clone()
                    .unwrap_or_else(|| "function".into()),
                function: crate::resources::ChatCompletionFunctionCall {
                    name: tool_call
                        .function
                        .as_ref()
                        .and_then(|function| function.name.clone())
                        .unwrap_or_default(),
                    arguments: String::new(),
                },
                extra: BTreeMap::new(),
            });

        if let Some(id) = &tool_call.id {
            entry.id = id.clone();
        }
        if let Some(call_type) = &tool_call.call_type {
            entry.call_type = call_type.clone();
        }
        if let Some(function) = &tool_call.function {
            if let Some(name) = &function.name {
                entry.function.name = name.clone();
            }
            if let Some(arguments) = &function.arguments {
                entry.function.arguments.push_str(arguments);
            }
        }
    }
}

fn merge_logprobs(target: &mut Option<Value>, incoming: &Value) {
    let Some(incoming_object) = incoming.as_object() else {
        *target = Some(incoming.clone());
        return;
    };

    let target_value = target.get_or_insert_with(|| Value::Object(Map::new()));
    let target_object = ensure_object(target_value);

    for (key, value) in incoming_object {
        match value {
            Value::Array(values) => {
                let slot = target_object
                    .entry(key.clone())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Some(existing) = slot.as_array_mut() {
                    existing.extend(values.iter().cloned());
                } else {
                    *slot = Value::Array(values.clone());
                }
            }
            _ => {
                target_object.insert(key.clone(), value.clone());
            }
        }
    }
}

fn parse_optional_json(payload: &str) -> Option<Value> {
    serde_json::from_str(payload).ok()
}

fn logprobs_values(logprobs: Option<&Value>, field_name: &str) -> Option<Vec<Value>> {
    logprobs?.get(field_name).and_then(Value::as_array).cloned()
}

fn derive_chat_choice_events(
    choice: &crate::resources::ChatCompletionChunkChoice,
    snapshot_choice: &crate::resources::ChatCompletionChoice,
    mut state: ChatChoiceEventState,
) -> (Vec<ChatCompletionRuntimeEvent>, ChatChoiceEventState) {
    let mut events = Vec::new();

    if let Some(delta) = &choice.delta.content
        && let Some(snapshot_content) = snapshot_choice.message.content.clone()
    {
        events.push(ChatCompletionRuntimeEvent::ContentDelta(
            ChatContentSnapshotEvent {
                choice_index: choice.index,
                delta: delta.clone(),
                parsed: parse_optional_json(&snapshot_content),
                snapshot: snapshot_content,
            },
        ));
    }

    if let Some(delta) = &choice.delta.refusal
        && let Some(snapshot_refusal) = snapshot_choice.message.refusal.clone()
    {
        events.push(ChatCompletionRuntimeEvent::RefusalDelta(
            ChatRefusalSnapshotEvent {
                choice_index: choice.index,
                delta: delta.clone(),
                snapshot: snapshot_refusal,
            },
        ));
    }

    if let Some(values) = logprobs_values(choice.logprobs.as_ref(), "content") {
        events.push(ChatCompletionRuntimeEvent::LogProbsContentDelta(
            ChatLogProbsSnapshotEvent {
                choice_index: choice.index,
                snapshot: logprobs_values(snapshot_choice.logprobs.as_ref(), "content")
                    .unwrap_or_default(),
                values,
            },
        ));
    }

    if let Some(values) = logprobs_values(choice.logprobs.as_ref(), "refusal") {
        events.push(ChatCompletionRuntimeEvent::LogProbsRefusalDelta(
            ChatLogProbsSnapshotEvent {
                choice_index: choice.index,
                snapshot: logprobs_values(snapshot_choice.logprobs.as_ref(), "refusal")
                    .unwrap_or_default(),
                values,
            },
        ));
    }

    for tool_call in &choice.delta.tool_calls {
        let tool_call_index = tool_call.index.unwrap_or_default();
        if state.current_tool_call_index != Some(tool_call_index) {
            if let Some(previous_index) = state.current_tool_call_index.take() {
                emit_chat_tool_call_done(
                    &mut events,
                    choice.index,
                    previous_index,
                    snapshot_choice,
                    &mut state,
                );
            }
            state.current_tool_call_index = Some(tool_call_index);
        }

        if let Some(arguments_delta) = tool_call
            .function
            .as_ref()
            .and_then(|function| function.arguments.clone())
            && let Some(snapshot_tool_call) = snapshot_choice
                .message
                .tool_calls
                .get(tool_call_index as usize)
        {
            events.push(ChatCompletionRuntimeEvent::ToolCallArgumentsDelta(
                ChatToolArgumentsSnapshotEvent {
                    choice_index: choice.index,
                    tool_call_index,
                    name: snapshot_tool_call.function.name.clone(),
                    parsed_arguments: parse_optional_json(&snapshot_tool_call.function.arguments),
                    arguments_delta,
                    arguments: snapshot_tool_call.function.arguments.clone(),
                },
            ));
        }
    }

    if choice.finish_reason.is_some() || snapshot_choice.finish_reason.is_some() {
        emit_chat_choice_done_events(&mut events, choice.index, snapshot_choice, &mut state);
    }

    (events, state)
}

fn emit_chat_choice_done_events(
    events: &mut Vec<ChatCompletionRuntimeEvent>,
    choice_index: u32,
    snapshot_choice: &crate::resources::ChatCompletionChoice,
    state: &mut ChatChoiceEventState,
) {
    if !state.content_done
        && let Some(content) = snapshot_choice.message.content.clone()
    {
        events.push(ChatCompletionRuntimeEvent::ContentDone(
            ChatContentDoneEvent {
                choice_index,
                parsed: parse_optional_json(&content),
                content,
            },
        ));
        state.content_done = true;
    }

    if !state.refusal_done
        && let Some(refusal) = snapshot_choice.message.refusal.clone()
    {
        events.push(ChatCompletionRuntimeEvent::RefusalDone(
            ChatRefusalDoneEvent {
                choice_index,
                refusal,
            },
        ));
        state.refusal_done = true;
    }

    if !state.logprobs_content_done
        && let Some(values) = logprobs_values(snapshot_choice.logprobs.as_ref(), "content")
    {
        events.push(ChatCompletionRuntimeEvent::LogProbsContentDone(
            ChatLogProbsDoneEvent {
                choice_index,
                values,
            },
        ));
        state.logprobs_content_done = true;
    }

    if !state.logprobs_refusal_done
        && let Some(values) = logprobs_values(snapshot_choice.logprobs.as_ref(), "refusal")
    {
        events.push(ChatCompletionRuntimeEvent::LogProbsRefusalDone(
            ChatLogProbsDoneEvent {
                choice_index,
                values,
            },
        ));
        state.logprobs_refusal_done = true;
    }

    if let Some(tool_call_index) = state.current_tool_call_index.take() {
        emit_chat_tool_call_done(
            events,
            choice_index,
            tool_call_index,
            snapshot_choice,
            state,
        );
    }
}

fn emit_chat_tool_call_done(
    events: &mut Vec<ChatCompletionRuntimeEvent>,
    choice_index: u32,
    tool_call_index: u32,
    snapshot_choice: &crate::resources::ChatCompletionChoice,
    state: &mut ChatChoiceEventState,
) {
    if state.done_tool_calls.contains(&tool_call_index) {
        return;
    }

    let Some(snapshot_tool_call) = snapshot_choice
        .message
        .tool_calls
        .get(tool_call_index as usize)
    else {
        return;
    };

    events.push(ChatCompletionRuntimeEvent::ToolCallArgumentsDone(
        ChatToolArgumentsDoneEvent {
            choice_index,
            tool_call_index,
            name: snapshot_tool_call.function.name.clone(),
            parsed_arguments: parse_optional_json(&snapshot_tool_call.function.arguments),
            arguments: snapshot_tool_call.function.arguments.clone(),
        },
    ));
    state.done_tool_calls.insert(tool_call_index);
}

fn derive_response_runtime_event(event: Value, snapshot: Option<Response>) -> ResponseRuntimeEvent {
    let event_type = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    match event_type.as_str() {
        "response.created" => snapshot
            .map(ResponseRuntimeEvent::ResponseCreated)
            .unwrap_or(ResponseRuntimeEvent::Raw(event)),
        "response.output_item.added" => {
            if let (Some(output_index), Some(item), Some(snapshot)) = (
                event
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
                event.get("item").cloned(),
                snapshot,
            ) {
                ResponseRuntimeEvent::OutputItemAdded {
                    output_index,
                    item,
                    snapshot,
                }
            } else {
                ResponseRuntimeEvent::Raw(event)
            }
        }
        "response.content_part.added" => {
            if let (Some(output_index), Some(content_index), Some(part), Some(snapshot)) = (
                event
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
                event
                    .get("content_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize),
                event.get("part").cloned(),
                snapshot,
            ) {
                ResponseRuntimeEvent::ContentPartAdded {
                    output_index,
                    content_index,
                    part,
                    snapshot,
                }
            } else {
                ResponseRuntimeEvent::Raw(event)
            }
        }
        "response.output_text.delta" | "response.output_text.done" => {
            let output_index = event
                .get("output_index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_default();
            let content_index = event
                .get("content_index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_default();
            let text = event
                .get("delta")
                .or_else(|| event.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let snapshot_text = snapshot
                .as_ref()
                .and_then(|response| {
                    response_output_text_snapshot(response, output_index, content_index)
                })
                .unwrap_or_else(|| text.clone());
            let typed_event = ResponseOutputTextEvent {
                event_type: event_type.clone(),
                output_index,
                content_index,
                text,
                snapshot: snapshot_text,
            };
            if event_type == "response.output_text.delta" {
                ResponseRuntimeEvent::OutputTextDelta(typed_event)
            } else {
                ResponseRuntimeEvent::OutputTextDone(typed_event)
            }
        }
        "response.function_call_arguments.delta" => {
            let output_index = event
                .get("output_index")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_default();
            let item_id = event
                .get("item_id")
                .or_else(|| event.get("call_id"))
                .and_then(Value::as_str)
                .map(str::to_owned);
            let delta = event
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let snapshot_arguments = snapshot
                .as_ref()
                .and_then(|response| response_function_arguments_snapshot(response, output_index))
                .unwrap_or_else(|| delta.clone());
            ResponseRuntimeEvent::FunctionCallArgumentsDelta(ResponseFunctionCallArgumentsEvent {
                output_index,
                parsed_arguments: parse_optional_json(&snapshot_arguments),
                item_id,
                delta,
                snapshot: snapshot_arguments,
            })
        }
        "response.completed" => snapshot
            .map(ResponseRuntimeEvent::Completed)
            .unwrap_or(ResponseRuntimeEvent::Raw(event)),
        _ => ResponseRuntimeEvent::Raw(event),
    }
}

fn response_output_text_snapshot(
    response: &Response,
    output_index: usize,
    content_index: usize,
) -> Option<String> {
    let output = response.output.get(output_index)?;
    let content = output.get("content")?.as_array()?;
    let item = content.get(content_index)?;
    item.get("text").and_then(Value::as_str).map(str::to_owned)
}

fn response_function_arguments_snapshot(
    response: &Response,
    output_index: usize,
) -> Option<String> {
    response
        .output
        .get(output_index)
        .and_then(|output| output.get("arguments"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[derive(Debug, Default, Clone)]
struct ResponseAccumulator {
    response: Option<Response>,
    output_text: String,
    function_arguments: HashMap<String, String>,
}

impl ResponseAccumulator {
    fn apply(&mut self, event: &Value) {
        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            return;
        };

        match event_type {
            "response.created" => {
                if let Some(response) = event.get("response") {
                    self.response = serde_json::from_value(response.clone()).ok();
                    self.sync_output_text_from_snapshot();
                }
            }
            "response.output_item.added" => {
                let Some(response) = &mut self.response else {
                    return;
                };
                let Some(item) = event.get("item") else {
                    return;
                };
                let index = event
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or(response.output.len());
                ensure_vec_len(&mut response.output, index + 1);
                response.output[index] = item.clone();
                self.sync_output_text_from_snapshot();
            }
            "response.content_part.added" => {
                let Some(response) = &mut self.response else {
                    return;
                };
                let Some(part) = event.get("part") else {
                    return;
                };
                let output_index = event
                    .get("output_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or_default();
                let content_index = event
                    .get("content_index")
                    .and_then(Value::as_u64)
                    .map(|value| value as usize)
                    .unwrap_or_default();
                if let Some(output) = response.output.get_mut(output_index) {
                    let content = ensure_array_field(output, "content");
                    ensure_vec_len(content, content_index + 1);
                    content[content_index] = part.clone();
                    self.sync_output_text_from_snapshot();
                }
            }
            "response.output_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    self.output_text.push_str(delta);
                }
                if let Some(response) = &mut self.response {
                    append_response_content_text(response, event, "text", "output_text");
                }
            }
            "response.output_text.done" => {
                if self.output_text.is_empty()
                    && let Some(text) = event.get("text").and_then(Value::as_str)
                {
                    self.output_text = text.to_owned();
                }
                if let Some(response) = &mut self.response {
                    set_response_content_text(response, event, "text", "output_text");
                }
            }
            "response.function_call_arguments.delta" => {
                let key = event
                    .get("item_id")
                    .and_then(Value::as_str)
                    .or_else(|| event.get("call_id").and_then(Value::as_str))
                    .unwrap_or("default");
                let delta = event.get("delta").and_then(Value::as_str).unwrap_or("");
                self.function_arguments
                    .entry(key.to_owned())
                    .and_modify(|value| value.push_str(delta))
                    .or_insert_with(|| delta.to_owned());
                if let Some(response) = &mut self.response {
                    append_function_call_arguments(response, event, delta);
                }
            }
            "response.reasoning_text.delta" => {
                if let Some(response) = &mut self.response {
                    append_response_content_text(response, event, "text", "reasoning_text");
                    self.sync_output_text_from_snapshot();
                }
            }
            "response.completed" => {
                if let Some(response) = event.get("response") {
                    self.response = serde_json::from_value(response.clone()).ok();
                    self.sync_output_text_from_snapshot();
                }
            }
            _ => {}
        }
    }

    fn sync_output_text_from_snapshot(&mut self) {
        if let Some(response) = &self.response
            && let Some(text) = response.output_text()
        {
            self.output_text = text;
        }
    }
}

impl AssistantStreamSnapshot {
    fn apply(&mut self, event: &AssistantStreamEvent) {
        match event.event.as_str() {
            "thread.created" => {
                self.thread = Some(event.data.clone());
            }
            "thread.run.created"
            | "thread.run.queued"
            | "thread.run.in_progress"
            | "thread.run.requires_action"
            | "thread.run.completed"
            | "thread.run.incomplete"
            | "thread.run.failed"
            | "thread.run.cancelling"
            | "thread.run.cancelled"
            | "thread.run.expired" => {
                if let Some(id) = event.data.get("id").and_then(Value::as_str) {
                    self.latest_run_id = Some(id.to_owned());
                    self.runs.insert(id.to_owned(), event.data.clone());
                }
            }
            "thread.message.created"
            | "thread.message.in_progress"
            | "thread.message.completed"
            | "thread.message.incomplete" => {
                if let Some(id) = event.data.get("id").and_then(Value::as_str) {
                    self.latest_message_id = Some(id.to_owned());
                    self.messages.insert(id.to_owned(), event.data.clone());
                }
            }
            "thread.run.step.created"
            | "thread.run.step.in_progress"
            | "thread.run.step.completed"
            | "thread.run.step.failed"
            | "thread.run.step.cancelled"
            | "thread.run.step.expired" => {
                if let Some(id) = event.data.get("id").and_then(Value::as_str) {
                    self.latest_run_step_id = Some(id.to_owned());
                    self.run_steps.insert(id.to_owned(), event.data.clone());
                }
            }
            "thread.message.delta" => {
                if let Some(id) = event.data.get("id").and_then(Value::as_str) {
                    self.latest_message_id = Some(id.to_owned());
                    let entry = self
                        .messages
                        .entry(id.to_owned())
                        .or_insert_with(|| empty_assistant_snapshot(id, "thread.message"));
                    apply_message_delta(entry, &event.data);
                }
            }
            "thread.run.step.delta" => {
                if let Some(id) = event.data.get("id").and_then(Value::as_str) {
                    self.latest_run_step_id = Some(id.to_owned());
                    let entry = self
                        .run_steps
                        .entry(id.to_owned())
                        .or_insert_with(|| empty_assistant_snapshot(id, "thread.run.step"));
                    apply_run_step_delta(entry, &event.data);
                }
            }
            _ => {}
        }
    }
}

fn ensure_vec_len(values: &mut Vec<Value>, len: usize) {
    while values.len() < len {
        values.push(Value::Null);
    }
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("value must be object")
}

fn ensure_array_field<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    let object = ensure_object(value);
    let field = object
        .entry(key.to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !field.is_array() {
        *field = Value::Array(Vec::new());
    }
    field.as_array_mut().expect("field must be array")
}

fn ensure_object_field<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    let object = ensure_object(value);
    let field = object
        .entry(key.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    ensure_object(field)
}

fn empty_assistant_snapshot(id: &str, object: &str) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), Value::String(id.to_owned()));
    map.insert("object".into(), Value::String(object.to_owned()));
    Value::Object(map)
}

fn append_response_content_text(
    response: &mut Response,
    event: &Value,
    field_name: &str,
    default_type: &str,
) {
    let output_index = event
        .get("output_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let content_index = event
        .get("content_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let delta = event.get("delta").and_then(Value::as_str).unwrap_or("");

    let Some(output) = response.output.get_mut(output_index) else {
        return;
    };
    let content = ensure_array_field(output, "content");
    ensure_vec_len(content, content_index + 1);
    if content[content_index].is_null() {
        let mut content_map = Map::new();
        content_map.insert("type".into(), Value::String(default_type.to_owned()));
        content_map.insert(field_name.into(), Value::String(String::new()));
        content[content_index] = Value::Object(content_map);
    }

    let slot = &mut content[content_index];
    let slot_object = ensure_object(slot);
    slot_object
        .entry("type")
        .or_insert_with(|| Value::String(default_type.to_owned()));
    match field_name {
        "text" => {
            let text = slot_object
                .entry("text")
                .or_insert_with(|| Value::String(String::new()));
            if let Some(existing) = text.as_str() {
                *text = Value::String(format!("{existing}{delta}"));
            } else {
                *text = Value::String(delta.to_owned());
            }
        }
        _ => {
            let nested = slot_object
                .entry(field_name)
                .or_insert_with(|| Value::Object(Map::new()));
            let nested_object = ensure_object(nested);
            let text = nested_object
                .entry("text")
                .or_insert_with(|| Value::String(String::new()));
            if let Some(existing) = text.as_str() {
                *text = Value::String(format!("{existing}{delta}"));
            } else {
                *text = Value::String(delta.to_owned());
            }
        }
    }
}

fn set_response_content_text(
    response: &mut Response,
    event: &Value,
    field_name: &str,
    default_type: &str,
) {
    let Some(text) = event.get("text").and_then(Value::as_str) else {
        return;
    };
    let output_index = event
        .get("output_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let content_index = event
        .get("content_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();

    let Some(output) = response.output.get_mut(output_index) else {
        return;
    };
    let content = ensure_array_field(output, "content");
    ensure_vec_len(content, content_index + 1);
    if content[content_index].is_null() {
        let mut content_map = Map::new();
        content_map.insert("type".into(), Value::String(default_type.to_owned()));
        content[content_index] = Value::Object(content_map);
    }

    let slot = &mut content[content_index];
    let slot_object = ensure_object(slot);
    slot_object.insert("type".into(), Value::String(default_type.to_owned()));
    match field_name {
        "text" => {
            slot_object.insert("text".into(), Value::String(text.to_owned()));
        }
        _ => {
            let nested = slot_object
                .entry(field_name)
                .or_insert_with(|| Value::Object(Map::new()));
            let nested_object = ensure_object(nested);
            nested_object.insert("text".into(), Value::String(text.to_owned()));
        }
    }
}

fn append_function_call_arguments(response: &mut Response, event: &Value, delta: &str) {
    let output_index = event
        .get("output_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_default();
    let Some(output) = response.output.get_mut(output_index) else {
        return;
    };
    let object = ensure_object(output);
    object
        .entry("type")
        .or_insert_with(|| Value::String("function_call".into()));
    let arguments = object
        .entry("arguments")
        .or_insert_with(|| Value::String(String::new()));
    if let Some(existing) = arguments.as_str() {
        *arguments = Value::String(format!("{existing}{delta}"));
    } else {
        *arguments = Value::String(delta.to_owned());
    }
}

fn apply_message_delta(message: &mut Value, event: &Value) {
    let Some(delta) = event.get("delta") else {
        return;
    };
    if let Some(role) = delta.get("role").and_then(Value::as_str) {
        ensure_object(message).insert("role".into(), Value::String(role.to_owned()));
    }

    let Some(content_deltas) = delta.get("content").and_then(Value::as_array) else {
        return;
    };
    let content = ensure_array_field(message, "content");
    for content_delta in content_deltas {
        let index = content_delta
            .get("index")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(content.len());
        ensure_vec_len(content, index + 1);
        if content[index].is_null() {
            content[index] = Value::Object(Map::new());
        }

        let slot = &mut content[index];
        let slot_object = ensure_object(slot);
        if let Some(part_type) = content_delta.get("type").and_then(Value::as_str) {
            slot_object.insert("type".into(), Value::String(part_type.to_owned()));
            match part_type {
                "text" => {
                    let text_object = ensure_object_field(slot, "text");
                    let value = content_delta
                        .get("text")
                        .and_then(|value| value.get("value"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let current = text_object
                        .get("value")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    text_object.insert("value".into(), Value::String(format!("{current}{value}")));
                }
                "refusal" => {
                    let value = content_delta
                        .get("refusal")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let current = slot_object
                        .get("refusal")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    slot_object
                        .insert("refusal".into(), Value::String(format!("{current}{value}")));
                }
                _ => merge_object(slot_object, content_delta),
            }
        }
    }
}

fn apply_run_step_delta(run_step: &mut Value, event: &Value) {
    let Some(delta) = event.get("delta") else {
        return;
    };
    let Some(step_details) = delta.get("step_details") else {
        return;
    };
    let step_details_object = ensure_object_field(run_step, "step_details");
    if let Some(step_type) = step_details.get("type").and_then(Value::as_str) {
        step_details_object.insert("type".into(), Value::String(step_type.to_owned()));
        match step_type {
            "message_creation" => {
                if let Some(message_creation) = step_details.get("message_creation") {
                    let target = step_details_object
                        .entry("message_creation")
                        .or_insert_with(|| Value::Object(Map::new()));
                    merge_object(ensure_object(target), message_creation);
                }
            }
            "tool_calls" => {
                let tool_calls = step_details
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let target = step_details_object
                    .entry("tool_calls")
                    .or_insert_with(|| Value::Array(Vec::new()));
                let target_calls = if let Some(array) = target.as_array_mut() {
                    array
                } else {
                    *target = Value::Array(Vec::new());
                    target.as_array_mut().expect("tool_calls must be array")
                };
                for tool_call in tool_calls {
                    let index = tool_call
                        .get("index")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize)
                        .unwrap_or(target_calls.len());
                    ensure_vec_len(target_calls, index + 1);
                    if target_calls[index].is_null() {
                        target_calls[index] = Value::Object(Map::new());
                    }
                    merge_tool_call_delta(&mut target_calls[index], &tool_call);
                }
            }
            _ => merge_object(step_details_object, step_details),
        }
    }
}

fn merge_tool_call_delta(target: &mut Value, delta: &Value) {
    let target_object = ensure_object(target);
    merge_object(target_object, delta);
    if let Some(function_delta) = delta.get("function") {
        let function_target = target_object
            .entry("function")
            .or_insert_with(|| Value::Object(Map::new()));
        let function_object = ensure_object(function_target);
        if let Some(arguments) = function_delta.get("arguments").and_then(Value::as_str) {
            let current = function_object
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("");
            function_object.insert(
                "arguments".into(),
                Value::String(format!("{current}{arguments}")),
            );
        }
        if let Some(name) = function_delta.get("name").and_then(Value::as_str) {
            function_object.insert("name".into(), Value::String(name.to_owned()));
        }
    }
    if let Some(code_interpreter_delta) = delta.get("code_interpreter") {
        let code_interpreter_target = target_object
            .entry("code_interpreter")
            .or_insert_with(|| Value::Object(Map::new()));
        let code_interpreter_object = ensure_object(code_interpreter_target);
        if let Some(input) = code_interpreter_delta.get("input").and_then(Value::as_str) {
            let current = code_interpreter_object
                .get("input")
                .and_then(Value::as_str)
                .unwrap_or("");
            code_interpreter_object
                .insert("input".into(), Value::String(format!("{current}{input}")));
        }
        if let Some(outputs) = code_interpreter_delta
            .get("outputs")
            .and_then(Value::as_array)
        {
            let target_outputs = code_interpreter_object
                .entry("outputs")
                .or_insert_with(|| Value::Array(Vec::new()));
            let output_array = if let Some(array) = target_outputs.as_array_mut() {
                array
            } else {
                *target_outputs = Value::Array(Vec::new());
                target_outputs
                    .as_array_mut()
                    .expect("outputs must be array")
            };
            output_array.extend(outputs.iter().cloned());
        }
    }
}

fn merge_object(target: &mut Map<String, Value>, delta: &Value) {
    let Some(delta_object) = delta.as_object() else {
        return;
    };
    for (key, value) in delta_object {
        if matches!(value, Value::Null) {
            continue;
        }
        target.insert(key.clone(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::LineDecoder;

    #[test]
    fn test_should_decode_lines_for_mixed_newlines() {
        let mut decoder = LineDecoder::default();
        let first = decoder
            .push(b"data: one\r\ndata: two\rdata: three\n")
            .unwrap();
        assert_eq!(
            first,
            vec![
                "data: one".to_string(),
                "data: two".to_string(),
                "data: three".to_string(),
            ]
        );
        assert_eq!(decoder.finish().unwrap(), None);
    }

    #[test]
    fn test_should_decode_utf8_split_across_chunks() {
        let mut decoder = LineDecoder::default();
        let snowman = "你好";
        let bytes = snowman.as_bytes();
        let first = decoder.push(&bytes[..2]).unwrap();
        assert!(first.is_empty());
        let second = decoder.push(&bytes[2..]).unwrap();
        assert!(second.is_empty());
        let third = decoder.push(b"\n").unwrap();
        assert_eq!(third, vec![snowman.to_string()]);
    }
}
