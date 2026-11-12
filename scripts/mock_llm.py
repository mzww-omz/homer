#!/usr/bin/env python3
"""A tiny OpenAI-compatible mock server for Homer UI / integration tests.

This is NOT an LLM. It only streams a fixed Japanese reply so the UI,
cancellation, and persistence paths can be exercised without LM Studio.
Start it, then set Homer's endpoint to http://127.0.0.1:18080/v1 and model
to `mock-7b`.
"""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
import sys
import time

RESPONSE = [
    "「……また来たんだ」\n",
    "七海は本から顔を上げて、少しだけ目を細めた。\n",
    "「雨、やんでないね。傘、持ってる？」\n",
    "窓の外では、灰色の空から静かに雨が降り続いていた。",
]

PORT = int(os.environ.get("HOMER_MOCK_PORT", "18080"))


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, fmt, *args):
        sys.stderr.write("mock: " + (fmt % args) + "\n")

    def _json(self, status, payload):
        body = json.dumps(payload, ensure_ascii=False).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.rstrip("/").endswith("/models"):
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {"id": "mock-7b", "object": "model"},
                        {"id": "mock-12b", "object": "model"},
                    ],
                },
            )
        else:
            self._json(404, {"error": "not found"})

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        if not self.path.rstrip("/").endswith("/chat/completions"):
            self._json(404, {"error": "not found"})
            return
        if os.environ.get("HOMER_MOCK_ECHO"):
            sys.stderr.write(body.decode(errors="replace")[:4000] + "\n")

        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()
        try:
            for chunk in RESPONSE:
                event = {"choices": [{"delta": {"content": chunk}, "index": 0}]}
                self.wfile.write(
                    ("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode()
                )
                self.wfile.flush()
                time.sleep(0.35)
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass


if __name__ == "__main__":
    print(f"Homer mock LLM listening on http://127.0.0.1:{PORT}/v1", flush=True)
    try:
        ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
    except KeyboardInterrupt:
        pass
