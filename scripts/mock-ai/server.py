#!/usr/bin/env python3
"""Fake OpenAI API for testing HeyFlitty end to end without a real key.

Speaks just enough of the chat, transcription and models endpoints:
streams a canned reply that ends with a point tag, and records each request
summary to last-request.json so tests can assert what HeyFlitty sent.

    python3 scripts/mock-ai/server.py --host 127.0.0.1 --port 8766
"""
import argparse
import json
import pathlib
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

REPLY = "see the search box up top? click it and type what you need. [POINT:640,40:search box]"
TRANSCRIPT = "where do I search on this page"
RECORD = pathlib.Path(__file__).with_name("last-request.json")


def summarize(body: dict, api: str) -> dict:
    messages = body.get("messages", [])
    last = messages[-1]["content"] if messages else []
    parts = last if isinstance(last, list) else [{"type": "text", "text": last}]
    images = [part for part in parts if part.get("type") in ("image", "image_url")]
    texts = [part.get("text", "") for part in parts if part.get("type") == "text"]
    return {
        "api": api,
        "model": body.get("model"),
        "stream": body.get("stream"),
        "has_system": any(m.get("role") == "system" for m in messages),
        "history_messages": len(messages) - 2,
        "image_count": len(images),
        "texts": texts,
        "auth": "present" if body.get("_auth") else "absent",
    }


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        print(f"[mock-ai] {self.command} {self.path}")

    def _sse(self, events):
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        for event in events:
            self.wfile.write(f"data: {json.dumps(event)}\n\n".encode())
            self.wfile.flush()
            time.sleep(0.05)

    def do_GET(self):
        ok = self.path.endswith("/models")
        self.send_response(200 if ok else 404)
        self.send_header("content-type", "application/json")
        self.end_headers()
        if ok:
            self.wfile.write(b'{"data": []}')

    def do_POST(self):
        length = int(self.headers.get("content-length", 0))
        raw = self.rfile.read(length)
        auth = self.headers.get("authorization") or self.headers.get("x-api-key")
        if self.path.endswith("/audio/transcriptions"):
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"text": TRANSCRIPT}).encode())
            return
        body = json.loads(raw or b"{}")
        body["_auth"] = auth
        chunks = [REPLY[i : i + 12] for i in range(0, len(REPLY), 12)]
        if self.path.endswith("/chat/completions"):
            RECORD.write_text(json.dumps(summarize(body, "openai"), indent=2))
            self._sse([{"choices": [{"delta": {"content": chunk}}]} for chunk in chunks])
            self.wfile.write(b"data: [DONE]\n\n")
        else:
            self.send_response(404)
            self.end_headers()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8766)
    args = parser.parse_args()
    ThreadingHTTPServer((args.host, args.port), Handler).serve_forever()
