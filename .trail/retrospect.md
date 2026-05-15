# retrospect.md — harness-protocol

_Last updated: 2026-05-15 (run: post-phase-one-feature-sprint)_

## Current claims

**1. The loop has substantially resolved its own prior self-indictment.**
The 2026-05-07 retrospect named "built deeply, recorded shallowly" as the dominant finding. The arc since then: 10 trail entries across 5 improve iterations, 10 commits, every entry with a pre-commit prediction and a reflection. That finding is no longer the limiting factor. A future run can falsify this by finding an undocumented architectural decision or reversal in the 2026-05-15 sprint.

**2. Every feature in this phase was a direct application of the dumb-pipe principle — none required a tradeoff.**
`think` field: capture more of what passes through. Streaming tee: stop buffering. Transparency flags: machine-readable signal on what the pipe saw. Gemini: extend to a third provider. SPEC.md: document what the pipe does. The architecture was stable; the loop filled it in. A future run can falsify this by finding a change that required choosing between fail-closed and usability (other than the documented SSE ceiling).

**3. The proxy's structural extensibility was emergent, not designed.**
Three provider families (OpenAI/Grok, Anthropic, Gemini) in ~500 lines of Rust. The Gemini trail entry notes: "The extensibility was not planned explicitly, it emerged from the dumb-pipe principle." Every new provider follows the same pattern — route, handler, extract, accumulate. A future run can falsify this if adding a fourth provider requires refactoring the existing pattern rather than extending it.

**4. The fail-closed guarantee has been publicly weakened and honestly documented.**
The streaming ceiling (SSE mode weakens fail-closed) appears in the code, in the trail, and in SPEC.md §9.5. No arc entry glosses over it. A future run can falsify this by finding a streaming code path where the ceiling is not disclosed to the caller.

**5. Streaming tool call reconstruction has been deferred in three consecutive improve iterations without implementation.**
It appeared as a top-ranked candidate in: streaming tee (ranked #1), transparency (ranked #2), Gemini (ranked #2). Three consecutive entries. Not implemented. This is the single most consistently avoided item in the arc. Calling it "deferred" is a euphemism — the loop has been routing around it. A future run can falsify this by finding a commit that implements stateful `input_json_delta` accumulation for streaming tool calls.

**6. The proxy's integrity layer has never been tested at runtime.**
SPEC §12 documents 8 conformance test classes (round-trip, tamper detection, crash recovery, continuation gating, cross-process sequence). Zero test files exist. Every claim about the proxy's behaviour — fail-closed writes, chain integrity, fsync before response — has been verified only by code review through a remote CI compile gate. The proxy has never been exercised against a real LLM client in any trail entry. Vision names the end-to-end test as open since 2026-05-08. It has not been attempted.

**7. The self-hosting pledge remains unenacted after the full arc.**
Established in the 2026-05-07 vision run: "the race to build the harness so we can use the harness to finish the harness." The proxy is built, CI is green, three provider families are supported, the spec is current. The proxy has still never been used to record a development interaction on its own codebase. This is the longest-running open commitment in the arc. It predates the extension deletion, the Rust rebuild, and all five improve iterations in this sprint.

**8. The loop has been solving faithfulness while leaving the integrity layer's verification undone.**
This phase concentrated entirely on the extraction/capture layer: fields, providers, streaming, schema documentation. The integrity layer (ledger write, hash chain, fsync, concurrent-access safety) has not been touched since the initial Rust implementation in 2026-05-08. Vision's core claim is: "The agent is structurally incapable of receiving a response until the ledger has accepted it. Fail-closed." That claim has no test coverage. Faithfulness improvements built on an unverified integrity foundation strengthen the wrong layer first.

---

## What the next runs should test

**1. End-to-end proxy verification — highest leverage, directly tests the core claim.**
Run the proxy locally, point a real LLM client at `http://127.0.0.1:8080`, make a call, verify `.harness/sessions/*.jsonl` chain integrity. The local MSVC toolchain blocker exists (no `link.exe`); use WSL or a CI artifact download. This has been open since 2026-05-08. It is the first action that would actually demonstrate that the proxy delivers what vision promises.

**2. SPEC §12 conformance tests.**
Eight test classes are documented and zero exist. These cover the integrity layer — the part that has not been revisited since initial implementation. Round-trip, tamper detection, crash recovery, continuation gating, cross-process sequence. The proxy can have beautiful capture features and still silently corrupt entries under concurrent access. The spec says what must be tested.

**3. Streaming tool call reconstruction.**
The most consistently avoided item. `input_json_delta` events arrive fragmented; the `act` field for streaming agentic workflows is currently a presence marker only. Agentic workflows are the primary use case for Observable Autonomy. Full recording of tool calls requires this.

**4. Self-hosting enactment.**
Use the proxy to record a real development session on harness-protocol itself. Even one captured `.harness/sessions/*.jsonl` from a development interaction satisfies the founding pledge. This is not a feature — it is a credibility test.

---

## Active operational rules

- **Spec updates belong with every feature commit, not in periodic catch-up iterations.** The SPEC.md catch-up (commit `8176fc6`) was necessitated by three iterations of accumulated schema drift. Update SPEC.md in the same commit as the schema change.
- **Integrity layer before capture layer.** Do not add new extraction features without verifying the underlying write path. A chain is only as trustworthy as its least-tested link.
- **Name avoidance when it happens.** When the same item appears as a top candidate in three consecutive trail entries and is not implemented, it is not being deferred — it is being avoided. Name the reason explicitly or implement it.
- **End-to-end before scope expansion.** Before adding a fourth provider or a new ledger field, establish that the proxy works end-to-end with a real client.
- **Self-hosting gate (carried forward, still unmet).** Before declaring any capability "done," ask: has the proxy recorded a development interaction on this project? If no, the self-hosting pledge is unmet.

---

## Loop-effectiveness notes

The 2026-05-07 retrospect found: high build effectiveness, low record effectiveness. That is substantially resolved — the trail is now dense, honest, and commit-aligned.

The new risk is the opposite failure mode: **the loop is iterating on visible, enumerable features while the core guarantee (fail-closed writes, chain integrity under adversarial conditions) remains invisible and unverified.** Feature velocity is real. The credibility gap has moved from "unrecorded" to "untested." The proxy is well-built and honestly documented. It may also be broken in ways no one has looked for, because no test has been run.

The most important sentence in vision: "The agent is structurally incapable of receiving a response until the ledger has accepted it." That sentence has not been tested. Running the proxy is the single action that would begin to test it.
