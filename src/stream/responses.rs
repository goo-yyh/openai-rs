use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::partial_json::parse_optional_json;
use super::sse::SseStream;
use super::value_helpers::{ensure_array_field, ensure_object, ensure_vec_len};
use crate::error::Result;
use crate::resources::Response;
use crate::response_meta::ResponseMeta;

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
        while let Some(event) = futures_util::StreamExt::next(&mut self).await {
            event?;
        }
        Ok(self.accumulator.output_text)
    }

    /// 消费整个流并返回最终响应快照。
    pub async fn final_response(mut self) -> Result<Option<Response>> {
        while let Some(event) = futures_util::StreamExt::next(&mut self).await {
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
        while let Some(event) = futures_util::StreamExt::next(&mut self).await {
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
                let output_text = this.inner.output_text().to_owned();
                let function_arguments = this.inner.function_arguments().clone();
                Poll::Ready(Some(Ok(derive_response_runtime_event(
                    event,
                    snapshot,
                    &output_text,
                    &function_arguments,
                ))))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn derive_response_runtime_event(
    event: Value,
    snapshot: Option<Response>,
    output_text: &str,
    function_arguments: &HashMap<String, String>,
) -> ResponseRuntimeEvent {
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
                .filter(|snapshot_text| !snapshot_text.is_empty())
                .unwrap_or_else(|| {
                    if output_text.is_empty() {
                        text.clone()
                    } else {
                        output_text.to_owned()
                    }
                });
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
            let fallback_arguments = item_id
                .as_deref()
                .and_then(|key| function_arguments.get(key))
                .cloned()
                .or_else(|| function_arguments.get("default").cloned())
                .unwrap_or_else(|| delta.clone());
            let snapshot_arguments = snapshot
                .as_ref()
                .and_then(|response| response_function_arguments_snapshot(response, output_index))
                .filter(|snapshot_arguments| !snapshot_arguments.is_empty())
                .unwrap_or(fallback_arguments);
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
                let existing = response.output[index].clone();
                response.output[index] = merge_response_output_item(existing, item.clone());
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
                ensure_vec_len(&mut response.output, output_index + 1);
                if response.output[output_index].is_null() {
                    response.output[output_index] = Value::Object(Map::new());
                }
                let output = &mut response.output[output_index];
                let content = ensure_array_field(output, "content");
                ensure_vec_len(content, content_index + 1);
                let existing = content[content_index].clone();
                content[content_index] = merge_response_content_part(existing, part.clone());
                self.sync_output_text_from_snapshot();
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

fn merge_response_output_item(existing: Value, incoming: Value) -> Value {
    let (Some(existing_object), Some(mut incoming_object)) =
        (existing.as_object(), incoming.as_object().cloned())
    else {
        return incoming;
    };

    if let Some(existing_arguments) = existing_object
        .get("arguments")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        let incoming_arguments = incoming_object
            .get("arguments")
            .and_then(Value::as_str)
            .unwrap_or("");
        if incoming_arguments.is_empty() {
            incoming_object.insert(
                "arguments".into(),
                Value::String(existing_arguments.to_owned()),
            );
        }
    }

    if let Some(existing_content) = existing_object
        .get("content")
        .and_then(Value::as_array)
        .filter(|value| !value.is_empty())
        .cloned()
    {
        let use_existing_content = incoming_object
            .get("content")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty);
        if use_existing_content {
            incoming_object.insert("content".into(), Value::Array(existing_content));
        }
    }

    Value::Object(incoming_object)
}

fn merge_response_content_part(existing: Value, incoming: Value) -> Value {
    let (Some(existing_object), Some(mut incoming_object)) =
        (existing.as_object(), incoming.as_object().cloned())
    else {
        return incoming;
    };

    if let Some(existing_text) = existing_object
        .get("text")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        let incoming_text = incoming_object
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        if incoming_text.is_empty() {
            incoming_object.insert("text".into(), Value::String(existing_text.to_owned()));
        }
    }

    for key in ["output_text", "reasoning_text"] {
        let Some(existing_text) = existing_object
            .get(key)
            .and_then(|value| value.get("text"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let incoming_value = incoming_object
            .entry(key.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        let incoming_nested = ensure_object(incoming_value);
        let incoming_text = incoming_nested
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("");
        if incoming_text.is_empty() {
            incoming_nested.insert("text".into(), Value::String(existing_text.to_owned()));
        }
    }

    Value::Object(incoming_object)
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

    ensure_vec_len(&mut response.output, output_index + 1);
    if response.output[output_index].is_null() {
        response.output[output_index] = Value::Object(Map::new());
    }
    let output = &mut response.output[output_index];
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

    ensure_vec_len(&mut response.output, output_index + 1);
    if response.output[output_index].is_null() {
        response.output[output_index] = Value::Object(Map::new());
    }
    let output = &mut response.output[output_index];
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
    ensure_vec_len(&mut response.output, output_index + 1);
    if response.output[output_index].is_null() {
        response.output[output_index] = Value::Object(Map::new());
    }
    let output = &mut response.output[output_index];
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

#[cfg(test)]
mod tests {
    use super::ResponseAccumulator;
    use serde_json::json;

    #[test]
    fn test_should_keep_response_snapshot_consistent_for_out_of_order_events() {
        let mut accumulator = ResponseAccumulator::default();
        for event in [
            json!({
                "type": "response.created",
                "response": {
                    "id": "resp_1",
                    "object": "response",
                    "status": "in_progress",
                    "output": []
                }
            }),
            json!({
                "type": "response.output_text.delta",
                "output_index": 0,
                "content_index": 0,
                "delta": "hel"
            }),
            json!({
                "type": "response.output_item.added",
                "output_index": 0,
                "item": {
                    "id": "msg_1",
                    "type": "message",
                    "role": "assistant",
                    "content": []
                }
            }),
            json!({
                "type": "response.content_part.added",
                "output_index": 0,
                "content_index": 0,
                "part": {
                    "type": "output_text",
                    "text": ""
                }
            }),
            json!({
                "type": "response.output_text.delta",
                "output_index": 0,
                "content_index": 0,
                "delta": "lo"
            }),
            json!({
                "type": "response.function_call_arguments.delta",
                "output_index": 1,
                "item_id": "fc_1",
                "delta": "{\"city\":\"Sha"
            }),
            json!({
                "type": "response.output_item.added",
                "output_index": 1,
                "item": {
                    "id": "fc_1",
                    "type": "function_call",
                    "arguments": ""
                }
            }),
            json!({
                "type": "response.function_call_arguments.delta",
                "output_index": 1,
                "item_id": "fc_1",
                "delta": "nghai\"}"
            }),
        ] {
            accumulator.apply(&event);
        }

        let response = accumulator.response.clone().unwrap();
        assert_eq!(accumulator.output_text, "hello");
        assert_eq!(response.output_text().as_deref(), Some("hello"));
        assert_eq!(
            response.output[1]
                .get("arguments")
                .and_then(serde_json::Value::as_str),
            Some("{\"city\":\"Shanghai\"}"),
        );
    }
}
