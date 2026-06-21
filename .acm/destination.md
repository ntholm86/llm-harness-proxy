# Destination — llm-harness-proxy

_Operator-held. Updated by Vision run 2026-05-19 (session: foundational-violation-surfaced)._

## The Destination
This repository is the practical **delivery mechanism** for the Autonomous Agent Principles defined in the manifesto repository. The goal is to build the "Immune System against Revisionism" so any developer can adopt Architectural Constraint with zero friction.

The hard principle everything hangs on: The harness is not a participant — it is a pipe and an append. The agent is structurally incapable of receiving a response until the ledger has accepted it. Fail-closed.

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

---

## [2026-05-15] Direction change — proxy only. Extension gone.

_Updated this session after reading all vision files across the workspace and examining the current proxy-rust source._

**[!DIRECTION CHANGE] The VS Code extension is deleted. Permanently. Not rebuilt.**

The extension was rebuilt on 2026-05-08 as a "dumb viewer" after the chatParticipant violation was confirmed. Today the operator concluded the viewer concept itself is not needed. The harness does not require a built-in UI to deliver its value. The repo's scope is now: the Rust proxy, the SPEC, and the ledger format. Nothing more.

**Scope is now: Rust proxy only.**

`llm-harness-proxy` is a single-purpose external HTTP proxy. It intercepts LLM API traffic, writes a cryptographically hash-chained ledger entry before forwarding the response, and fails closed if the write fails. That is the entire product. It requires no host process, no IDE plugin, no VS Code.

**Standalone and detached — confirmed.**

The harness has no knowledge of ai-steward or any other calling system. Any developer, any project, any provider can point their LLM client's `base_url` at the proxy and get Observable Autonomy for free. The governance boundary between llm-harness-proxy and ai-steward is structural: separate repos, separate ownership, no direct dependency in either direction.

**Stale items retired:**
- "Python MVP exists but crashes" — superseded by the Rust implementation. CI confirmed green (Windows x64 + Linux x64). `llm-harness-proxy.exe` is already built.
- "Remove recording logic from chatParticipant.ts" — moot. The extension is gone.
- "Marketplace publish" — moot. No extension to publish.
- "Tests for ledgerWriter.ts" — moot. No TypeScript ledger path remains.

**What is still open — revised priority order:**
1. Delete `extension/` folder from repo, update README.md and SPEC.md to remove extension references. ✓ Done 2026-05-15.
2. **Streaming capture** — current proxy buffers full response body before writing one ledger entry. Next meaningful work: capture the full stream verbatim in order (prompt → tool call → tool result → reasoning chunk → reply chunk). This is what closes the faithfulness gap.
3. SPEC.md section explicitly specifying thinking-token / reasoning-block capture as a required ledger field.
4. End-to-end test: run proxy locally, point a real client at `http://127.0.0.1:8080`, verify `.harness/sessions/*.jsonl` chain integrity.

---

## [2026-05-15] Vision run — proxy scope confirmed, provider-aware capture, streaming prerequisite

_Confirmed after examining proxy source and cross-referencing ai-steward vision._

### What the harness is

**Dumb pipe, provider-aware.** The harness captures everything that flows through a model interaction — text, tool calls, thinking blocks, reasoning traces — verbatim, in order. It makes zero decisions about what matters. It records everything.

It is provider-aware but not provider-opinionated: it knows how Anthropic, OpenAI/GPT, Gemini, and Grok structure their responses, so it can extract and label each content type correctly. This knowledge is purely structural — parsing, not judging.

**Scoring stays in ai-steward.** The harness emits a `transparency` metadata field per entry (what was present: thinking tokens, tool calls, reasoning trace, etc.) but makes no judgment about whether that is good or bad. ai-steward reads the ledger and applies the scoring. The harness does not reason about its own captures.

### The thinking-block gap is a current bug

`extract_anthropic` currently discards `"type": "thinking"` blocks with `_ => {}`. This is the most valuable data the harness is supposed to capture, and it is being silently dropped today. Fix is immediate — add a `think` field to the ledger schema and extract thinking blocks from all four providers.

### Streaming is a prerequisite for real use — with a documented ceiling

The proxy currently buffers the entire response before forwarding. Any client using `"stream": true` gets a broken experience. Streaming support is required before the proxy can be used in actual workflows.

**The fail-closed guarantee under streaming:** Strict fail-closed (withhold response if ledger write fails) is only achievable in buffered mode. Under streaming, the harness tees the stream — forwarding chunks to the client as they arrive while accumulating a buffer — and writes the ledger entry when the stream closes. If the final ledger write fails, the response has already been delivered chunk by chunk. This weakens the guarantee from "response withheld" to "stream flagged as unrecorded." This is an honest ceiling, not a design failure. It will be documented explicitly in SPEC.md.

**Provider transparency ceiling:** Not all model families expose the same internal state. Where thinking tokens or reasoning traces are unavailable, the harness documents what is absent rather than silently omitting it. The `transparency` field in each ledger entry records what was available, so the ceiling is visible per call.

### Provider scope

Four providers to support, in priority order:
1. **Anthropic** — `content[]` blocks: `text`, `tool_use`, `thinking` (extended thinking). Currently: `thinking` discarded.
2. **OpenAI/GPT** — `choices[0].message`: `content` (text), `tool_calls`. Reasoning tokens (o-series): available as a count in `usage.completion_tokens_details.reasoning_tokens`, not as content. Ceiling: reasoning content not exposed by OpenAI API.
3. **Gemini** — `candidates[0].content.parts[]`: `text`, `functionCall`, `thought` (Gemini 2.0+ thinking mode). Route: `/v1beta/models/*/generateContent`.
4. **Grok (xAI)** — OpenAI-compatible format. Route: `/v1/chat/completions` against `api.x.ai`. Grok 3 thinking: `reasoning_content` field alongside `content`.

### What is open — current priority order
1. **Ledger schema**: add `think` field for reasoning/thinking blocks; add `transparency` metadata object.
2. **Fix Anthropic extraction**: capture `thinking` blocks into `think` field.
3. **Fix OpenAI extraction**: capture reasoning token count; document content-not-exposed ceiling.
4. **Add Gemini support**: new route + extraction for `thought` parts.
5. **Add Grok support**: extend OpenAI handler or add dedicated route; extract `reasoning_content`.
6. **Streaming**: tee architecture, write ledger at stream close, document weakened fail-closed guarantee.
7. **SPEC.md**: update schema section (§4), add provider notes per family with ceiling documentation.


---

## 2026-06-20 — Trail write-path operational rule

**Trail entries for this repo always go to .acm/audit-trail.md in the repo root.**
Never .harness/ — that directory is for JSONL proxy session evidence only.

This rule exists because a 2026-06-19 session wrote an improve-skill trail entry to
.harness/trail.md by mistake, confusing the proxy session directory with the trail directory.
The entry was reconstructed into .acm/audit-trail.md on 2026-06-20.

**Write-path ownership (for future sessions):**
- .acm/audit-trail.md — improve / trail skill (AI assistant writes here)
- .acm/retrospect.md — retrospect skill
- .acm/destination.md — operator / destination skill
- .acm/history.md, .acm/learning.md — skills-suite 	ools/record.py (generated)
- .acm/log.md — legacy format trail (read-only)
- .harness/sessions/ — proxy JSONL evidence (written by the proxy binary only)
