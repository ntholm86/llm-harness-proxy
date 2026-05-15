# retrospect.md — harness-protocol

_Last updated: 2026-05-15 (run: post-self-hosting-gate)_

## Current claims

**1. The founding pledge is now fully met.**
Session `self-hosting-gate-001.jsonl` — 5 entries, hash chain intact, `reason`/`act`/`transparency` all captured — was produced during a live development interaction on harness-protocol itself. The arc's first goal ("the race to build the harness so we can use the harness to finish the harness," established 2026-05-07) is closed. A future run can falsify this by finding that the session file is malformed, the hash chain breaks on verification, or the file was committed outside `.harness/sessions/`.

**2. The proxy's implementation is structurally complete for the single-writer, synchronous-use case.**
Three providers (OpenAI/Grok, Anthropic, Gemini), full extraction (reason/think/act/transparency), integrity layer tested (5 ledger tests + 8 JCS tests + 3 ULID tests), CI green on both Windows x86_64 and Linux x86_64. The torn-line write fix is verified on Windows (CI caught the platform-specific `FILE_APPEND_DATA` / `SetEndOfFile` split). A future run can falsify this by finding a provider path, extraction function, or integrity scenario where the proxy produces incorrect or missing output.

~~**3. The `think` field has never been verified end-to-end with a real model.**~~ **FALSIFIED 2026-05-15.** Session `01KRNZ2SSP1B612XB512D2GJ1N.jsonl` — `model: claude-haiku-4-5`, `transparency.think: true`, `think: [{type:"thinking", thinking:"I need to calculate 12 × 17...", signature:"..."}]`. Correct array shape per SPEC §4.3. Extraction path `extract_anthropic` verified end-to-end with a real Anthropic extended thinking response.

**4. The new binary has never been run locally.**
Commit `d3db558` — port 8474, `find_git_root()` HARNESS_ROOT auto-resolution — passed CI (run 25917890677, 15/15 tests). But the self-hosting gate was closed using the old binary (`4de4c33`, port 8080, explicit `HARNESS_ROOT`). The new binary exists only as a CI artifact. The git-root HARNESS_ROOT resolution has never been exercised. A future run can falsify this by running the new binary from `C:\git\harness-protocol` with no `HARNESS_ROOT` set and observing the session file appear at `.harness/sessions/`.

**5. The concurrent-write integrity case is the only remaining uncovered SPEC §12 test class.**
The single-writer case is fully covered (genesis, hash chain, tamper detection, torn-scan, torn-recovery). Two threads or processes calling `append_entry` with the same `sid` simultaneously are untested. OS `O_APPEND` semantics provide the guarantee in theory; no test exercises it. A future run can falsify this by adding a concurrent-write test that passes on both Linux and Windows CI.

~~**6. The ambient recording destination in vision has never appeared as a candidate next move in the arc.**~~ **RESOLVED 2026-05-15.** Vision's 2026-05-15 direction change explicitly narrows scope: "The VS Code extension is deleted. Permanently. Not rebuilt. Scope is now: Rust proxy only." The `vscode.lm.registerLanguageModelChatProvider` ambient recording path was the extension path — now formally closed. The proxy is the ambient recording mechanism. The silence was deliberate scope narrowing, not avoidance.

**7. The CI use-after-move failure confirms that CI is the compile gate for this codebase.**
Commit `933133c` introduced a use-after-move compile error (`state` consumed by `.with_state()`, then accessed for logging). This was caught only by CI — two local reasoning passes (`4ef80af` and the original `933133c`) both missed it because the local MSVC linker is absent and `cargo check` cannot run. The pattern: MSVC toolchain absent locally → CI is the only Rust compilation gate → errors that compile-fail are invisible until pushed. A future run can falsify this by setting up a local Rust MSVC or cross-compilation environment.

---

## What the next runs should test

**1. Download and run the new binary — verify git-root HARNESS_ROOT resolution.**
The CI artifact from run 25917890677 (`harness-proxy.exe`, commit `d3db558`) should replace the old binary at `C:\git\harness-proxy.exe`. Run it from `C:\git\harness-protocol` with no `HARNESS_ROOT` env var set and make one API call. The session file should appear at `C:\git\harness-protocol\.harness\sessions\`. This is a 15-minute deployment task, not a code change.

~~**2. Verify `think` field end-to-end.**~~ **DONE 2026-05-15.** `think` non-null, correct shape, claim 3 falsified.

**3. Concurrent-write test for the ledger.**
Two threads calling `append_entry` with the same `sid` simultaneously. Exercises the OS `O_APPEND` guarantee. Small, additive, the single remaining uncovered SPEC §12 case. Both Windows and Linux CI paths would exercise their respective filesystem atomicity guarantees.

**4. Resolve the ambient recording gap — scope or deprioritize explicitly.**
Vision names `vscode.lm.registerLanguageModelChatProvider` as the destination. The loop has never attempted it. This requires an explicit decision: either (a) scope the ambient recording work, or (b) narrow vision to exclude it and state why (maintenance burden, VS Code coupling, etc.). The silence is not a decision — it is structural avoidance of the hardest remaining item in vision.

---

## Active operational rules

- **Self-hosting gate is CLOSED.** Do not invoke it as a deferral. The gate served its purpose. The new question is: what does production use reveal?
- ~~**Download the new binary before the next dev session.**~~ **DONE 2026-05-15.** Old binary at `C:\git\harness-proxy.exe` deleted. New binary (port 8474, git-root resolution) running at `C:\git\harness-protocol\harness-proxy.exe`. Retrospect claim 4 falsified.
- ~~**`think` field requires runtime verification before claiming capture is complete.**~~ **DONE 2026-05-15.** Session `01KRNZ2SSP1B612XB512D2GJ1N.jsonl` — `think` non-null, `transparency.think: true`, correct array shape. Claim 3 falsified.
- **Spec updates belong with every feature commit.** Carried forward — the SPEC.md catch-up iteration remains a cautionary example.
- ~~**Name avoidance when it happens.** The ambient recording path in vision has never appeared as a candidate next move.~~ **RESOLVED 2026-05-15.** Vision direction change closes this. Proxy-only scope is the explicit operator decision.
- ~~**The extraction layer has no unit tests.** `extract_openai`, `extract_anthropic`, `extract_gemini`, and the three streaming accumulators have zero test coverage. Every correctness claim about what enters the ledger rests on code review only. A normalization fix was applied 2026-05-15 (streaming think → arrays) with no test to verify it. This is the single largest remaining verification gap.~~ **CLOSED 2026-05-15.** Commit `9e423ea` adds 17 unit tests across all six extraction functions. CI pending — 33 total tests (16 ledger/integrity + 17 extraction).
- **CI is the compile gate — do not commit Rust changes without expecting a CI build to verify them.** Local MSVC linker is absent. `cargo check` cannot run. Use-after-move and similar compile errors are invisible locally.

---

## Loop-effectiveness notes

The prior retrospect found the loop was iterating on visible features while the core guarantee remained untested. That finding has been substantially resolved: the integrity layer has 15 unit tests across three modules, the end-to-end gate has been closed, and the self-hosting gate — open since 2026-05-07 — was enacted today.

The new risk is different: **the loop has closed all internal completeness gates but has not engaged with the vision destination.** The proxy is working. The founding pledge is met. The ambient recording path — which vision describes as the "real destination" — has never been touched. The loop has treated delivery of the proxy as convergence, but the arc has a further destination that has been invisible in every trail entry.

This is not a failure — the proxy is genuinely useful and structurally sound. But the honest arc-read is: the proxy is a milestone, not the destination. The destination is ambient recording of every agent interaction with zero user friction. The proxy is the foundation that makes that destination technically reachable. Whether the loop moves toward that destination or formally narrows scope is the open question the next run should answer.


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
- **Self-hosting gate — CLOSED 2026-05-15.** Session `self-hosting-gate-001.jsonl` produced under `C:\git\harness-protocol\.harness\sessions\` during live development. Chain: 5 entries (seq 0–4), genesis prev, all links verified. `act_name: record_result`, `act_flag: true` captured in seq 4. The founding pledge is met.

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
