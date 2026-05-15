# Harness Protocol

A transparent MITM proxy that writes a tamper-evident, hash-chained ledger of every LLM interaction — before the response is released to the caller.

The proxy enforces **Architectural Constraint**: an LLM response cannot reach the client unless its reasoning and actions have been durably persisted. This is a structural guarantee, not a behavioral one.

The ledger format is specified in [SPEC.md](./SPEC.md).

---

## Quickstart

**1. Download the binary** from the latest [CI build](../../actions/workflows/build-proxy.yml):
- `harness-proxy-windows` — Windows x86_64
- `harness-proxy-linux` — Linux x86_64

**2. Run the proxy:**

```sh
# Sessions land in .harness/sessions/ relative to cwd by default
HARNESS_ROOT=/path/to/harness ./harness-proxy
```

The proxy listens on `127.0.0.1:8080` by default (`HARNESS_LISTEN` to override).

**3. Point your LLM client at the proxy:**

```python
# OpenAI / Grok
client = OpenAI(base_url="http://127.0.0.1:8080/v1", api_key="...")

# Anthropic
client = Anthropic(base_url="http://127.0.0.1:8080", api_key="...")
```

**4. Each call produces a session file:**

```jsonc
// .harness/sessions/<ulid>.jsonl — one line per turn
{
  "v": 1, "seq": 0, "sid": "01KRNDE2C2DBE9AWNYPXKGSD7M",
  "ts": "2026-05-15T08:53:17.241Z", "model": "claude-haiku-4-5",
  "in":   "sha256:faf7bc...",   // SHA-256 of the canonicalized request
  "prev": "sha256:000000...",   // genesis; subsequent turns chain here
  "think": null,                // extended reasoning tokens (if any)
  "reason": "",                 // model text output
  "act": {                      // tool call (if any)
    "name": "record_result",
    "input": { "status": "harness-act-verified" }
  },
  "transparency": { "think": false, "act": true }
}
```

---

## Supported providers

| Provider | Endpoint | Notes |
|---|---|---|
| OpenAI / Grok | `/v1/chat/completions` | Streaming + buffered. `UPSTREAM_BASE_URL` to override (default: `https://api.openai.com`). |
| Anthropic | `/v1/messages` | Streaming + buffered. `ANTHROPIC_BASE_URL` to override (default: `https://api.anthropic.com`). |
| Gemini | `/v1beta/models/*` | Streaming + buffered. `GEMINI_BASE_URL` to override (default: `https://generativelanguage.googleapis.com`). |

The proxy is a dumb pipe: all request headers (including `Authorization` / `x-api-key`) are forwarded verbatim. No credentials are read or stored.

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `HARNESS_ROOT` | `.harness` | Directory for session files. Created if absent. |
| `HARNESS_LISTEN` | `127.0.0.1:8080` | Address to bind. |
| `UPSTREAM_BASE_URL` | `https://api.openai.com` | Upstream for `/v1/chat/completions`. |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | Upstream for `/v1/messages`. |
| `GEMINI_BASE_URL` | `https://generativelanguage.googleapis.com` | Upstream for `/v1beta/models/*`. |

---

## How it works

```
Client → harness-proxy:8080 → Real LLM API
                ↓
         .harness/sessions/<ulid>.jsonl
         (fsync'd before response forwarded)
```

Each session is one JSONL file named by a ULID (sortable by creation time). Each line is one turn. Consecutive turns are linked by a SHA-256 hash chain over RFC 8785 canonicalized entries — tampering or reordering any entry breaks the chain.

The `transparency` object records whether `think` and `act` carried content, enabling downstream analysis without content inspection.

See [SPEC.md](./SPEC.md) for the full protocol definition (entry format, hash chain, streaming continuations, failure semantics, conformance tiers).

---

## Build from source

Requires Rust stable.

```sh
cd proxy-rust
cargo test    # 15 unit tests: ledger integrity, JCS canonicalization, ULID
cargo build --release
```

CI builds and uploads artifacts for Windows x86_64 and Linux x86_64 on every push to `proxy-rust/`.

---

## Design principles

- **Fail-closed.** If the ledger write fails, the response is withheld. The client sees an error. The LLM cannot act without a record.
- **Dumb pipe.** The proxy does not interpret, filter, or modify content. It captures what the model sent and what the client sent.
- **Zero client integration.** One `base_url` change. No SDK wrapping, no library import.
- **Single binary.** No runtime, no daemon, no configuration file.