//! SSE、行解码与流式聚合。

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use serde_json::Value;

use crate::error::{Result, SerializationError, StreamError};
use crate::resources::{
    ChatCompletion, ChatCompletionChunk, ChatCompletionChunkDelta, ChatCompletionMessage,
    ChatCompletionToolCall,
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

    /// 消费整个流并返回最终文本快照。
    pub async fn into_output_text(mut self) -> Result<String> {
        while let Some(event) = self.next().await {
            event?;
        }
        Ok(self.accumulator.output_text)
    }

    /// 返回底层响应元信息。
    pub fn meta(&self) -> &ResponseMeta {
        self.inner.meta()
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
    reasoning_content: String,
    finish_reason: Option<String>,
    tool_calls: HashMap<u32, ChatCompletionToolCall>,
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
                        tool_calls: choice.tool_calls.values().cloned().collect(),
                        reasoning_content: (!choice.reasoning_content.is_empty())
                            .then(|| choice.reasoning_content.clone()),
                        reasoning_details: Vec::new(),
                        extra,
                    },
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
                    arguments: tool_call
                        .function
                        .as_ref()
                        .and_then(|function| function.arguments.clone())
                        .unwrap_or_default(),
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

#[derive(Debug, Default)]
struct ResponseAccumulator {
    output_text: String,
    function_arguments: HashMap<String, String>,
}

impl ResponseAccumulator {
    fn apply(&mut self, event: &Value) {
        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            return;
        };

        match event_type {
            "response.output_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    self.output_text.push_str(delta);
                }
            }
            "response.output_text.done" => {
                if self.output_text.is_empty()
                    && let Some(text) = event.get("text").and_then(Value::as_str)
                {
                    self.output_text = text.to_owned();
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
            }
            _ => {}
        }
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
