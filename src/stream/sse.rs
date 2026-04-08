use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};

use crate::error::{Result, SerializationError, StreamError};
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
                    if index + 1 < self.buffer.len() {
                        if self.buffer[index + 1] == b'\n' {
                            index += 1;
                            lines.push(bytes_to_string(&self.buffer[start..end])?);
                            start = index + 1;
                        } else {
                            lines.push(bytes_to_string(&self.buffer[start..end])?);
                            start = index + 1;
                        }
                    } else {
                        break;
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

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    use super::LineDecoder;

    #[derive(Debug, Clone, Copy)]
    enum Separator {
        Lf,
        Cr,
        CrLf,
    }

    impl Separator {
        fn as_str(self) -> &'static str {
            match self {
                Self::Lf => "\n",
                Self::Cr => "\r",
                Self::CrLf => "\r\n",
            }
        }
    }

    fn separator_strategy() -> impl Strategy<Value = Separator> {
        prop_oneof![
            Just(Separator::Lf),
            Just(Separator::Cr),
            Just(Separator::CrLf),
        ]
    }

    proptest! {
        #[test]
        fn line_decoder_preserves_lines_across_arbitrary_chunking(
            lines in prop::collection::vec("[^\r\n]{0,16}", 1..8),
            separator in separator_strategy(),
            chunk_sizes in prop::collection::vec(1usize..8, 1..32),
        ) {
            let mut payload = String::new();
            for line in lines.iter() {
                payload.push_str(line);
                payload.push_str(separator.as_str());
            }

            let mut decoder = LineDecoder::default();
            let mut decoded = Vec::new();
            let bytes = payload.as_bytes();
            let mut offset = 0usize;

            for chunk_size in chunk_sizes {
                if offset >= bytes.len() {
                    break;
                }
                let end = (offset + chunk_size).min(bytes.len());
                decoded.extend(decoder.push(&bytes[offset..end]).unwrap());
                offset = end;
            }

            if offset < bytes.len() {
                decoded.extend(decoder.push(&bytes[offset..]).unwrap());
            }

            if let Some(tail) = decoder.finish().unwrap() {
                decoded.push(tail);
            }
            prop_assert_eq!(decoded, lines);
        }
    }

    #[test]
    fn line_decoder_flushes_final_partial_line() {
        let mut decoder = LineDecoder::default();
        assert!(decoder.push(b"event: response.created").unwrap().is_empty());
        assert_eq!(
            decoder.finish().unwrap(),
            Some("event: response.created".into())
        );
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

#[cfg(test)]
mod tests {
    use super::{LineDecoder, PendingSseEvent};

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

    #[test]
    fn test_should_preserve_crlf_split_across_chunks() {
        let mut decoder = LineDecoder::default();
        assert_eq!(decoder.push(b"data: one\r").unwrap(), Vec::<String>::new());
        assert_eq!(decoder.push(b"\n").unwrap(), vec!["data: one".to_string()]);
        assert_eq!(decoder.finish().unwrap(), None);
    }

    #[test]
    fn test_should_parse_empty_and_multiline_sse_data_fields() {
        let mut pending = PendingSseEvent::default();
        assert_eq!(pending.push_line("event: message").unwrap(), None);
        assert_eq!(pending.push_line("data:").unwrap(), None);
        assert_eq!(pending.push_line("data: hello").unwrap(), None);

        let event = pending.push_line("").unwrap().unwrap();
        assert_eq!(event.event.as_deref(), Some("message"));
        assert_eq!(event.data, "\nhello");
    }
}
