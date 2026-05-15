# Harness Protocol Specification

**Version:** 1
**Status:** Draft
**Conformance keywords** (MUST, MUST NOT, SHOULD, SHOULD NOT, MAY) are used as defined in RFC 2119 / RFC 8174.

---

## 1. Overview

The Harness Protocol defines an enforcement and transport architecture for tamper-evident, append-only ledgers of LLM reasoning and actions. It builds upon the baseline ledger format defined in the *Agent Audit Trail* specification (`draft-sharif-agent-audit-trail`) by introducing strict fail-closed action gating, streaming continuations, and an invisible Man-In-The-Middle (MITM) proxy deployment model.

The protocol covers: the required entry subset, canonicalization, hash chaining, storage layout, write ordering, and crucially, the failure semantics that guarantee Architectural Constraint. It does NOT cover: reasoning correctness, authorization, or encryption.

## 2. Relationship to Prior Art

The core cryptographic primitive—an append-only, hash-chained JSON ledger tracking autonomous agent actions—was independently developed by multiple parties in early 2026. The initial primitive was disclosed and implemented as the "proof ledger" in the `evo` open-source framework (GitHub commit `3964ca0`, 22 March 2026), followed a week later by the *Agent Audit Trail* Internet-Draft `draft-sharif-agent-audit-trail` (29 March 2026), which codified a specific JCS-canonicalized JSONL variant.

This specification acknowledges the Sharif AAT draft as the baseline ledger component and focuses entirely on the **enforcement mechanisms** required to realize *Observable Autonomy* (see the companion `manifesto` repository):

1. **Fail-closed semantics:** The guarantee that an action is never released or executed if its reasoning is not durably persisted.
2. **Invisible MITM proxy:** A deployment architecture that intercepts standard LLM traffic via `base_url` overrides, requiring zero client-side library integration.
3. **Streaming continuations:** The `cont` states that prevent action release until a logical reasoning unit has completed.

---

## 3. Terminology

- **Entry** — a single JSON object representing one unit of model output.
- **Ledger** — an ordered sequence of entries, persisted as a JSONL file.
- **Session** — a contiguous sequence of entries sharing one `sid`. Identified by a ULID.
- **Action** — a side-effecting operation (e.g., tool call) emitted by the model.
- **Canonicalization** — RFC 8785 (JSON Canonicalization Scheme, "JCS").
- **JCS bytes of X** — the byte sequence produced by applying RFC 8785 to JSON value X.

---

## 4. Entry format

### 3.1 Encoding

Each entry MUST be a single JSON object serialized as one line of UTF-8 text terminated by a single LF (`\n`, U+000A). CRLF MUST NOT be used.

### 3.2 Required fields

Every entry MUST contain:

| Field   | Type    | Definition                                                                 |
|---------|---------|----------------------------------------------------------------------------|
| `v`     | integer | Protocol version (see §10).                                                 |
| `seq`   | integer | Monotonic sequence number within the session, starting at 0.               |
| `sid`   | string  | Session identifier. MUST be a 26-character Crockford-base32 ULID.          |
| `ts`    | string  | RFC 3339 UTC timestamp with millisecond precision and `Z` suffix.          |
| `model` | string  | Model identifier producing this entry.                                     |
| `in`    | string  | `"sha256:" + lowercase_hex(sha256(JCS_bytes(input)))`. See §10.             |
| `reason`| string  | Model reasoning text. MAY be empty.                                        |
| `act`   | object \| null | Action object, or `null` if this entry carries no action.            |
| `prev`  | string  | `"sha256:" + lowercase_hex(sha256(JCS_bytes(previous_entry)))`. See §10.    |

### 3.3 Optional fields

| Field   | Type   | Meaning                                                            |
|---------|--------|--------------------------------------------------------------------|
| `cont`  | string | `"open"` or `"closed"`. See §10. Absence is equivalent to `"closed"`. |
| `error` | object | Forensic error record. Present only when `act` is `null` due to a schema or processing failure. |

Implementations MAY add fields not listed here. Readers MUST ignore unknown fields (forward compatibility).

### 3.4 Field constraints

- `seq` of the first entry of a session MUST be 0.
- `seq` MUST increase by exactly 1 between consecutive entries of the same session.
- `prev` of the entry with `seq = 0` MUST be `"sha256:" + ("0" repeated 64 times)`.
- `act` MUST be `null` on any entry with `cont: "open"` (see §10.3).

---

## 5. Hash chain

For an entry E with `seq > 0`, `E.prev` is computed over the entire previous entry — including that entry's own `prev` field — as follows:

```
E.prev = "sha256:" + lowercase_hex(sha256(JCS_bytes(E_previous)))
```

Where `E_previous` is the JSON value of the immediately preceding entry in the ledger file, parsed and re-canonicalized via RFC 8785.

A reader verifies the chain by computing `sha256(JCS_bytes(E_n))` and comparing to `E_{n+1}.prev` for every adjacent pair.

A ledger whose chain does not verify is **invalid**. Implementations MUST refuse to append to an invalid ledger and MUST report the first invalid `seq` to the caller.

---

## 6. The `in` field

`in` records the LLM input that produced this entry's reasoning and action.

`input` is the JSON value:

```json
{
  "system": "<system prompt or null>",
  "messages": [ ... ],
  "tools": [ ... ]
}
```

containing the system prompt, the full message history submitted to the model, and any tool definitions in scope at the time of the call. Implementations MUST include all three fields; absent components MUST be represented as `null` or `[]` rather than omitted.

The hash is computed once per LLM call, not per entry, and reused for every entry produced by that call (including all continuation entries — see §10).

---

## 7. Streaming and continuations

Long reasoning MAY be split across multiple entries.

### 9.1 Open continuation

An entry with `cont: "open"` indicates that further entries with the same `sid` will continue the same logical reasoning unit.

### 9.2 Closed continuation

An entry with `cont: "closed"` (or with `cont` absent) terminates the logical reasoning unit.

### 9.3 Action gating

An `act` other than `null` MUST appear only on a closed entry. An open entry with a non-null `act` is a protocol violation; readers MUST flag it and writers MUST NOT produce it.

### 9.4 Abandoned continuations

If a session is reopened and the last entry of that session has `cont: "open"`, the implementation MUST append exactly one sealing entry with the next `seq`, `cont: "abandoned"`, `reason: ""`, `act: null`, and a valid `prev` before any new content is written.

---

## 8. Storage layout

### 9.1 Required layout

A conformant implementation MUST organize ledger files as:

```
<root>/
  sessions/
    <sid>.jsonl
  index.jsonl
```

Where `<root>` is an implementation-chosen directory (default name SHOULD be `.harness/`).

### 9.2 Session files

- Exactly one file per session, named `<sid>.jsonl`.
- File MUST contain only entries with that `sid`.
- File MUST contain only entries of a single `v` (see §10).

### 9.3 The index

`index.jsonl` is itself a hash-chained ledger. Each line is a JSON object:

| Field         | Type    | Meaning                                                       |
|---------------|---------|---------------------------------------------------------------|
| `v`           | integer | Protocol version.                                             |
| `seq`         | integer | Monotonic across the index file.                              |
| `sid`         | string  | Session this index entry describes.                           |
| `started`     | string  | `ts` of the session's `seq=0` entry.                          |
| `ended`       | string  | `ts` of the session's last entry.                             |
| `entry_count` | integer | Total entries in the session file.                            |
| `last_hash`   | string  | `"sha256:" + lowercase_hex(sha256(JCS_bytes(last_entry)))`.   |
| `prev`        | string  | Hash of previous index entry, per §5 rules.                   |

### 9.4 Index lifecycle

- One index entry per session, appended on clean session shutdown.
- On startup, the implementation MUST enumerate `sessions/`. For every session file with no corresponding index entry, the implementation MUST compute `entry_count` and `last_hash` by reading the session file and append a reconciling index entry.
- The session file is the source of truth that a session existed. The index is derivable and MAY be rebuilt by scanning.

### 9.5 Canonical root (optional)

An implementation MAY accept configuration designating a fixed `<root>` (e.g., a repository's `.harness/` directory) as the canonical ledger location. This is a deployment configuration; it does not change protocol semantics.

---

## 9. Write semantics

### 9.1 Fail-closed action release

An action MUST NOT be released to any executor before its containing entry is durably persisted. "Durably persisted" means the entry's bytes have been written to the session file and `fsync` (or platform equivalent) has returned successfully.

### 9.2 Failure handling

| Condition                          | Required behavior                                                                                                                |
|-----------------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| Write or `fsync` fails             | Action MUST NOT be released. Error MUST be returned to caller.                                                                   |
| Hash chain verification fails      | Writer MUST halt. New appends MUST be refused until manual recovery.                                                              |
| Schema validation of action fails  | Action MUST NOT be released. The entry MAY be written with `act: null` and an `error` object describing the failure.              |
| Process crashes mid-entry          | The torn final line MUST be detected on next open (parse failure on last line). Recovery MUST truncate to the last valid entry.   |

### 9.3 Sequence continuity across processes

A session's `seq` is monotonic for the lifetime of its `sid`, regardless of how many processes append to it. A process resuming a session MUST read `last_seq` from the session file and start at `last_seq + 1`.

### 9.4 Single writer per session

A session file MUST have at most one writer at any instant. Implementations SHOULD use a file lock (`flock` / `LockFileEx`) to enforce this. Cross-session parallelism is unrestricted.

---

## 10. Versioning

- `v` is an integer denoting a breaking-change generation.
- Backward-compatible additions (new optional fields, new `cont` states) MUST NOT bump `v`. Readers MUST ignore unknown fields and unknown optional values.
- Renaming, removing, or changing the semantics of an existing field MUST bump `v`.
- A single ledger file MUST contain entries of a single `v`.
- Readers MUST refuse to process entries with `v` greater than they support. Readers SHOULD process older `v` if they implement that version's specification.
- This document defines `v = 1`.

---

## 11. Conformance tiers

This specification aligns with the trust tiers defined in the *Agent Audit Trail* specification, but mandates strict write enforcement for compliance:

| Tier  | Requirements                                                                                                            |
|-------|-------------------------------------------------------------------------------------------------------------------------|
| L0    | Append-only file. No hash chain, no required fields beyond raw lines.                                                   |
| L1    | L0 + `seq`, `ts`, `sid` on every entry.                                                                                  |
| **L2**| L1 + RFC 8785 canonicalization, SHA-256 hash chain via `prev`, full §4 entry format, §8 storage layout, §9 write semantics. |
| L3    | L2 + cryptographic signature over `JCS_bytes(entry)` carried in a `sig` field, with key identifier in a `kid` field.           |

A claim of "Harness Protocol compliant" without qualification MUST mean **L2 or higher** with strictly enforced **Fail-Closed (§9.1)** action gating. L0 and L1 are named substandards and MUST NOT claim compliance.

L3 signature semantics are reserved; a future revision will fully specify the `sig` and `kid` fields.

---

## 12. Conformance test surface

A conformant L2 implementation MUST pass the following test classes (the test suite is maintained alongside this specification):

1. **Round-trip** — write N entries, read back, verify every `prev` matches the recomputed hash of the prior entry.
2. **Canonicalization** — given a fixed input value, two implementations produce byte-identical entries (modulo timestamp and `sid`).
3. **Tamper detection** — flipping one bit in any non-final entry causes verification to fail at the next entry.
4. **Crash recovery** — a file truncated mid-line is recovered to the last valid entry; subsequent appends produce a valid chain.
5. **Continuation gating** — a writer attempting to emit `act != null` with `cont: "open"` is rejected.
6. **Index reconciliation** — deleting `index.jsonl` and restarting reproduces a byte-identical index (modulo `ts` of the reconciliation event itself).
7. **Cross-process sequence** — two processes appending to the same session in alternation produce strictly increasing `seq` with no gaps and a valid chain.
8. **Version refusal** — a reader for `v=1` refuses an entry with `v=2`.

---

## 13. Non-goals

The following are explicitly outside the scope of this specification:

- Reasoning correctness or quality.
- Authorization of actions (the executor's responsibility).
- Content moderation or safety filtering.
- Distributed consensus across writers (single-writer-per-session by design).
- Encryption at rest or in transit.
- Network transport between model, proxy, and executor.

---

## 15. Reference implementations

Two reference implementations are maintained in this repository:

### 15.1 Proxy (`proxy-rust/`)

A Rust binary implementing the invisible MITM proxy deployment model (§1).

- Listens on `127.0.0.1:8080` by default.
- Environment variables: `HARNESS_ROOT` (ledger directory, default `.harness/`), `HARNESS_UPSTREAM` (upstream API base URL, default `https://api.anthropic.com`).
- Implements §4 entry format, §5 hash chain, §6 `in` field, §8 storage layout, §9 write semantics (fail-closed, `fsync` before response released).
- Conformance tier: **L2**.
- Pre-built binaries (`harness-proxy.exe` / `harness-proxy`) are produced by CI at `.github/workflows/build-proxy.yml`.

---

## 14. Normative references

- RFC 2119 — Key words for use in RFCs to Indicate Requirement Levels.
- RFC 3339 — Date and Time on the Internet: Timestamps.
- RFC 8174 — Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words.
- RFC 8785 — JSON Canonicalization Scheme (JCS).
- FIPS 180-4 — Secure Hash Standard (SHA-256).
- ULID specification — github.com/ulid/spec.
- (L3 only) RFC 8032 — Edwards-Curve Digital Signature Algorithm (EdDSA), Ed25519.



