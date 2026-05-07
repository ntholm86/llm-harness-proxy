"""Harness Protocol MVP proxy — invisible MITM for OpenAI-compatible APIs.

Run:
    uvicorn harness_proxy.proxy:app --host 127.0.0.1 --port 8080

Configure your client:
    base_url = "http://127.0.0.1:8080/v1"
    UPSTREAM_BASE_URL env var sets the real upstream (default: api.openai.com).
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

import httpx
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import Response

from .ledger import LedgerError, SessionLedger, hash_input
from .ulid import new_ulid

UPSTREAM_BASE = os.environ.get("UPSTREAM_BASE_URL", "https://api.openai.com").rstrip("/")
ANTHROPIC_BASE = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com").rstrip("/")
ROOT = Path(os.environ.get("HARNESS_ROOT", ".harness")).resolve()

app = FastAPI(title="Harness Protocol MVP Proxy")
_client = httpx.AsyncClient(timeout=httpx.Timeout(120.0, connect=10.0))

# Sessions are keyed by an opaque header the client may send. Without it we
# mint one per request — sufficient for MVP, refined when we add VS Code UX.
_SESSION_HEADER = "x-harness-session"


@app.post("/v1/chat/completions")
async def chat_completions(request: Request) -> Response:
    body_bytes = await request.body()
    try:
        payload = json.loads(body_bytes)
    except json.JSONDecodeError:
        raise HTTPException(400, "request body is not valid JSON")

    sid = request.headers.get(_SESSION_HEADER) or new_ulid()
    in_hash = hash_input(
        system=_extract_system(payload),
        messages=payload.get("messages", []),
        tools=payload.get("tools"),
    )

    target_base = request.headers.get("x-harness-upstream", UPSTREAM_BASE).rstrip("/")
    upstream_url = f"{target_base}/v1/chat/completions"
    
    headers = {k: v for k, v in request.headers.items()
               if k.lower() not in {"host", "content-length", _SESSION_HEADER, "x-harness-upstream"}}
    # Adjust host header dynamically
    headers["host"] = target_base.replace("https://", "").replace("http://", "").split("/")[0]
    
    try:
        upstream_res = await _client.post(upstream_url, content=body_bytes, headers=headers)
    except httpx.HTTPError as e:
        raise HTTPException(502, f"upstream request failed: {e}")

    res_bytes = upstream_res.content
    res_headers = dict(upstream_res.headers)
    res_headers.pop("content-encoding", None)  # we already have decoded bytes
    res_headers.pop("transfer-encoding", None)

    # Parse response and write the ledger entry under fail-closed semantics.
    model = payload.get("model", "unknown")
    reason, act = _extract_reason_and_action(res_bytes)

    ledger = SessionLedger(ROOT, sid)
    try:
        try:
            entry = ledger.append(
                model=model,
                in_hash=in_hash,
                reason=reason,
                act=act,
            )
        except LedgerError as e:
            # FAIL-CLOSED: refuse to deliver the upstream response.
            raise HTTPException(500, f"harness ledger write failed: {e}")
    finally:
        ledger.close()

    res_headers["x-harness-session"] = sid
    res_headers["x-harness-seq"] = str(entry["seq"])
    res_headers["x-harness-prev"] = entry["prev"]
    return Response(
        content=res_bytes,
        status_code=upstream_res.status_code,
        headers=res_headers,
        media_type=upstream_res.headers.get("content-type"),
    )


@app.post("/v1/messages")
async def anthropic_messages(request: Request) -> Response:
    body_bytes = await request.body()
    try:
        payload = json.loads(body_bytes)
    except json.JSONDecodeError:
        raise HTTPException(400, "request body is not valid JSON")

    sid = request.headers.get(_SESSION_HEADER) or new_ulid()
    
    sys_prompt = payload.get("system")
    if isinstance(sys_prompt, list):
        sys_prompt = json.dumps(sys_prompt)
        
    in_hash = hash_input(
        system=sys_prompt,
        messages=payload.get("messages", []),
        tools=payload.get("tools"),
    )

    target_base = request.headers.get("x-harness-upstream", ANTHROPIC_BASE).rstrip("/")
    upstream_url = f"{target_base}/v1/messages"

    headers = {k: v for k, v in request.headers.items()
               if k.lower() not in {"host", "content-length", _SESSION_HEADER, "x-harness-upstream"}}
    # Override host to target anthropic 
    headers["host"] = target_base.replace("https://", "").replace("http://", "").split("/")[0]
    
    try:
        upstream_res = await _client.post(upstream_url, content=body_bytes, headers=headers)
    except httpx.HTTPError as e:
        raise HTTPException(502, f"upstream request failed: {e}")

    res_bytes = upstream_res.content
    res_headers = dict(upstream_res.headers)
    res_headers.pop("content-encoding", None)
    res_headers.pop("transfer-encoding", None)

    model = payload.get("model", "unknown")
    reason, act = _extract_anthropic_reason_and_action(res_bytes)

    ledger = SessionLedger(ROOT, sid)
    try:
        try:
            entry = ledger.append(
                model=model,
                in_hash=in_hash,
                reason=reason,
                act=act,
            )
        except LedgerError as e:
            raise HTTPException(500, f"harness ledger write failed: {e}")
    finally:
        ledger.close()

    res_headers["x-harness-session"] = sid
    res_headers["x-harness-seq"] = str(entry["seq"])
    res_headers["x-harness-prev"] = entry["prev"]
    return Response(
        content=res_bytes,
        status_code=upstream_res.status_code,
        headers=res_headers,
        media_type=upstream_res.headers.get("content-type"),
    )


def _extract_system(payload: dict[str, Any]) -> str | None:
    for msg in payload.get("messages", []):
        if msg.get("role") == "system":
            content = msg.get("content")
            return content if isinstance(content, str) else json.dumps(content)
    return None


def _extract_reason_and_action(res_bytes: bytes) -> tuple[str, dict[str, Any] | None]:
    """Best-effort extraction of reasoning text and tool-call action from a
    chat-completions response. MVP: non-streaming only."""
    try:
        body = json.loads(res_bytes)
    except json.JSONDecodeError:
        return "", None
    choice = (body.get("choices") or [{}])[0]
    message = choice.get("message") or {}
    reason = message.get("content") or ""
    tool_calls = message.get("tool_calls") or []
    if tool_calls:
        # Treat the first tool call as the action for MVP. Multi-call support
        # will require one entry per call (each gated separately).
        return reason, {"tool_calls": tool_calls}
    return reason, None


def _extract_anthropic_reason_and_action(res_bytes: bytes) -> tuple[str, dict[str, Any] | None]:
    try:
        body = json.loads(res_bytes)
    except json.JSONDecodeError:
        return "", None
    
    reason = []
    act = None
    for block in body.get("content", []):
        if block.get("type") == "text":
            reason.append(block.get("text", ""))
        elif block.get("type") == "tool_use" and act is None:
            act = block
            
    return "".join(reason), act


@app.get("/healthz")
async def healthz() -> dict[str, str]:
    return {"status": "ok", "upstream": f"openai={UPSTREAM_BASE}, anthropic={ANTHROPIC_BASE}", "root": str(ROOT)}
