"""End-to-end smoke test: stand up the proxy with a fake upstream and verify
fail-closed semantics, hash chain integrity, and ledger format."""

from __future__ import annotations

import json
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import httpx
import pytest

from harness_proxy.jcs import canonicalize
from harness_proxy.ledger import (
    GENESIS_PREV,
    LedgerError,
    SessionLedger,
    hash_entry,
    hash_input,
)


# --- JCS ----------------------------------------------------------------------

def test_jcs_orders_keys():
    assert canonicalize({"b": 1, "a": 2}) == b'{"a":2,"b":1}'


def test_jcs_escapes_control_chars():
    assert canonicalize("\x01") == b'"\\u0001"'


def test_jcs_handles_nested():
    out = canonicalize({"x": [1, {"b": True, "a": None}]})
    assert out == b'{"x":[1,{"a":null,"b":true}]}'


# --- Ledger -------------------------------------------------------------------

def test_first_entry_uses_genesis_prev(tmp_path: Path):
    led = SessionLedger(tmp_path, "01TESTSIDAAAAAAAAAAAAAAAAA")
    try:
        e = led.append(model="m", in_hash="sha256:x", reason="hi", act=None)
    finally:
        led.close()
    assert e["seq"] == 0
    assert e["prev"] == GENESIS_PREV


def test_hash_chain_links_entries(tmp_path: Path):
    led = SessionLedger(tmp_path, "01TESTSIDBBBBBBBBBBBBBBBBB")
    try:
        e0 = led.append(model="m", in_hash="sha256:x", reason="r0", act=None)
        e1 = led.append(model="m", in_hash="sha256:x", reason="r1", act=None)
    finally:
        led.close()
    assert e1["prev"] == hash_entry(e0)
    assert e1["seq"] == 1


def test_open_continuation_rejects_action(tmp_path: Path):
    led = SessionLedger(tmp_path, "01TESTSIDCCCCCCCCCCCCCCCCC")
    try:
        with pytest.raises(LedgerError, match="§7.3"):
            led.append(
                model="m",
                in_hash="sha256:x",
                reason="r",
                act={"tool_calls": []},
                cont="open",
            )
    finally:
        led.close()


def test_resume_session_recovers_seq_and_prev(tmp_path: Path):
    sid = "01TESTSIDDDDDDDDDDDDDDDDDD"
    led = SessionLedger(tmp_path, sid)
    e0 = led.append(model="m", in_hash="sha256:x", reason="r0", act=None)
    led.close()

    led2 = SessionLedger(tmp_path, sid)
    try:
        e1 = led2.append(model="m", in_hash="sha256:x", reason="r1", act=None)
    finally:
        led2.close()
    assert e1["seq"] == 1
    assert e1["prev"] == hash_entry(e0)


def test_torn_final_line_is_truncated(tmp_path: Path):
    sid = "01TESTSIDEEEEEEEEEEEEEEEEE"
    led = SessionLedger(tmp_path, sid)
    led.append(model="m", in_hash="sha256:x", reason="r0", act=None)
    led.close()

    # Append a torn line directly.
    path = tmp_path / "sessions" / f"{sid}.jsonl"
    with open(path, "ab") as f:
        f.write(b'{"v":1,"seq":1,"sid"')

    led2 = SessionLedger(tmp_path, sid)
    try:
        e = led2.append(model="m", in_hash="sha256:x", reason="r1", act=None)
    finally:
        led2.close()
    assert e["seq"] == 1


def test_input_hash_is_deterministic():
    h1 = hash_input("sys", [{"role": "user", "content": "hi"}], None)
    h2 = hash_input("sys", [{"role": "user", "content": "hi"}], [])
    assert h1 == h2  # tools=None and tools=[] normalize the same way


# --- End-to-end proxy ---------------------------------------------------------

class _FakeUpstream(BaseHTTPRequestHandler):
    def do_POST(self):  # noqa: N802
        length = int(self.headers.get("content-length", "0"))
        _ = self.rfile.read(length)
        body = json.dumps({
            "id": "chatcmpl-fake",
            "object": "chat.completion",
            "model": "fake-model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hello back"},
                "finish_reason": "stop",
            }],
        }).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args, **kwargs):  # silence
        pass


@pytest.fixture
def upstream():
    server = HTTPServer(("127.0.0.1", 0), _FakeUpstream)
    port = server.server_address[1]
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    try:
        yield f"http://127.0.0.1:{port}"
    finally:
        server.shutdown()


def test_end_to_end_proxy_writes_ledger(tmp_path: Path, upstream: str, monkeypatch):
    monkeypatch.setenv("UPSTREAM_BASE_URL", upstream)
    monkeypatch.setenv("HARNESS_ROOT", str(tmp_path))

    # Re-import with fresh env.
    import importlib
    import harness_proxy.proxy as proxy_mod
    importlib.reload(proxy_mod)

    from fastapi.testclient import TestClient
    with TestClient(proxy_mod.app) as client:
        r = client.post(
            "/v1/chat/completions",
            json={"model": "fake-model", "messages": [{"role": "user", "content": "hi"}]},
        )
    assert r.status_code == 200
    sid = r.headers["x-harness-session"]
    assert r.headers["x-harness-seq"] == "0"

    ledger_file = tmp_path / "sessions" / f"{sid}.jsonl"
    assert ledger_file.exists()
    line = ledger_file.read_text().strip()
    entry = json.loads(line)
    assert entry["seq"] == 0
    assert entry["sid"] == sid
    assert entry["model"] == "fake-model"
    assert entry["reason"] == "hello back"
    assert entry["act"] is None
    assert entry["prev"] == GENESIS_PREV
