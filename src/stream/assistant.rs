use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::sse::RawSseStream;
use super::value_helpers::{
    ensure_array_field, ensure_object, ensure_object_field, ensure_vec_len, merge_object,
};
use crate::error::{Result, SerializationError, StreamError};
use crate::response_meta::ResponseMeta;

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

fn empty_assistant_snapshot(id: &str, object: &str) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), Value::String(id.to_owned()));
    map.insert("object".into(), Value::String(object.to_owned()));
    Value::Object(map)
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
    if let Some(delta_object) = delta.as_object() {
        for (key, value) in delta_object {
            if matches!(key.as_str(), "function" | "code_interpreter")
                || matches!(value, Value::Null)
            {
                continue;
            }
            target_object.insert(key.clone(), value.clone());
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{AssistantStreamEvent, AssistantStreamSnapshot};
    use serde_json::json;

    #[test]
    fn test_should_merge_assistant_deltas_into_snapshot_before_created_events() {
        let mut snapshot = AssistantStreamSnapshot::default();

        snapshot.apply(&AssistantStreamEvent {
            event: "thread.message.delta".into(),
            data: json!({
                "id": "msg_1",
                "object": "thread.message.delta",
                "delta": {
                    "content": [{
                        "index": 0,
                        "type": "text",
                        "text": { "value": "hel" }
                    }]
                }
            }),
        });
        snapshot.apply(&AssistantStreamEvent {
            event: "thread.message.delta".into(),
            data: json!({
                "id": "msg_1",
                "object": "thread.message.delta",
                "delta": {
                    "content": [{
                        "index": 0,
                        "type": "text",
                        "text": { "value": "lo" }
                    }]
                }
            }),
        });
        snapshot.apply(&AssistantStreamEvent {
            event: "thread.run.step.delta".into(),
            data: json!({
                "id": "step_1",
                "object": "thread.run.step.delta",
                "delta": {
                    "step_details": {
                        "type": "tool_calls",
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "name": "lookup_weather",
                                "arguments": "{\"city\":\"Sha"
                            }
                        }]
                    }
                }
            }),
        });
        snapshot.apply(&AssistantStreamEvent {
            event: "thread.run.step.delta".into(),
            data: json!({
                "id": "step_1",
                "object": "thread.run.step.delta",
                "delta": {
                    "step_details": {
                        "type": "tool_calls",
                        "tool_calls": [{
                            "index": 0,
                            "type": "function",
                            "function": {
                                "arguments": "nghai\"}"
                            }
                        }]
                    }
                }
            }),
        });

        assert_eq!(
            snapshot
                .message_raw("msg_1")
                .and_then(|message| message.get("content"))
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.first())
                .and_then(|part| part.get("text"))
                .and_then(|text| text.get("value"))
                .and_then(serde_json::Value::as_str),
            Some("hello"),
        );
        assert_eq!(
            snapshot
                .run_step_raw("step_1")
                .and_then(|step| step.get("step_details"))
                .and_then(|details| details.get("tool_calls"))
                .and_then(serde_json::Value::as_array)
                .and_then(|tool_calls| tool_calls.first())
                .and_then(|tool_call| tool_call.get("function"))
                .and_then(|function| function.get("arguments"))
                .and_then(serde_json::Value::as_str),
            Some("{\"city\":\"Shanghai\"}"),
        );
    }
}
