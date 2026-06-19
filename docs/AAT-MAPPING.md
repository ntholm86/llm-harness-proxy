# LLM Harness Protocol → IETF Agent Audit Trail Mapping

**Version:** 1.0.0  
**Date:** 2026-06-19  
**Harness Protocol Version:** 1  
**AAT Draft Version:** draft-sharif-agent-audit-trail-00 (March 29, 2026)

---

## Overview

The LLM Harness Protocol implements the core cryptographic guarantees of the IETF Agent Audit Trail (AAT) specification while extending it with:

1. **Enforcement semantics** — fail-closed action gating (actions cannot execute until their reasoning is durably persisted)
2. **Streaming continuations** — the `cont` field for long reasoning that spans multiple entries
3. **Full reasoning capture** — the `think` field preserves reasoning tokens, not just their hash
4. **MITM proxy deployment** — invisible interception via `base_url` override

This document provides a field-by-field mapping for interoperability analysis and audit purposes.

---

## Field Mapping: Harness → AAT

| Harness Field | AAT Field | Mapping Notes |
|---------------|-----------|---------------|
| `v` | — | Protocol version. AAT does not have a version field per-record. |
| `seq` | (derived from `parent_record_id` chain) | Harness uses explicit monotonic integer; AAT uses UUID chain. Semantically equivalent for ordering. |
| `sid` | `session_id` | Harness uses 26-character ULID; AAT uses UUIDv4. Both uniquely identify sessions. |
| `ts` | `timestamp` | Identical. Both require RFC 3339 UTC with millisecond precision. |
| `model` | `model_id` | Harness requires this field; AAT marks it OPTIONAL. |
| `in` | `input_hash` | Identical semantics. SHA-256 hash of input. Harness computes over structured `{system, messages, tools}` object. |
| `reason` | (no direct equivalent) | Harness captures reasoning text directly. AAT uses `reasoning_hash` in `action_detail` for decisions. |
| `act` | `action_detail` | Harness `act` is an untyped object. AAT requires `action_type` classification. See "Taxonomy Gap" below. |
| `prev` | `prev_hash` | **Identical primitive.** Both use `SHA-256(JCS(previous_entry))` per RFC 8785. |
| `cont` | — | Harness extension for streaming. AAT does not address multi-entry reasoning units. |
| `think` | (no direct equivalent) | Harness captures full extended reasoning tokens. AAT's privacy-by-default design uses `reasoning_hash`. |
| `error` | (maps to `action_type: "error"`) | Harness uses optional `error` object; AAT uses `action_type: "error"` with `action_detail` fields. |
| `transparency` | — | Harness extension. Machine-readable presence flags for `think` and `act`. |

---

## Field Mapping: AAT → Harness

| AAT Field | Harness Equivalent | Notes |
|-----------|-------------------|-------|
| `record_id` | — | AAT uses UUIDv4 per-record. Harness uses `sid` + `seq` for unique addressing. |
| `timestamp` | `ts` | Identical. |
| `agent_id` | — | AAT requires URI format. Harness does not have agent identity beyond `model`. |
| `agent_version` | — | Not present in Harness. |
| `session_id` | `sid` | Format differs (UUID vs ULID); semantics match. |
| `action_type` | — | **Gap.** Harness `act` is untyped. See "Taxonomy Gap" below. |
| `action_detail` | `act` | Harness `act` contains action payload without type classification. |
| `outcome` | — | **Gap.** Harness does not classify action outcomes (success/failure/timeout/denied/escalated). |
| `trust_level` | — | **Gap.** Harness does not model trust tiers (L0-L4). |
| `parent_record_id` | (derived from `seq`) | AAT uses UUID reference; Harness uses `seq - 1` within session. |
| `prev_hash` | `prev` | **Identical.** |
| `human_override` | — | Not present in Harness. |
| `risk_score` | — | Not present in Harness. |
| `signature` | — | Harness does not implement ECDSA P-256 signing. Hash chain provides tamper evidence; cryptographic non-repudiation is not in scope. |

---

## Structural Mapping

### Session Lifecycle

| Concept | Harness | AAT |
|---------|---------|-----|
| Session start | `seq = 0`, `prev = "sha256:0000...0000"` | `action_type: "lifecycle"`, `action_detail.event: "session_start"`, `prev_hash: null` |
| Session close | `index.jsonl` entry with `last_hash` | `action_type: "lifecycle"`, `action_detail.event: "session_end"`, `session_hash` |
| Orphaned sessions | Reconciled on startup via index rebuild | Detected by absence of close record |

### Hash Chain

Both specifications use identical cryptographic primitives:

```
prev_hash(N) = "sha256:" + lowercase_hex(SHA-256(JCS(record(N-1))))
```

- **Canonicalization:** RFC 8785 (JSON Canonicalization Scheme)
- **Hash algorithm:** SHA-256
- **Encoding:** Lowercase hexadecimal (64 characters)

The only difference is genesis record encoding:
- **Harness:** `prev = "sha256:" + ("0" × 64)`
- **AAT:** `prev_hash = null`

Both convey "this is the first record." Conversion is trivial.

---

## Taxonomy Gap

AAT defines a controlled vocabulary for `action_type`:

| AAT action_type | Harness representation |
|-----------------|------------------------|
| `tool_call` | `act` object with tool invocation payload |
| `tool_response` | Not separately logged; response is part of next reasoning cycle's input |
| `decision` | `act: null` with reasoning in `reason` field |
| `delegation` | Not in scope (single-agent focus) |
| `escalation` | Not in scope |
| `error` | `error` object when `act: null` due to failure |
| `lifecycle` | Not logged (harness is proxy, not agent lifecycle manager) |

**Interoperability path:** A Harness-to-AAT transformer can infer `action_type` from entry structure:
- `act` non-null → `tool_call`
- `act` null + `error` present → `error`
- `act` null + `reason` present + no error → `decision`

This inference is lossy for multi-action patterns (e.g., `delegation`, `escalation`) that the Harness does not model.

---

## What the Harness Adds Beyond AAT

### 1. Enforcement Semantics (§9.1)

AAT is observational — it defines what to log, not when to release actions. The Harness adds:

> An action MUST NOT be released to any executor before its containing entry is durably persisted.

This is the fail-closed guarantee that makes the ledger architecturally binding, not advisory.

### 2. Streaming Continuations (§7)

AAT assumes one record = one complete action. The Harness supports long reasoning split across entries:

- `cont: "open"` — more entries follow for this reasoning unit
- `cont: "closed"` — reasoning unit complete, action may be released
- `cont: "abandoned"` — session resumed after incomplete unit

AAT has no equivalent; implementers would need to concatenate multi-record reasoning manually.

### 3. Full Reasoning Capture

AAT's privacy-by-default design stores `reasoning_hash` — the hash of reasoning, not the reasoning itself. The Harness stores:

- `reason` — the model's reasoning text
- `think` — extended thinking / chain-of-thought tokens from providers that expose them

This enables post-hoc analysis of reasoning quality (e.g., ARF probes) at the cost of larger ledger files and potential privacy exposure.

### 4. Invisible MITM Deployment

The Harness operates as a transparent proxy via `base_url` override, requiring no client library changes. AAT is transport-agnostic but assumes the agent itself emits records. The Harness intercepts and logs externally.

---

## What the Harness Omits from AAT

| AAT Feature | Status | Impact |
|-------------|--------|--------|
| `agent_id` / `agent_version` | Not implemented | Cannot distinguish agents in multi-agent deployments |
| `action_type` taxonomy | Not implemented | Requires inference for AAT export |
| `outcome` classification | Not implemented | Success/failure not machine-readable |
| `trust_level` (L0-L4) | Not implemented | Cannot express graduated autonomy |
| `signature` (ECDSA P-256) | Not implemented | No cryptographic non-repudiation |
| Tombstone records | Not implemented | No GDPR Article 17 deletion support |
| `human_override` | Not implemented | Human interventions not logged |

These omissions reflect scope: the Harness is a reasoning-capture substrate for ARF measurement, not a full agent governance system.

---

## Conversion Examples

### Harness Entry → AAT Record

**Harness:**
```json
{
  "v": 1,
  "seq": 1,
  "sid": "01KVEYV31SRY0KW4V48BBD5BR8",
  "ts": "2026-06-19T10:30:00.123Z",
  "model": "claude-haiku-4-5-20251001",
  "in": "sha256:9f86d081884c7d659a2feaa0c55ad015...",
  "reason": "The user asked for a code review...",
  "act": {"type": "function", "name": "read_file", "arguments": "{\"path\": \"src/main.py\"}"},
  "prev": "sha256:7d865e959b2466918a2b4e1f5d3c8a0b..."
}
```

**AAT equivalent:**
```json
{
  "record_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "timestamp": "2026-06-19T10:30:00.123Z",
  "agent_id": "urn:agent:harness-proxy",
  "agent_version": "1.0.0",
  "session_id": "01KVEYV31SRY0KW4V48BBD5BR8",
  "action_type": "tool_call",
  "action_detail": {
    "tool_name": "read_file",
    "parameters_hash": "sha256:...",
    "reasoning_hash": "sha256:..."
  },
  "outcome": "success",
  "trust_level": "L1",
  "parent_record_id": "a1000000-0000-4000-8000-000000000000",
  "prev_hash": "7d865e959b2466918a2b4e1f5d3c8a0b...",
  "model_id": "claude-haiku-4-5-20251001",
  "input_hash": "9f86d081884c7d659a2feaa0c55ad015..."
}
```

**Notes:**
- `record_id` generated (UUIDv4)
- `agent_id` synthesized (harness doesn't capture this)
- `action_type` inferred from `act` structure
- `outcome` assumed "success" (harness doesn't capture)
- `trust_level` defaulted to L1 (harness doesn't capture)
- `reasoning_hash` computed from `reason` field
- `parameters_hash` computed from `act.arguments`

---

## Regulatory Alignment via AAT

Because the Harness implements AAT's core hash-chaining primitive, it inherits AAT's regulatory mappings:

| Regulation | AAT Section | Harness Compliance |
|------------|-------------|-------------------|
| EU AI Act Article 12 (Record-Keeping) | §9.1 | ✅ Hash-chained records, session structure |
| EU AI Act Article 13 (Transparency) | §9.1 | ✅ Full reasoning capture exceeds AAT baseline |
| SOC 2 CC7.2 (System Monitoring) | §9.2 | ✅ Tamper-evident logging |
| ISO/IEC 42001 Clause 8.4 | §9.3 | ✅ Session audit trails |
| PCI DSS v4.0.1 Req 10.2, 10.5 | §9.4 | ✅ Audit logs with integrity protection |

**Gap:** EU AI Act Article 12(3) retention requirements (6-12 months) are deployment policy, not protocol-level. The Harness does not enforce retention.

---

## References

- [IETF Agent Audit Trail (draft-sharif-agent-audit-trail-00)](https://datatracker.ietf.org/doc/html/draft-sharif-agent-audit-trail)
- [RFC 8785: JSON Canonicalization Scheme](https://datatracker.ietf.org/doc/html/rfc8785)
- [Harness Protocol Specification](../SPEC.md)
- [Principles of Earned Autonomy (PEA)](https://github.com/ntholm86/pea-manifesto)
