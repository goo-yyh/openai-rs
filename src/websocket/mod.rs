//! WebSocket protocol models and transport implementation.

mod core;
mod events;

use crate::{Client, Result};

pub use core::*;
pub use events::*;

/// Standalone OpenAI Realtime WebSocket connector.
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAIRealtimeWebSocket;

/// Backwards-compatible alias for the standalone Realtime connector.
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub type OpenAIRealtimeWS = OpenAIRealtimeWebSocket;

#[cfg(feature = "realtime")]
impl OpenAIRealtimeWebSocket {
    /// Connect using the standalone Realtime helper.
    pub async fn connect(client: Client, model: impl Into<String>) -> Result<RealtimeSocket> {
        client.realtime().ws().model(model).connect().await
    }
}

/// Standalone OpenAI Responses WebSocket connector.
#[cfg(feature = "responses-ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "responses-ws")))]
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAIResponsesWebSocket;

#[cfg(feature = "responses-ws")]
impl OpenAIResponsesWebSocket {
    /// Connect using the standalone Responses helper.
    pub async fn connect(client: Client) -> Result<ResponsesSocket> {
        client.responses().ws().connect().await
    }
}
