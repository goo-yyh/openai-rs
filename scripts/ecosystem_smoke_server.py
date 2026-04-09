#!/usr/bin/env python3
import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


RESPONSES_PAYLOAD = {
    "id": "resp_fixture_1",
    "object": "response",
    "model": "gpt-5.4",
    "status": "completed",
    "output": [
        {
            "id": "msg_fixture_1",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": '{"city":"Shanghai"}'}],
        }
    ],
}

CHAT_COMPLETION_PAYLOAD = {
    "id": "chatcmpl_fixture_1",
    "object": "chat.completion",
    "created": 1,
    "model": "gpt-5.4",
    "choices": [
        {
            "index": 0,
            "finish_reason": "stop",
            "message": {
                "role": "assistant",
                "content": "fixture assistant reply",
                "tool_calls": [],
                "reasoning_details": [{"summary": "fixture"}],
            },
            "logprobs": {
                "content": [
                    {
                        "token": "fixture",
                        "bytes": [102, 105, 120, 116, 117, 114, 101],
                        "logprob": -0.1,
                        "top_logprobs": [
                            {
                                "token": "fixture",
                                "bytes": [102, 105, 120, 116, 117, 114, 101],
                                "logprob": -0.1,
                            }
                        ],
                    }
                ]
            },
        }
    ],
    "usage": {
        "prompt_tokens": 3,
        "completion_tokens": 2,
        "total_tokens": 5,
        "prompt_tokens_details": {"cached_tokens": 1},
        "completion_tokens_details": {"reasoning_tokens": 1},
    },
}

REALTIME_SECRET_PAYLOAD = {
    "value": "ek_fixture_123",
    "expires_at": 1_900_000_000,
    "session": {
        "type": "realtime",
        "model": "gpt-realtime",
    },
}

RESPONSES_STREAM_BODY = (
    "event: response.created\n"
    'data: {"type":"response.created","response":{"id":"resp_stream_fixture_1","object":"response","model":"gpt-5.4","status":"in_progress","output":[]}}\n\n'
    "event: response.output_item.added\n"
    'data: {"type":"response.output_item.added","output_index":0,"item":{"id":"msg_stream_fixture_1","type":"message","role":"assistant","content":[]}}\n\n'
    "event: response.content_part.added\n"
    'data: {"type":"response.content_part.added","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}\n\n'
    "event: response.output_text.delta\n"
    'data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"stream fixture"}\n\n'
    "event: response.completed\n"
    'data: {"type":"response.completed","response":{"id":"resp_stream_fixture_1","object":"response","model":"gpt-5.4","status":"completed","output":[{"id":"msg_stream_fixture_1","type":"message","role":"assistant","content":[{"type":"output_text","text":"stream fixture"}]}]}}\n\n'
    "data: [DONE]\n\n"
)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, format, *args):  # noqa: A003
        return

    def _consume_body(self) -> bytes:
        length = int(self.headers.get("Content-Length", "0"))
        if length:
            return self.rfile.read(length)
        return b""

    def _send_json(self, status: int, payload: dict) -> None:
        data = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _send_sse(self, body: str) -> None:
        data = body.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self) -> None:  # noqa: N802
        body = self._consume_body()
        if self.path == "/v1/chat/completions":
            self._send_json(200, CHAT_COMPLETION_PAYLOAD)
            return
        if self.path == "/v1/responses":
            payload = json.loads(body.decode("utf-8")) if body else {}
            if payload.get("stream") is True:
                self._send_sse(RESPONSES_STREAM_BODY)
                return
            self._send_json(200, RESPONSES_PAYLOAD)
            return
        if self.path == "/v1/realtime/client_secrets":
            self._send_json(200, REALTIME_SECRET_PAYLOAD)
            return
        self._send_json(
            404,
            {
                "error": {
                    "message": f"unknown fixture route: {self.path}",
                    "type": "fixture_error",
                }
            },
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Local HTTP stub for ecosystem smoke fixtures"
    )
    parser.add_argument("--port", type=int, default=4010)
    args = parser.parse_args()

    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print(f"ecosystem smoke server listening on 127.0.0.1:{args.port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
