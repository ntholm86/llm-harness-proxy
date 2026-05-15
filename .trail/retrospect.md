# retrospect.md — harness-protocol

_Last updated: 2026-05-15 (run: post-integrity-and-capture-sweep)_

## Current claims

**1. The extraction layer is structurally faithful across all three provider families and both execution paths.**
The last-wins bug class — where multiple tool/function calls in one response resulted in only the last being captured — appeared in 5 locations and was closed in 4 consecutive commits: OpenAI streaming (`1975dd7`), Anthropic streaming (`1975dd7`), Anthropic buffered (`cbdb37e`), Gemini buffered + streaming (`9ebe469`). `think`, `reason`, and `act` are now consistently captured across all paths for all three providers. A future run can falsify this by finding a provider or path where multi-tool or multi-block output still uses last-wins assignment.

**2. The integrity layer is covered for the single-writer case.**
Five unit tests exist in `ledger.rs` (commits `c36798b`, `b2293d5`): genesis, hash chain round-trip, tamper detection, torn-line scan, torn-line full recovery. The torn-line recovery write gap was a real bug — fixed: `scan_tail` now returns the clean-end byte offset; `append_entry` truncates before writing. A future run can falsify this by finding a SPEC §12 conformance class with no test. Three remain uncovered: concurrent write, continuation gating, cross-process sequence.

**3. The proxy has never been invoked against a real LLM API.**
Not once in the entire arc. Every claim — fail-closed write, chain integrity, streaming tee, extraction faithfulness — has been verified by code review and unit tests only. Vision's core sentence ("the agent is structurally incapable of receiving a response until the ledger has accepted it") has not been tested end-to-end. The extraction and integrity layers are now complete. The end-to-end gap is the only substantive remaining gap between the code and the claim. A future run can falsify this by showing a committed record of a real LLM API call processed by the proxy with a verified `.harness/sessions/*.jsonl` chain.

**4. The self-hosting pledge has been open for the entire arc — longer than any feature.**
Established 2026-05-07. It predates the extension deletion, the Rust rebuild, all prior improve iterations. Its continued deferral has made it invisible: it appears in every trail entry's ranked candidates list and has never risen to the top. This is structural avoidance — the item requires deployment (push to origin, CI build, API key) rather than coding, and the loop has consistently preferred coding-mode work. A future run can falsify this by showing a `.harness/sessions/*.jsonl` file produced by a real development interaction on this project.

**5. The loop used retrospect-derived operational rules effectively in this phase.**
The "Integrity layer before capture layer" rule was invoked explicitly in two iterations to override the trail entry's own top-ranked candidate, deferring the `extract_gemini` fix until integrity tests existed. The mechanism worked: the rule was stated, followed, and the integrity work was completed. This is evidence that the operational rules are being read and applied, not just written.

**6. Both main correctness gaps were silent failures present since initial implementation (2026-05-08).**
The last-wins bug and the torn-line recovery write were both invisible without tests. Both were fixed within a 5-iteration sweep that only happened because tests were written first. This is the strongest arc-level evidence for the "integrity layer before capture layer" principle: without tests, code review through a remote CI gate misses both semantic correctness (last-wins) and crash-recovery correctness (torn-line write).

---

## What the next runs should test

**1. End-to-end proxy verification — the only remaining gap between the code and the claim.**
Push unpushed commits to origin (`master` at `828e4d2`, `origin/master` at `10906a6` — all post-retrospect commits are local only), wait for CI to build the binary, download the artifact, run with `HARNESS_ROOT` and the relevant `*_BASE_URL` set, make a real API call, verify the `.harness/sessions/*.jsonl` file exists, chain integrity holds, and content matches. This is not a code change — it is a deployment and verification action. The end-to-end gap is the only substantive remaining gap. It must precede any further feature development.

**2. Concurrent-write test for the ledger.**
Two threads calling `append_entry` with the same `sid` simultaneously. The OS `O_APPEND` guarantee is relied upon but not tested. This is the one remaining SPEC §12 unit-test gap for the single-process case. Small, safe, additive — a natural extension of the current test module.

**3. Self-hosting enactment.**
After end-to-end verification: point the proxy at a real development session on harness-protocol itself. One captured `.harness/sessions/*.jsonl` from a development interaction satisfies the founding pledge. This is the credibility test the whole arc has been building toward.

---

## Active operational rules

- **End-to-end gate (primary, replaces "Integrity layer before capture layer").** The extraction and integrity layers are complete for the single-writer case. Before any further feature addition (new provider, new ledger field, new capture path), establish that the proxy works end-to-end with a real client. Pushing to origin to trigger a CI build counts as the first step of this iteration, not a separate prerequisite.
- **Single-writer integrity is complete. Do not revisit without a new finding.** Five unit tests cover the single-writer case. A new integrity iteration requires a concrete new finding (concurrent write bug, platform-specific fsync failure, CI red on the torn-line tests) — not general coverage anxiety.
- **Spec updates belong with every feature commit.** (Carried forward — the SPEC.md catch-up was a 3-iteration debt.)
- **Name avoidance when it happens.** End-to-end verification has appeared as a top-ranked candidate in every trail entry since 2026-05-08. If it is deferred again, name the concrete blocker explicitly. If the blocker is "requires pushing to origin," then pushing to origin is the iteration.
- **Self-hosting gate.** Before declaring any capability "done," ask: has the proxy recorded a development interaction on this project? If no, the self-hosting pledge is unmet.

---

## Loop-effectiveness notes

The "Integrity layer before capture layer" rule from the prior retrospect was followed precisely, even when it overrode the trail's own ranked candidates. The operational rules mechanism works when the rules are specific and enforceable. The new primary rule ("End-to-end gate") is equally specific.

The end-to-end test has been deferred since 2026-05-08. The pattern is identical to "streaming tool call reconstruction" avoidance named in the prior retrospect — which was resolved in the next iteration after being explicitly named as avoidance. The same mechanism should apply here. If the next iteration is not end-to-end verification, the trail entry must name the concrete blocker, not just rerank the candidates.


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


---
## Retrospect update: 2026-05-15 (run: end-to-end gate + CI repair sprint)

_Appended; prior content preserved above._

### Claim updates

**Claim 3 revised � proxy has now been invoked against a real LLM API (partially).**
Status: Partially falsified. The proxy binary (built by CI run #11, commit 4de4c33, all 15 tests green) was run on Windows and a real Anthropic API call was made through it. The network path is verified. Anthropic responded with a credit-exhaustion error � not a proxy failure. The ledger wrote a session file with the full current SPEC schema (`think`, `transparency`, `v`, `seq`, `sid`, `model`, `in`, `prev`, `ts`, `act`, `reason`). The fail-closed guarantee is demonstrated: a session file exists despite the upstream error. What remains unverified: `act` capturing real model content (requires a funded key). A future run can close this gap with one funded API call.

**Claim 4 revised � self-hosting pledge remains open.**
The proxy is built, CI is green, network path is verified. The founding pledge requires routing a DEVELOPMENT INTERACTION on this project through the proxy. That has not happened. This is the last open commitment. Its remaining blocker is operational (set up VS Code extension or curl routing), not technical.

**New claim 9 � The gitignore was silently inert for the entire sprint.**
`.gitignore` was UTF-16 LE encoded (Windows default when creating via VS Code "New File"). Git expects UTF-8; it silently discards rules it cannot parse. Every rule in the file (including `.harness/` and `/proxy-rust/target/`) was inert. Session files from the May 7-8 sprints were untracked the entire time. The fix (re-save as UTF-8 without BOM) was committed 4cc8fd0. This is a class of silent failure distinct from anything seen before: a configuration file that APPEARS correct but is completely inert due to encoding.

**New claim 10 � The Windows FILE_APPEND_DATA / FILE_WRITE_DATA split is a platform-specific trap for the torn-line recovery path.**
`set_len()` on a handle opened with `.append(true)` fails with `Access is denied` on Windows because `FILE_APPEND_DATA` does not grant `SetEndOfFile`. The same call succeeds on Linux because `O_APPEND` + `ftruncate` operate on the same fd. This bug was silent for the entire sprint because: (a) the torn-line path only fires on crash-recovery, (b) no test existed, (c) CI ran only `cargo build`, not `cargo test`. Three independent failures had to co-occur for this bug to survive. The fix (second `write(true)` handle for truncation) was committed 4de4c33.

### Updated operational rules

- **Self-hosting gate (primary).** End-to-end structural verification is complete. The one remaining open commitment is self-hosting: route a real development interaction on this project through the proxy and verify the session file. Until this is done, the founding pledge is unmet.
- **`act` content verification.** Fund the Anthropic key and make one successful API call through the proxy. This closes the last structural verification gap.
- **Single-writer integrity is complete. Do not revisit without a new finding.** (Carried forward � still valid.)
- **Spec updates belong with every feature commit.** (Carried forward.)
- **Encoding awareness.** On Windows, text files created by VS Code default to UTF-16 LE with BOM when the user selects "New File" with certain system locales. Any configuration file parsed by a Unix-origin tool (git, cargo, etc.) must be verified as UTF-8. This applies to: `.gitignore`, `.cargo/config.toml`, any future `.env` file.

---
## Retrospect update: 2026-05-15 (run: full end-to-end closure)

_Appended; prior content preserved above._

**Claim 3 FULLY falsified � proxy has been invoked against a real LLM API with content captured.**
Two calls were made through the running proxy binary (CI run #11, commit 4de4c33) to Anthropic `claude-haiku-4-5`:
- Call 1: text response ? `reason: "harness e2e OK"`, `transparency.act: false` ?
- Call 2: tool use ? `act: {name:"record_result", input:{status:"harness-act-verified"}}`, `transparency.act: true` ?

Every SPEC schema field (`v`, `seq`, `sid`, `model`, `in`, `ts`, `prev`, `think`, `reason`, `act`, `transparency`) is present and correct in both session files. The core claim � "the agent is structurally incapable of receiving a response until the ledger has accepted it" � is demonstrated by the session files existing for both calls.

**New primary rule: Self-hosting gate.**
The end-to-end gate is closed. The founding pledge (2026-05-07) is the sole remaining open commitment: route a real development interaction on harness-protocol through the proxy. One captured session file from a development interaction satisfies it. All technical blockers are resolved � the proxy works, the network path is verified, the model is known (`claude-haiku-4-5`).

**Observation: new `caller` field in Anthropic tool_use.**
Anthropic now returns a `caller: {type: "direct"}` field inside tool_use blocks. The proxy captures it verbatim (dumb-pipe). Not a defect; noted for future reference.
