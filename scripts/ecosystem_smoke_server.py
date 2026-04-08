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

REALTIME_SECRET_PAYLOAD = {
    "client_secret": {
        "expires_at": 1_900_000_000,
        "value": "ek_fixture_123",
    },
    "type": "realtime",
    "session": {
        "model": "gpt-realtime",
    },
}


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, format, *args):  # noqa: A003
        return

    def _consume_body(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        if length:
            self.rfile.read(length)

    def _send_json(self, status: int, payload: dict) -> None:
        data = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self) -> None:  # noqa: N802
        self._consume_body()
        if self.path == "/v1/responses":
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
