//! WebSocket transport and connection management internals.

use futures_util::stream::BoxStream;
use url::Url;

use crate::Client;
use crate::config::RequestOptions;
use crate::error::{Error, Result, SerializationError, WebSocketError};

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
    use tracing::{debug, error, info, warn};

    use super::{
        BoxStream, Client, Error, RequestOptions, Result, SerializationError, Url, WebSocketError,
    };
    use crate::config::{LogLevel, LogRecord, LoggerHandle};
    #[cfg(feature = "realtime")]
    use crate::providers::ProviderKind;
    use crate::transport::{join_url, prepare_request_context};
    #[cfg(feature = "realtime")]
    use crate::websocket::{RealtimeServerEvent, RealtimeStreamMessage};
    #[cfg(feature = "responses-ws")]
    use crate::websocket::{ResponsesServerEvent, ResponsesStreamMessage};
    use crate::websocket::{SocketCloseOptions, SocketStreamMessage, WebSocketServerEvent};

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
        log_level: LogLevel,
        logger: Option<LoggerHandle>,
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
    #[cfg(feature = "realtime")]
    #[derive(Debug, Clone)]
    pub struct RealtimeSocket {
        inner: Arc<SocketCore<RealtimeServerEvent>>,
    }

    /// 表示 Responses WebSocket 连接句柄。
    #[cfg(feature = "responses-ws")]
    #[derive(Debug, Clone)]
    pub struct ResponsesSocket {
        inner: Arc<SocketCore<ResponsesServerEvent>>,
    }

    #[cfg(feature = "realtime")]
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
                        return Err(Error::MissingRequiredField {
                            field: "deployment",
                        });
                    }
                    Ok(Self { inner: socket })
                }
                _ => {
                    let Some(model) = model else {
                        return Err(Error::MissingRequiredField { field: "model" });
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
        pub async fn send_json<T>(&self, event: &T) -> Result<()>
        where
            T: Serialize,
        {
            send_json(&self.inner, event).await
        }

        /// 主动关闭连接。
        pub async fn close(&self, options: SocketCloseOptions) -> Result<()> {
            close_socket(&self.inner, options).await
        }
    }

    #[cfg(feature = "responses-ws")]
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
        pub async fn send_json<T>(&self, event: &T) -> Result<()>
        where
            T: Serialize,
        {
            send_json(&self.inner, event).await
        }

        /// 主动关闭连接。
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
        emit_socket_log(
            client.inner.options.log_level,
            client.inner.options.logger.clone(),
            LogLevel::Debug,
            "openai_core::websocket",
            "建立 WebSocket 连接",
            BTreeMap::from([
                ("endpoint_id".into(), endpoint_id.to_string()),
                ("url".into(), url.to_string()),
            ]),
        );
        let request = build_websocket_request(&url, &context.headers)?;
        let (stream, _) = connect_async(request)
            .await
            .map_err(|error| Error::WebSocket(WebSocketError::transport(error.to_string())))?;

        let (sink, mut source) = stream.split();
        let (sender, _) = broadcast::channel(128);
        let inner = Arc::new(SocketCore {
            url,
            state: AtomicU8::new(ConnectionState::Open.as_u8()),
            events: sender,
            sink: Mutex::new(sink),
            log_level: client.inner.options.log_level,
            logger: client.inner.options.logger.clone(),
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
                    Ok(Message::Close(frame)) => {
                        handle_close_frame(&reader_inner, frame);
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                    Ok(_) => {}
                    Err(error) => {
                        push_error(&reader_inner, WebSocketError::transport(error.to_string()));
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
        let raw = match serde_json::from_slice::<WebSocketServerEvent>(payload) {
            Ok(raw) => raw,
            Err(error) => {
                let error = Error::Serialization(SerializationError::new(format!(
                    "WebSocket 事件反序列化失败: {error}"
                )));
                push_error(inner, WebSocketError::protocol(error.to_string()));
                return;
            }
        };

        if raw.is_error() {
            let message = raw
                .error_message()
                .unwrap_or_else(|| "WebSocket 收到错误事件".into());
            emit_socket_log(
                inner.log_level,
                inner.logger.clone(),
                LogLevel::Info,
                "openai_core::websocket",
                "收到 WebSocket 错误事件",
                BTreeMap::from([("event_type".into(), raw.event_type.clone())]),
            );
            push_error(
                inner,
                WebSocketError::server(message, Some(raw.event_type.clone())),
            );
            return;
        }

        match serde_json::from_slice::<T>(payload) {
            Ok(event) => {
                emit_socket_log(
                    inner.log_level,
                    inner.logger.clone(),
                    LogLevel::Debug,
                    "openai_core::websocket",
                    "收到 WebSocket 事件",
                    BTreeMap::from([("event_type".into(), raw.event_type.clone())]),
                );
                let _ = inner.events.send(SocketStreamMessage::Message(event));
            }
            Err(error) => {
                let error = Error::Serialization(SerializationError::new(format!(
                    "WebSocket 事件反序列化失败: {error}"
                )));
                push_error(inner, WebSocketError::protocol(error.to_string()));
            }
        }
    }

    fn push_error<T>(inner: &Arc<SocketCore<T>>, error: WebSocketError)
    where
        T: Clone + Send + 'static,
    {
        let _ = inner.events.send(SocketStreamMessage::Error(error));
    }

    fn handle_close_frame<T>(inner: &Arc<SocketCore<T>>, frame: Option<CloseFrame>)
    where
        T: Clone + Send + 'static,
    {
        let state = ConnectionState::from_u8(inner.state.load(Ordering::SeqCst));
        if state != ConnectionState::Closing
            && let Some(frame) = frame.as_ref()
            && let Some(error) = map_close_frame_error(frame)
        {
            push_error(inner, error);
        }
        mark_closed(inner);
    }

    fn map_close_frame_error(frame: &CloseFrame) -> Option<WebSocketError> {
        if frame.code == CloseCode::Normal {
            return None;
        }

        let code = u16::from(frame.code);
        let reason = frame.reason.to_string();
        let message = if reason.is_empty() {
            format!("WebSocket 连接被关闭: code={code}")
        } else {
            format!("WebSocket 连接被关闭: code={code}, reason={reason}")
        };
        Some(WebSocketError::protocol(message))
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
        emit_socket_log(
            inner.log_level,
            inner.logger.clone(),
            LogLevel::Debug,
            "openai_core::websocket",
            "发送 WebSocket 消息",
            BTreeMap::from([("url".into(), inner.url.to_string())]),
        );
        let mut sink = inner.sink.lock().await;
        sink.send(Message::Text(payload.into()))
            .await
            .map_err(|error| Error::WebSocket(WebSocketError::transport(error.to_string())))
    }

    async fn close_socket<T>(inner: &Arc<SocketCore<T>>, options: SocketCloseOptions) -> Result<()>
    where
        T: Clone + Send + 'static,
    {
        inner
            .state
            .store(ConnectionState::Closing.as_u8(), Ordering::SeqCst);
        let _ = inner.events.send(SocketStreamMessage::Closing);
        emit_socket_log(
            inner.log_level,
            inner.logger.clone(),
            LogLevel::Info,
            "openai_core::websocket",
            "关闭 WebSocket 连接",
            BTreeMap::from([
                ("url".into(), inner.url.to_string()),
                ("code".into(), options.code.to_string()),
            ]),
        );

        let mut sink = inner.sink.lock().await;
        sink.send(Message::Close(Some(CloseFrame {
            code: CloseCode::from(options.code),
            reason: Utf8Bytes::from(options.reason),
        })))
        .await
        .map_err(|error| Error::WebSocket(WebSocketError::transport(error.to_string())))?;
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

    fn emit_socket_log(
        configured_level: LogLevel,
        logger: Option<LoggerHandle>,
        level: LogLevel,
        target: &'static str,
        message: impl Into<String>,
        fields: BTreeMap<String, String>,
    ) {
        if !configured_level.allows(level) {
            return;
        }

        let record = LogRecord {
            level,
            target,
            message: message.into(),
            fields,
        };
        if let Some(logger) = &logger {
            logger.log(&record);
        }

        let rendered_fields = if record.fields.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                record
                    .fields
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        let rendered = format!("[{}] {}{}", target, record.message, rendered_fields);
        match level {
            LogLevel::Off => {}
            LogLevel::Error => error!("{rendered}"),
            LogLevel::Warn => warn!("{rendered}"),
            LogLevel::Info => info!("{rendered}"),
            LogLevel::Debug => debug!("{rendered}"),
        }
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
        use crate::error::WebSocketErrorKind;

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
        fn test_should_reject_unsupported_websocket_base_scheme() {
            let error = build_websocket_url("ftp://example.com", "/realtime", &BTreeMap::new())
                .unwrap_err();

            assert!(matches!(error, Error::InvalidConfig(_)));
            assert!(error.to_string().contains("ftp"));
        }

        #[test]
        fn test_should_reject_invalid_websocket_headers() {
            let error = build_websocket_request(
                &Url::parse("ws://example.com/realtime").unwrap(),
                &BTreeMap::from([("x-test".into(), "bad\nvalue".into())]),
            )
            .unwrap_err();

            assert!(matches!(error, Error::InvalidConfig(_)));
        }

        #[test]
        fn test_should_parse_error_message_from_event() {
            let event = WebSocketServerEvent {
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

        #[test]
        fn test_should_map_abnormal_close_frame_to_protocol_error() {
            let error = map_close_frame_error(&CloseFrame {
                code: CloseCode::from(1008),
                reason: Utf8Bytes::from("quota exceeded"),
            })
            .unwrap();

            assert_eq!(error.kind, WebSocketErrorKind::Protocol);
            assert!(error.message.contains("1008"));
            assert!(error.message.contains("quota exceeded"));
        }

        #[test]
        fn test_should_ignore_normal_close_frame_for_error_mapping() {
            let error = map_close_frame_error(&CloseFrame {
                code: CloseCode::Normal,
                reason: Utf8Bytes::from("OK"),
            });

            assert!(error.is_none());
        }
    }
}

#[cfg(not(any(feature = "realtime", feature = "responses-ws")))]
mod enabled {
    use futures_util::stream::{self, BoxStream};
    use serde::Serialize;

    use super::{Client, Error, RequestOptions, Result, Url};
    use crate::websocket::{RealtimeStreamMessage, ResponsesStreamMessage, SocketCloseOptions};

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

#[cfg(feature = "realtime")]
pub use enabled::RealtimeSocket;
#[cfg(feature = "responses-ws")]
pub use enabled::ResponsesSocket;
