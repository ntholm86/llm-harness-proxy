# Vision — harness-protocol

_Operator-held. Updated by Vision run 2026-05-19 (session: foundational-violation-surfaced)._

## The Destination
This repository is the practical **delivery mechanism** for the Autonomous Agent Principles defined in the manifesto repository. The goal is to build the "Immune System against Revisionism" so any developer can adopt Architectural Constraint with zero friction.

## The Foundational Principle — confirmed this session
**Observable Autonomy means every autonomous action, thought, reasoning — everything — is logged. Word-by-word, thought-by-thought. Like a git history for its domain.**

The harness must be structurally **outside** the agent — not a participant that records itself. An interceptor the agent cannot bypass. The agent receives the response **only after** the ledger has accepted it. If the ledger fails — the response is withheld. That is the fail-closed guarantee.

**[!VIOLATION CONFIRMED]** The current chatParticipant.ts approach violates this principle. The agent and the recorder are the same process. The agent IS deciding what to record — which is exactly the behaviour the harness exists to prevent. The reason field in the ledger is the final reply text, not the reasoning. Tool calls, intermediate decisions, and thinking steps are discarded before the ledger ever sees them.

## The Correct Architecture — confirmed this session
The harness must be **dumb and simple**. It contains no logic about what is important. It is a pipe and an append. Every event that flows through a model interaction — prompt, tool call, tool result, reasoning chunk, reply chunk — is written to the ledger in order, verbatim, with a timestamp and a type label. The harness makes zero decisions about what matters. It records everything.

## Two Delivery Paths — roles clarified
Both paths implement the same ledger contract and are governed by the same SPEC.md.

1. **The Proxy (the real harness — the enforcer):** Sits outside the agent between the HTTP client and the upstream LLM API. Intercepts the raw stream before returning it to the caller. The agent is structurally incapable of receiving a response unless the ledger is written first. This is the correct architecture for Observable Autonomy. The Python MVP exists but crashes (ModuleNotFoundError: No module named harness_proxy) — fixing this is the priority.

2. **The VS Code Extension (the viewer — not the recorder):** Its role is the sidebar ledger viewer, the proxy launcher, and a thin @harness pass-through that routes through the proxy. The chatParticipant.ts recording logic violates the foundational principle and must be removed or replaced with a dumb pipe to the proxy. The extension should never be the source of truth for the ledger.

## Ambient Recording — confirmed destination, not yet possible
The operator confirmed: the real destination is a **silent always-on recorder** where every Copilot interaction is captured automatically with zero change to how the operator works. No @harness invocation required. This is not yet possible with current VS Code APIs. For v1: accept explicit @harness invocation. Ambient recording is the next vision milestone.

## The Protocol Specification
SPEC.md is the single authoritative document for **both** paths. It must formally specify: the append-only ledger format, the JSON payload schema (separating thoughts from actions — NOT just the reply), the JCS canonicalization and SHA-256 hash-chain rules, the fsync/fail-closed contract. Any future harness client — JetBrains, CLI, Rust — must be implementable from SPEC.md alone.

## What is still open — in priority order
1. Fix the proxy — ModuleNotFoundError: No module named harness_proxy — the proxy is the correct architecture and must run cleanly.
2. Remove recording logic from chatParticipant.ts — it violates the foundational principle. Extension becomes viewer only.
3. Capture full stream in proxy — every token: prompt, tool call, tool result, reasoning chunk, reply chunk — verbatim, in order.
4. Ambient recording — future milestone when VS Code exposes chat middleware API.
5. Marketplace publish — .vsix submission.
6. Tests for ledgerWriter.ts — 0 tests currently.
7. Rust rewrite of proxy — if performance demands it.
