//! Streaming runtime split into protocol, chat, responses, and assistant layers.

mod assistant;
mod chat;
mod partial_json;
mod responses;
mod sse;
mod value_helpers;

pub use assistant::{
    AssistantEventStream, AssistantImageFileDoneEvent, AssistantMessageCreatedEvent,
    AssistantMessageDeltaEvent, AssistantMessageDoneEvent, AssistantRunStepCreatedEvent,
    AssistantRunStepDeltaEvent, AssistantRunStepDoneEvent, AssistantRuntimeEvent, AssistantStream,
    AssistantStreamEvent, AssistantStreamSnapshot, AssistantTextCreatedEvent,
    AssistantTextDeltaEvent, AssistantTextDoneEvent, AssistantToolCallCreatedEvent,
    AssistantToolCallDeltaEvent, AssistantToolCallDoneEvent,
};
pub use chat::{
    ChatCompletionEventStream, ChatCompletionRuntimeEvent, ChatCompletionStream,
    ChatContentDoneEvent, ChatContentSnapshotEvent, ChatLogProbsDoneEvent,
    ChatLogProbsSnapshotEvent, ChatRefusalDoneEvent, ChatRefusalSnapshotEvent,
    ChatToolArgumentsDoneEvent, ChatToolArgumentsSnapshotEvent,
};
pub use responses::{
    ResponseEventStream, ResponseFunctionCallArgumentsEvent, ResponseOutputTextEvent,
    ResponseRuntimeEvent, ResponseStream,
};
pub use sse::{LineDecoder, RawSseStream, SseEvent, SseStream};
