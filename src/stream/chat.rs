use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::partial_json::parse_optional_json;
use super::sse::SseStream;
use crate::error::Result;
use crate::json_payload::JsonPayload;
use crate::resources::{
    ChatCompletion, ChatCompletionChoiceLogprobs, ChatCompletionChunk, ChatCompletionChunkDelta,
    ChatCompletionMessage, ChatCompletionTokenLogprob, ChatCompletionToolCall,
};
use crate::response_meta::ResponseMeta;

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
        while let Some(chunk) = futures_util::StreamExt::next(&mut self).await {
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
    pub parsed: Option<JsonPayload>,
}

/// 表示聊天文本完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatContentDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 完整文本。
    pub content: String,
    /// 如果完整文本是合法 JSON，则提供解析结果。
    pub parsed: Option<JsonPayload>,
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
    pub parsed_arguments: Option<JsonPayload>,
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
    pub parsed_arguments: Option<JsonPayload>,
}

/// 表示 token logprobs 增量和当前聚合快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLogProbsSnapshotEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 本次 logprobs 增量。
    pub values: Vec<ChatCompletionTokenLogprob>,
    /// 当前累计 logprobs。
    pub snapshot: Vec<ChatCompletionTokenLogprob>,
}

/// 表示 token logprobs 完成事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatLogProbsDoneEvent {
    /// 候选索引。
    pub choice_index: u32,
    /// 完整 logprobs。
    pub values: Vec<ChatCompletionTokenLogprob>,
}

/// 表示聊天流在运行时派生出的高层事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
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
        while let Some(event) = futures_util::StreamExt::next(&mut self).await {
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
    logprobs: Option<ChatCompletionChoiceLogprobs>,
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

fn merge_logprobs(
    target: &mut Option<ChatCompletionChoiceLogprobs>,
    incoming: &ChatCompletionChoiceLogprobs,
) {
    let target_logprobs = target.get_or_insert_with(ChatCompletionChoiceLogprobs::default);
    target_logprobs
        .content
        .extend(incoming.content.iter().cloned());
    target_logprobs
        .refusal
        .extend(incoming.refusal.iter().cloned());
    for (key, value) in &incoming.extra {
        target_logprobs.extra.insert(key.clone(), value.clone());
    }
}

fn logprobs_values(
    logprobs: Option<&ChatCompletionChoiceLogprobs>,
    field_name: &str,
) -> Option<Vec<ChatCompletionTokenLogprob>> {
    logprobs?.values(field_name).map(<[_]>::to_vec)
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
                parsed: parse_optional_json(&snapshot_content).map(JsonPayload::from),
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
                    parsed_arguments: parse_optional_json(&snapshot_tool_call.function.arguments)
                        .map(JsonPayload::from),
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
                parsed: parse_optional_json(&content).map(JsonPayload::from),
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
            parsed_arguments: parse_optional_json(&snapshot_tool_call.function.arguments)
                .map(JsonPayload::from),
            arguments: snapshot_tool_call.function.arguments.clone(),
        },
    ));
    state.done_tool_calls.insert(tool_call_index);
}
