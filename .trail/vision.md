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

## Ambient Recording — destination, now technically reachable (with one caveat)
The operator confirmed: the real destination is a **silent always-on recorder** where every Copilot interaction is captured automatically with zero change to how the operator works. No @harness invocation required.

**Update 2026-05-08:** A viable mechanism has been identified — `vscode.lm.registerLanguageModelChatProvider`. An extension can register itself as a chat model provider that appears in the Copilot Chat model dropdown alongside GPT-4o, Claude, etc. When the user selects it, `provideLanguageModelChatResponse(model, messages, options, progress, token)` receives every prompt and must emit every response chunk via `progress` — full stream, in order, in extension hands. The extension forwards to the real upstream model via `vscode.lm.selectChatModels(...).sendRequest(...)` (using the user's existing Copilot subscription — no personal API key) and tees every chunk to the ledger before releasing it downstream. Fail-closed is achievable at the chunk boundary.

**The caveat — one-time selection, not zero-touch:** The user must pick the harness provider from the chat model dropdown once. After that, every chat is captured silently. This is closer to ambient than @harness ever was, but it is not the fully invisible install-and-forget product the operator described. Whether this is acceptable for v1, or whether v1 must wait for a true global hook, is an open product decision.

**What this still does not give us:** Reasoning tokens depend on the upstream model and on whether Copilot's relay strips them. If the underlying model emits thinking blocks (Anthropic extended thinking, OpenAI o-series reasoning summaries), they may or may not reach the provider. Empirical verification required before claiming reasoning capture.

**What was wrong in the previous note:** "not yet possible with current VS Code APIs" was outdated. The provider API did not exist when chatParticipant was the only option. It now does. The shape changes from *be the participant* to **be the model**.

## Why the proxy solves two distinct problems — not one

This distinction matters for how the proxy is positioned relative to the skills suite and the manifesto.

**Problem 1 — Integrity:** The agent cannot author or edit its own trail. The proxy solves this structurally: the agent never touches the ledger; the proxy intercepts and writes before the response is released to the caller.

**Problem 2 — Faithfulness:** The skills suite asks the agent to write its own trail entry (pre-commit prediction, reasoning record). Even though pre-commit prediction prevents *after-the-fact* reconstruction, the stated reasoning may still not reflect the actual internal computation — the model's chain-of-thought is generated as output, not read off from a causal process. This is the Turpin et al. finding: CoT can be plausible and wrong.

**What the proxy adds on faithfulness:** The raw MCP/API response payload contains the model's extended thinking tokens — the scratchpad visible when you expand a model's reasoning in a chat UI. These are not a summary the model composed for the trail. They are the actual computation as it streamed. A proxy that intercepts at this layer captures thinking tokens, tool calls, tool results, and the final response — all verbatim, in order, written by the harness, not by the agent.

This is materially stronger than a self-reported trail entry. The proxy captures the closest available approximation to *what actually drove the output*, before the agent has any opportunity to compose a narrative around it.

**The remaining ceiling:** Thinking tokens are still generated tokens — they are not a direct read of internal weights or activation patterns. Whether they faithfully reflect internal computation is itself an open research question (Anthropic's interpretability work). But they are a fundamentally better signal than post-hoc self-report, and capturing them via proxy is the strongest evidence this framework can currently produce.

**Summary:** The proxy is not just an integrity layer on top of the skills suite. It is a more complete answer to the post-hoc rationalization problem — closing integrity and making a materially stronger claim on faithfulness than prompt-level mitigations alone can achieve.

## The Protocol Specification
SPEC.md is the single authoritative document for **both** paths. It must formally specify: the append-only ledger format, the JSON payload schema (separating thoughts from actions — NOT just the reply), the JCS canonicalization and SHA-256 hash-chain rules, the fsync/fail-closed contract. Any future harness client — JetBrains, CLI, Rust — must be implementable from SPEC.md alone.

## What is still open — in priority order
1. Fix the proxy — ModuleNotFoundError: No module named harness_proxy — the proxy is the correct architecture and must run cleanly.
2. Remove recording logic from chatParticipant.ts — it violates the foundational principle. Extension becomes viewer only.
3. Capture full stream in proxy — every token: prompt, tool call, tool result, reasoning chunk, reply chunk — verbatim, in order.
4. Ambient recording — mechanism identified (`vscode.lm.registerLanguageModelChatProvider`). Open: spike to verify reasoning-chunk visibility, then decide whether one-time provider-selection UX is acceptable for v1.
5. Marketplace publish — .vsix submission.
6. Tests for ledgerWriter.ts — 0 tests currently.
7. Rust rewrite of proxy — if performance demands it.
