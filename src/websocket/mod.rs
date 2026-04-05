//! Realtime 与 Responses WebSocket 能力封装。

use std::collections::BTreeMap;

use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result, SerializationError, WebSocketError};

/// 表示服务端推送的通用 WebSocket 事件。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WebSocketServerEvent {
    /// 事件类型。
    #[serde(rename = "type", default)]
    pub event_type: String,
    /// 除 `type` 外的原始负载字段。
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}

impl WebSocketServerEvent {
    /// 判断当前事件是否为错误事件。
    pub fn is_error(&self) -> bool {
        self.event_type == "error"
    }

    /// 尝试从错误事件中提取可读错误消息。
    pub fn error_message(&self) -> Option<String> {
        self.data
            .get("error")
            .and_then(|value| {
                value
                    .get("message")
                    .or_else(|| value.get("error"))
                    .or_else(|| value.get("detail"))
            })
            .or_else(|| self.data.get("message"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

/// Realtime 服务端事件。
pub type RealtimeServerEvent = WebSocketServerEvent;

/// Responses 服务端事件。
pub type ResponsesServerEvent = WebSocketServerEvent;

/// 表示 WebSocket 流中的生命周期或消息事件。
#[derive(Debug, Clone)]
pub enum SocketStreamMessage<T> {
    /// 连接正在建立。
    Connecting,
    /// 连接已建立。
    Open,
    /// 连接正在关闭。
    Closing,
    /// 连接已经关闭。
    Close,
    /// 收到服务端消息。
    Message(T),
    /// 收到协议层或业务层错误。
    Error(WebSocketError),
}

/// Realtime WebSocket 流消息。
pub type RealtimeStreamMessage = SocketStreamMessage<RealtimeServerEvent>;

/// Responses WebSocket 流消息。
pub type ResponsesStreamMessage = SocketStreamMessage<ResponsesServerEvent>;

/// 表示关闭 WebSocket 时附带的参数。
#[derive(Debug, Clone)]
pub struct SocketCloseOptions {
    /// WebSocket close code。
    pub code: u16,
    /// 关闭原因。
    pub reason: String,
}

impl Default for SocketCloseOptions {
    fn default() -> Self {
        Self {
            code: 1000,
            reason: "OK".into(),
        }
    }
}

#[cfg(any(feature = "realtime", feature = "responses-ws"))]
mod enabled {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    use futures_util::{SinkExt, StreamExt};
    use serde::Serialize;
    use tokio::sync::{Mutex, broadcast};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::protocol::frame::Utf8Bytes;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    use super::{
        BoxStream, Client, Error, RealtimeServerEvent, RealtimeStreamMessage, RequestOptions,
        ResponsesServerEvent, ResponsesStreamMessage, Result, SerializationError,
        SocketCloseOptions, SocketStreamMessage, Url, WebSocketError,
    };
    use crate::providers::ProviderKind;
    use crate::transport::{join_url, prepare_request_context};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ConnectionState {
        Connecting,
        Open,
        Closing,
        Closed,
    }

    impl ConnectionState {
        fn as_u8(self) -> u8 {
            match self {
                Self::Connecting => 0,
                Self::Open => 1,
                Self::Closing => 2,
                Self::Closed => 3,
            }
        }

        fn from_u8(value: u8) -> Self {
            match value {
                0 => Self::Connecting,
                1 => Self::Open,
                2 => Self::Closing,
                _ => Self::Closed,
            }
        }

        fn into_message<T>(self) -> SocketStreamMessage<T> {
            match self {
                Self::Connecting => SocketStreamMessage::Connecting,
                Self::Open => SocketStreamMessage::Open,
                Self::Closing => SocketStreamMessage::Closing,
                Self::Closed => SocketStreamMessage::Close,
            }
        }
    }

    type WsSink = futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >;

    struct SocketCore<T> {
        url: Url,
        state: AtomicU8,
        events: broadcast::Sender<SocketStreamMessage<T>>,
        sink: Mutex<WsSink>,
    }

    impl<T> std::fmt::Debug for SocketCore<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("SocketCore")
                .field("url", &self.url)
                .field(
                    "state",
                    &ConnectionState::from_u8(self.state.load(Ordering::SeqCst)),
                )
                .finish()
        }
    }

    impl<T> SocketCore<T>
    where
        T: Clone + Send + 'static,
    {
        fn stream(&self) -> BoxStream<'static, SocketStreamMessage<T>> {
            let initial =
                ConnectionState::from_u8(self.state.load(Ordering::SeqCst)).into_message();
            let receiver = self.events.subscribe();
            Box::pin(futures_util::stream::unfold(
                (Some(initial), receiver, false),
                |(initial, mut receiver, closed)| async move {
                    if closed {
                        return None;
                    }

                    if let Some(message) = initial {
                        let closed = matches!(message, SocketStreamMessage::Close);
                        return Some((message, (None, receiver, closed)));
                    }

                    loop {
                        match receiver.recv().await {
                            Ok(message) => {
                                let closed = matches!(message, SocketStreamMessage::Close);
                                return Some((message, (None, receiver, closed)));
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                        }
                    }
                },
            ))
        }
    }

    /// 表示 Realtime WebSocket 连接句柄。
    #[derive(Debug, Clone)]
    pub struct RealtimeSocket {
        inner: Arc<SocketCore<RealtimeServerEvent>>,
    }

    /// 表示 Responses WebSocket 连接句柄。
    #[derive(Debug, Clone)]
    pub struct ResponsesSocket {
        inner: Arc<SocketCore<ResponsesServerEvent>>,
    }

    impl RealtimeSocket {
        /// 建立 Realtime WebSocket 连接。
        pub(crate) async fn connect(
            client: &Client,
            model: Option<String>,
            mut options: RequestOptions,
        ) -> Result<Self> {
            match client.provider().kind() {
                ProviderKind::Azure => {
                    if let Some(model) = model {
                        options.insert_query("deployment", model);
                    }
                    let socket =
                        connect_socket(client, "realtime.ws.connect", "/realtime", options).await?;
                    if !socket.url.query_pairs().any(|(key, _)| key == "deployment") {
                        return Err(Error::InvalidConfig(
                            "Azure Realtime WebSocket 连接缺少 deployment".into(),
                        ));
                    }
                    Ok(Self { inner: socket })
                }
                _ => {
                    let Some(model) = model else {
                        return Err(Error::InvalidConfig(
                            "Realtime WebSocket 连接缺少 model".into(),
                        ));
                    };
                    options.insert_query("model", model);
                    Ok(Self {
                        inner: connect_socket(client, "realtime.ws.connect", "/realtime", options)
                            .await?,
                    })
                }
            }
        }

        /// 返回当前连接的 URL。
        pub fn url(&self) -> &Url {
            &self.inner.url
        }

        /// 返回一个可迭代的事件流。
        pub fn stream(&self) -> BoxStream<'static, RealtimeStreamMessage> {
            self.inner.stream()
        }

        /// 发送一个可序列化事件。
        ///
        /// # Errors
        ///
        /// 当序列化失败或发送失败时返回错误。
        pub async fn send_json<T>(&self, event: &T) -> Result<()>
        where
            T: Serialize,
        {
            send_json(&self.inner, event).await
        }

        /// 主动关闭连接。
        ///
        /// # Errors
        ///
        /// 当发送 close frame 失败时返回错误。
        pub async fn close(&self, options: SocketCloseOptions) -> Result<()> {
            close_socket(&self.inner, options).await
        }
    }

    impl ResponsesSocket {
        /// 建立 Responses WebSocket 连接。
        pub(crate) async fn connect(client: &Client, options: RequestOptions) -> Result<Self> {
            Ok(Self {
                inner: connect_socket(client, "responses.ws.connect", "/responses", options)
                    .await?,
            })
        }

        /// 返回当前连接的 URL。
        pub fn url(&self) -> &Url {
            &self.inner.url
        }

        /// 返回一个可迭代的事件流。
        pub fn stream(&self) -> BoxStream<'static, ResponsesStreamMessage> {
            self.inner.stream()
        }

        /// 发送一个可序列化事件。
        ///
        /// # Errors
        ///
        /// 当序列化失败或发送失败时返回错误。
        pub async fn send_json<T>(&self, event: &T) -> Result<()>
        where
            T: Serialize,
        {
            send_json(&self.inner, event).await
        }

        /// 主动关闭连接。
        ///
        /// # Errors
        ///
        /// 当发送 close frame 失败时返回错误。
        pub async fn close(&self, options: SocketCloseOptions) -> Result<()> {
            close_socket(&self.inner, options).await
        }
    }

    async fn connect_socket<T>(
        client: &Client,
        endpoint_id: &'static str,
        path: &str,
        options: RequestOptions,
    ) -> Result<Arc<SocketCore<T>>>
    where
        T: serde::de::DeserializeOwned + Clone + Send + 'static,
    {
        let context =
            prepare_request_context(&client.inner, endpoint_id, path.into(), None, &options)
                .await?;
        let url = build_websocket_url(client.base_url(), &context.path, &context.query)?;
        let request = build_websocket_request(&url, &context.headers)?;
        let (stream, _) = connect_async(request)
            .await
            .map_err(|error| Error::WebSocket(WebSocketError::new(error.to_string())))?;

        let (sink, mut source) = stream.split();
        let (sender, _) = broadcast::channel(128);
        let inner = Arc::new(SocketCore {
            url,
            state: AtomicU8::new(ConnectionState::Open.as_u8()),
            events: sender,
            sink: Mutex::new(sink),
        });
        let reader_inner = inner.clone();

        tokio::spawn(async move {
            while let Some(message) = source.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        handle_server_payload::<T>(&reader_inner, text.as_bytes());
                    }
                    Ok(Message::Binary(bytes)) => {
                        handle_server_payload::<T>(&reader_inner, bytes.as_ref());
                    }
                    Ok(Message::Close(_)) => {
                        mark_closed(&reader_inner);
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                    Ok(_) => {}
                    Err(error) => {
                        push_error(&reader_inner, error.to_string());
                        mark_closed(&reader_inner);
                        break;
                    }
                }
            }

            if ConnectionState::from_u8(reader_inner.state.load(Ordering::SeqCst))
                != ConnectionState::Closed
            {
                mark_closed(&reader_inner);
            }
        });

        Ok(inner)
    }

    fn handle_server_payload<T>(inner: &Arc<SocketCore<T>>, payload: &[u8])
    where
        T: serde::de::DeserializeOwned + Clone + Send + 'static,
    {
        match serde_json::from_slice::<T>(payload) {
            Ok(event) => {
                let raw = serde_json::from_slice::<super::WebSocketServerEvent>(payload).ok();
                if raw
                    .as_ref()
                    .is_some_and(super::WebSocketServerEvent::is_error)
                {
                    let message = raw
                        .as_ref()
                        .and_then(super::WebSocketServerEvent::error_message)
                        .unwrap_or_else(|| "WebSocket 收到错误事件".into());
                    push_error(inner, message);
                } else {
                    let _ = inner.events.send(SocketStreamMessage::Message(event));
                }
            }
            Err(error) => {
                let error = Error::Serialization(SerializationError::new(format!(
                    "WebSocket 事件反序列化失败: {error}"
                )));
                push_error(inner, error.to_string());
            }
        }
    }

    fn push_error<T>(inner: &Arc<SocketCore<T>>, message: String)
    where
        T: Clone + Send + 'static,
    {
        let _ = inner
            .events
            .send(SocketStreamMessage::Error(WebSocketError::new(message)));
    }

    fn mark_closed<T>(inner: &Arc<SocketCore<T>>)
    where
        T: Clone + Send + 'static,
    {
        inner
            .state
            .store(ConnectionState::Closed.as_u8(), Ordering::SeqCst);
        let _ = inner.events.send(SocketStreamMessage::Close);
    }

    async fn send_json<T, U>(inner: &Arc<SocketCore<T>>, event: &U) -> Result<()>
    where
        T: Clone + Send + 'static,
        U: Serialize,
    {
        let payload = serde_json::to_string(event)
            .map_err(|error| Error::Serialization(SerializationError::new(error.to_string())))?;
        let mut sink = inner.sink.lock().await;
        sink.send(Message::Text(payload.into()))
            .await
            .map_err(|error| Error::WebSocket(WebSocketError::new(error.to_string())))
    }

    async fn close_socket<T>(inner: &Arc<SocketCore<T>>, options: SocketCloseOptions) -> Result<()>
    where
        T: Clone + Send + 'static,
    {
        inner
            .state
            .store(ConnectionState::Closing.as_u8(), Ordering::SeqCst);
        let _ = inner.events.send(SocketStreamMessage::Closing);

        let mut sink = inner.sink.lock().await;
        sink.send(Message::Close(Some(CloseFrame {
            code: CloseCode::from(options.code),
            reason: Utf8Bytes::from(options.reason),
        })))
        .await
        .map_err(|error| Error::WebSocket(WebSocketError::new(error.to_string())))?;
        Ok(())
    }

    fn build_websocket_url(
        base_url: &str,
        path: &str,
        query: &BTreeMap<String, String>,
    ) -> Result<Url> {
        let joined = join_url(base_url, path)?;
        let mut url = Url::parse(&joined)
            .map_err(|error| Error::InvalidConfig(format!("WebSocket URL 无效: {error}")))?;
        match url.scheme() {
            "http" => {
                let _ = url.set_scheme("ws");
            }
            "https" => {
                let _ = url.set_scheme("wss");
            }
            "ws" | "wss" => {}
            scheme => {
                return Err(Error::InvalidConfig(format!(
                    "不支持的 WebSocket 基础协议: {scheme}"
                )));
            }
        }

        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            pairs.clear();
            for (key, value) in query {
                pairs.append_pair(key, value);
            }
        }
        Ok(url)
    }

    fn build_websocket_request(
        url: &Url,
        headers: &BTreeMap<String, String>,
    ) -> Result<http::Request<()>> {
        let mut request = url.as_str().into_client_request().map_err(|error| {
            Error::InvalidConfig(format!("构建 WebSocket 握手请求失败: {error}"))
        })?;
        for (key, value) in headers {
            request.headers_mut().insert(
                http::header::HeaderName::from_bytes(key.as_bytes()).map_err(|error| {
                    Error::InvalidConfig(format!("构建 WebSocket 握手请求失败: {error}"))
                })?,
                http::header::HeaderValue::from_str(value).map_err(|error| {
                    Error::InvalidConfig(format!("构建 WebSocket 握手请求失败: {error}"))
                })?,
            );
        }
        Ok(request)
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;

        use super::*;

        #[test]
        fn test_should_build_ws_url_from_https_base_url() {
            let url = build_websocket_url(
                "https://api.openai.com/v1",
                "/realtime",
                &BTreeMap::from([("model".into(), "gpt-4o-realtime-preview".into())]),
            )
            .unwrap();

            assert_eq!(
                url.as_str(),
                "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview"
            );
        }

        #[test]
        fn test_should_parse_error_message_from_event() {
            let event = super::super::WebSocketServerEvent {
                event_type: "error".into(),
                data: BTreeMap::from([(
                    "error".into(),
                    serde_json::json!({
                        "message": "bad request"
                    }),
                )]),
            };

            assert_eq!(event.error_message().as_deref(), Some("bad request"));
        }
    }
}

#[cfg(not(any(feature = "realtime", feature = "responses-ws")))]
mod enabled {
    use futures_util::stream::{self, BoxStream};
    use serde::Serialize;

    use super::{
        Client, Error, RealtimeStreamMessage, RequestOptions, ResponsesStreamMessage, Result,
        SocketCloseOptions, Url,
    };

    /// 表示 Realtime WebSocket 连接句柄。
    #[derive(Debug, Clone)]
    pub struct RealtimeSocket {
        url: Url,
    }

    /// 表示 Responses WebSocket 连接句柄。
    #[derive(Debug, Clone)]
    pub struct ResponsesSocket {
        url: Url,
    }

    impl RealtimeSocket {
        /// 建立 Realtime WebSocket 连接。
        pub(crate) async fn connect(
            _client: &Client,
            _model: Option<String>,
            _options: RequestOptions,
        ) -> Result<Self> {
            Err(Error::InvalidConfig(
                "当前未启用 WebSocket 支持，请开启 `realtime` 或 `responses-ws` feature".into(),
            ))
        }

        /// 返回当前连接的 URL。
        pub fn url(&self) -> &Url {
            &self.url
        }

        /// 返回一个空事件流。
        pub fn stream(&self) -> BoxStream<'static, RealtimeStreamMessage> {
            Box::pin(stream::empty())
        }

        /// 发送一个可序列化事件。
        pub async fn send_json<T>(&self, _event: &T) -> Result<()>
        where
            T: Serialize,
        {
            Err(Error::InvalidConfig(
                "当前未启用 WebSocket 支持，请开启 `realtime` 或 `responses-ws` feature".into(),
            ))
        }

        /// 主动关闭连接。
        pub async fn close(&self, _options: SocketCloseOptions) -> Result<()> {
            Ok(())
        }
    }

    impl ResponsesSocket {
        /// 建立 Responses WebSocket 连接。
        pub(crate) async fn connect(_client: &Client, _options: RequestOptions) -> Result<Self> {
            Err(Error::InvalidConfig(
                "当前未启用 WebSocket 支持，请开启 `realtime` 或 `responses-ws` feature".into(),
            ))
        }

        /// 返回当前连接的 URL。
        pub fn url(&self) -> &Url {
            &self.url
        }

        /// 返回一个空事件流。
        pub fn stream(&self) -> BoxStream<'static, ResponsesStreamMessage> {
            Box::pin(stream::empty())
        }

        /// 发送一个可序列化事件。
        pub async fn send_json<T>(&self, _event: &T) -> Result<()>
        where
            T: Serialize,
        {
            Err(Error::InvalidConfig(
                "当前未启用 WebSocket 支持，请开启 `realtime` 或 `responses-ws` feature".into(),
            ))
        }

        /// 主动关闭连接。
        pub async fn close(&self, _options: SocketCloseOptions) -> Result<()> {
            Ok(())
        }
    }
}

pub use enabled::{RealtimeSocket, ResponsesSocket};
