
---
## [2026-05-08] CI green — Rust proxy builds on Windows and Linux

**Target:** `proxy-rust/` (GitHub Actions build-proxy workflow)

**Root causes of CI failures:**
1. Branch filter said `main`; repo uses `master`. Fixed to `["master","main"]`.
2. `reqwest 0.12` with `rustls-tls` feature pulled in `ring 0.17` → NASM required. Switched to `native-tls` (Schannel on Windows, OpenSSL on Linux). No NASM needed.
3. Source files `jcs.rs`, `ledger.rs`, `main.rs` were saved in Windows-1252 encoding (byte `0x97` = em-dash). Rust requires valid UTF-8. Re-encoded all three files as UTF-8.

**Result:** Build #5 — both `Build (Windows x86_64)` and `Build (Linux x86_64)` passed. Artifacts `harness-proxy.exe` and `harness-proxy` produced.

---
## [2026-05-08] Dumb-reader extension built, Rust proxy source committed, docs updated

**Target:** `harness-protocol` (whole repo)
**Operator ask:** "please continue" / "please do these things" — complete the new architecture.

**Actions taken:**
- `extension/` created from scratch: `package.json` (no `chatParticipants`), `src/extension.ts` (activate only — registers tree view + refresh), `src/harnessRoot.ts` (config resolution), `src/ledgerProvider.ts` (reads JSONL, shows tree: session → entry → fields). Zero recording logic.
- `proxy-rust/` committed (was untracked despite prior belief — git history confirmed it was never in a commit). `proxy-rust/target/` added to `.gitignore`.
- `SPEC.md` updated: added §15 (Reference implementations) describing both `proxy-rust/` and `extension/`.
- `README.md` rewritten: removed stale three-tier description referencing `.trail/log.md` and "extension runs proxy in background"; replaced with accurate two-component architecture.
- Pushed two commits: `a03acbd` (extension), `12bafda` (proxy-rust + gitignore), `f97e484` (docs).

**Open:**
- CI build result not yet confirmed (repo appears private; GitHub Actions page returned 404 over unauthenticated HTTP fetch). Next step: check `https://github.com/ntholm86/LLM-harness-protocol/actions` in browser.
- End-to-end test: download built binary, set `HARNESS_ROOT`, point a client at `http://127.0.0.1:8080`, verify `.harness/sessions/*.jsonl` is written and chain verifies.

---
## [2026-05-08] [!REVERSAL] Architectural reset — delete extension + Python proxy, build Rust proxy + CI
**Target:** `harness-protocol` (whole repo)
**Operator ask:** Delete VS Code extension, delete Python proxy, set up GitHub Actions for Rust proxy.

**[!REVERSAL] Root cause of deletion:**
`chatParticipant.ts` violated the founding principle of the harness: *the recorder must sit outside the agent*. The extension was both the agent responding to the user AND the recorder writing to the ledger. Fail-closed was structurally impossible — if the extension crashed mid-response the response was already delivered. Tool calls were discarded; only the final reply was stored in `reason`. The ledger did not record reasoning, it recorded output. Observable Autonomy was not achieved.

**[!DECISION]** Delete both the VS Code extension and the Python proxy. The correct architecture:
- **Rust proxy** (`proxy-rust/`) — dumb HTTP gate outside the agent. Intercepts `/v1/chat/completions` and `/v1/messages`. Writes ledger entry (JCS SHA-256 hash chain, ULID session key, fsync fail-closed) BEFORE forwarding response. Response is discarded if ledger write fails.
- **VS Code extension** (to be built) — dumb reader only. Reads `.harness/sessions/*.jsonl`. No recording logic whatsoever.

**Actions taken:**
- `git rm -rf extension/` — 13 source files deleted
- `git rm -r proxy/` — Python proxy deleted
- `.github/workflows/build-proxy.yml` created — builds `harness-proxy.exe` (Windows) and `harness-proxy` (Linux) as release artifacts via GitHub Actions
- `git remote` updated to renamed repo: `https://github.com/ntholm86/LLM-harness-protocol.git`
- Committed as `73dbef3`: "remove: delete VS Code extension and Python proxy"

**State after this entry:**
- `proxy-rust/` source is complete (Cargo.toml, main.rs, ledger.rs, jcs.rs, ulid.rs)
- Rust proxy cannot compile locally (no `link.exe`) — CI will be first compile verification
- New dumb-reader VS Code extension: not yet started
- SPEC.md and `.trail/vision.md` reflect correct architecture

---
## [2026-05-08] Improve (Kaizen run): fix broken verifyChain + unify LedgerEntry — v0.1.12
**Target:** `extension/src/ledgerProvider.ts`
**Operator ask:** "Run the Kaizen skill on the harness-protocol repo."

**Lenses applied:**
- *Inconsistency:* `LedgerEntry` interface declared in both `ledgerWriter.ts` and `ledgerProvider.ts` — identical at time of writing, silent divergence risk over time.
- *Correctness (dominant finding):* `verifyChain` used `sha256(JSON.stringify(e))` to recompute expected prev-hashes; `appendEntry` writes `sha256(JCS(e))`. JSON.stringify is not deterministic; JCS canonicalizes key order. The two algorithms produce different output for the same entry. Result: `verifyChain` returns `false` for every entry after the first in any multi-entry chain. The command `harness.verifyChain` silently lied about chain integrity. The comment in the code admitted this: *"We don't have JCS in TS here; the chain check uses the recorded prev for now."*
- *Waste:* `sha256OfEntry` function existed only to perpetuate broken verification. `crypto` import in `ledgerProvider.ts` was only used by that function.

**[!DECISION]** Single change: fix `verifyChain` to use `hashEntry` from `ledgerWriter.ts` (JCS-based). Remove `sha256OfEntry`, remove duplicate `LedgerEntry` interface, remove `crypto` import. `LedgerEntry` is now sourced exclusively from `ledgerWriter.ts` and re-exported by `ledgerProvider.ts` for backward compatibility.

**Prediction:** `verifyChain` returns `true` for every entry in a valid chain. `harness.verifyChain` command reports honest integrity. No callers break — neither `extension.ts` nor `chatParticipant.ts` imports `LedgerEntry` by name from `ledgerProvider`. Compile is clean.

**Verification:** `tsc -p . --noEmit` — zero errors. `vsce package` — 10 files, 14.9 KB, zero warnings. `harness-protocol-0.1.12.vsix` produced. Package.json bumped from 0.1.11 to 0.1.12 (the 0.1.11→0.1.12 bump was deferred from the prior harnessRoot.ts session; the filename mismatch is now resolved).

**Reflection:**
- *Model claim:* The extension's write path and verify path now share a single hash function via a single source of truth for both the ledger format (`ledgerWriter.ts`) and the root resolution (`harnessRoot.ts`). The `harness.verifyChain` command can now be trusted.
- *Blind spot:* Correctness is compiler-verified, not runtime-verified. `verifyChain` has not been run against a real session file in a live install. The adversarial contract (torn writes, concurrent sessions, disk-full) remains untested with 0 automated tests for the TypeScript ledger path.
- *Imagined reader pushback:* "Why was this caught in a Kaizen run and not when verifyChain was written?" No tests, and the per-iteration trail discipline was absent — the comment was left as tech debt rather than flagged as a broken contract.

**[!REALIZATION]** Three iterations of real work (v0.1.10: ULID 26-char fix; v0.1.11: tool forwarding + session continuity; v0.1.12: harnessRoot.ts shared module) are absent from this trail. The `prev` links are broken — this entry connects to v0.1.9 with a gap in between. Retrospect's diagnosis ("built deeply, recorded shallowly") is confirmed again in the same session. The trail is the evidence chain the harness is meant to protect; if its own development is unrecorded, the credibility claim is hollow.

**[!REALIZATION]** `package.json` was at 0.1.11 while the harnessRoot.ts VSIX was named 0.1.12 via the `-o` flag override. The version in the manifest and the file on disk were inconsistent. Fixed as part of this entry — `package.json` is now 0.1.12, matching the VSIX filename.

---
## [2026-05-07] Improve: .vsix packaging — first distributable build
**Target:** `extension/` packaging via `@vscode/vsce`

**Findings:**
- `package.json` missing `repository` field → vsce couldn't resolve relative link `../SPEC.md` in README → fatal error.
- `README.md` linked to `../SPEC.md` — broken outside the monorepo context.
- No `LICENSE` file → warning, interactive prompt required.

**Changes:**
- Added `repository` + `license` fields to `package.json`.
- Fixed README link to absolute GitHub URL.
- Added `extension/LICENSE` (MIT).

**Prediction:** Clean `vsce package` with zero warnings, producing `harness-protocol-0.1.0.vsix`.

**Outcome:** `harness-protocol-0.1.0.vsix` — 10 files, 15.24 KB, zero warnings. Prediction held.

**[!DECISION]** Used absolute GitHub URL in README rather than copying SPEC.md into the extension folder — avoids content duplication and keeps SPEC.md as the single source.

**Reflection:**
- *Model claim:* The extension is now distributable. A colleague can install `harness-protocol-0.1.0.vsix` directly. The marketplace publish step is the only remaining gap to the vision end-state.
- *Blind spot:* The `.vsix` hasn't been installed and tested on a clean machine. It compiles and packages correctly but live activation behaviour is unverified.
- *Imagined reader pushback:* "The `.vsix` is gitignored — where does a colleague actually get the file?" There's no release artifact or CI producing it yet.

**[!REALIZATION]** The next meaningful gap is either: (a) a GitHub release with the `.vsix` attached, or (b) a Marketplace publish. Both require the GitHub remote to exist first.

---
## [2026-05-07] Improve: Graceful proxy failure for .vsix install path
**Target:** `extension/src/proxyController.ts`, `extension/package.json`
**Interpretation:** Run improve on harness-protocol with orientation from vision (marketplace/`.vsix` end-state) and retrospect (built without correctness verification).

**Lenses applied:**
- *Inconsistency:* `harness.setApiKey` declared in `package.json`, no handler in `extension.ts` → Command Palette shows it, clicking gives "command not found."
- *Waste:* Same dead command.
- *Correctness:* `proxyController.start()` spawned python with no `proc.on('error')` handler. ENOENT on a fresh `.vsix` install → unhandled Node.js error event → extension host crash. Plus 5-second `waitForHealth()` hang on every activation.

**Decision:** One change — add pre-flight python existence check + `proc.on('error')` handler to `proxyController.start()`. Remove dead `harness.setApiKey` command.

**[!DECISION]** Pre-flight path abort rather than relying solely on runtime error handling — chosen because it also eliminates the 5-second health-check hang, not just the crash.

**Prediction:** Fresh `.vsix` install with no python venv → clean activation, friendly output message, `@harness` works via `vscode.lm`. No extension host crash. No 5-second hang.

**Verification:** `npx tsc -p .` — clean. Code inspection confirms error event is handled, proc nulled, statusBar updated, context key set correctly.

**Reflection:**
- *Model claim:* The extension's proxy path and chat-participant path are now cleanly decoupled at runtime — the proxy can fail without affecting `@harness`. This is the right shape for the "two peer paths" architecture.
- *Blind spot:* Did not exercise the actual `vscode.lm.selectChatModels()` call path in a live Extension Development Host. The participant logic could fail silently at runtime in ways the TypeScript compiler won't catch.
- *Imagined reader pushback:* "You still haven't actually built the `.vsix` — you've only removed a blocker. The packaging step itself hasn't been tried."

**[!REALIZATION]** The `.vsix` can now be attempted without crashing. The next run should actually run `vsce package` and see what it produces.

---
## [2026-05-07] Vision Run: Post-Extension Milestone
**Hunches formed and questions asked:**

1. *Extension vs proxy as centrepiece* — Is the VS Code extension now the primary path, with the proxy as secondary?
   - Operator response: Both are peers. The extension is the primary path for VS Code/Copilot Chat users because that's what the team uses; the proxy is for scripts and non-VS-Code agents. Same ledger contract, different delivery context.

2. *"Show colleagues" implies installability* — Does this mean install from marketplace / `.vsix`, not "clone + F5"?
   - Operator response: Confirmed. End-state is marketplace or a `.vsix` installer.

3. *SPEC.md as single authority for both paths* — Should SPEC.md cover both write paths so any future client implements against one document?
   - Operator response: Yes.

**What the agent now believes:** Two peer delivery paths, both governed by SPEC.md. Extension = primary for VS Code users; proxy = for scripts/agents. Marketplace/`.vsix` installability is the concrete end-state for colleague distribution. SPEC.md must formally specify the direct-write path.

**What was rejected:** The framing of proxy as centrepiece / extension as "UX wrapper."

**What is still open:** SSE/streaming in fail-closed paradigm; SPEC.md direct-write section; tests for `ledgerWriter.ts`; `.vsix` packaging.

**Actions:** Updated `.trail/vision.md` to reflect confirmed hunches.

---
## [2026-05-07] Retrospect: Post-Extension Milestone Arc-Read
**Scope:** Read the full arc from vision lock through SPEC, proxy MVP, and VS Code extension build. Determine what the project is becoming, where attention has been, and whether the loop has been looking at the right things.

**Arc-claims (falsifiable):**
- The VS Code extension (primary deliverable) has never been committed — 0 git entries for `extension/`.
- The self-hosting pledge in vision has not been enacted at any point in the arc. All development happened outside the harness.
- SPEC.md describes one ledger write path (proxy HTTP). `ledgerWriter.ts` introduces a second (direct TypeScript). The spec and implementation have diverged.
- The trail log had 1 entry for a full session that included an architectural reversal (HTTP+API-key → `vscode.lm`). No `[!REVERSAL]` was recorded.

**[!REALIZATION]** The harness cannot credibly claim to be an immune system against revisionism when its own development history is unrecorded. The credibility gap must be closed before extending features.

**Loop-effectiveness:** High build effectiveness, low record effectiveness. The system works; its provenance does not.

---
## [2026-05-07] Vision Alignment: The Blueprint and the MITM
**Interpretation of the ask:**
The operator requested the formal execution of the ision skill before proceeding, confirming the hypothesis that we must define the Protocol Specification first, design the proxy as an invisible man-in-the-middle, and rush an MVP so we can "self-host" the structural constraint.

**Examination & Decisions:**
- The previous "manual" ision.md generation skipped the operator-confirmation step.
- The operator confirmed all three hunches.
- Decision: Update .trail/vision.md to lock in the starting point (Protocol Spec), the proxy design (zero-friction ase_url override), and the production strategy (self-host ASAP).
- Decision: Resolve the "what is open" questions from the previous draft since we now have consensus on the starting point.

**Actions:**
- Rewrote C:\git\harness-protocol\.trail\vision.md to reflect the newly confirmed priorities.

**Reflection:**
We have successfully transitioned from the theoretical realm of the manifesto repo into the execution realm of harness-protocol. The constraint to "eat our own dog food" is formally established: our race is to build the harness so we can use the harness to finish the harness.

---
## [2026-05-09] Improve: Extension — strip to minimal working core
**Target:** `extension/` — full audit for dead code, stale artefacts, misleading defaults
**Operator intent:** "Finished means someone runs the installer and starts using the extension. No bloat, no dead code."

**Lenses applied:**
- *Waste:* `proxyController.ts` already deleted from `src/` but `proxyController.js.map` remained in `out/` — orphaned artefact shipping inside the `.vsix`.
- *Waste:* `harness.model.name` + `harness.model.upstreamUrl` config keys defined in `package.json` but never read in any source file — deleted.
- *Waste:* `harness.verifyChain` was a stub — only counted files, never called `verifyChain()`. Fixed.
- *Inconsistency:* `autoStart: true` + `injectEnv: true` defaults made the proxy path feel mandatory. Flipped to `false`.
- *Inconsistency:* `ProxyController` wired into activation before the chat participant — proxy failure could block `@harness`. Decoupled.
- *Waste:* README described old proxy-first design. Rewritten to describe actual behaviour.

**[!DECISION]** Single config key retained: `harness.root`. Everything else removed as dead weight.

**[!DECISION]** `ProxyController` is now unreachable from `extension.ts`. The "harness off" status bar it owned is gone entirely from the installed extension.

**Prediction:** Fresh install of `0.1.9.vsix` -> no Python startup, no status bar, no output channel noise. `@harness` responds immediately in Copilot Chat.

**Actions:**
- Deleted `harness.model.name` + `harness.model.upstreamUrl` from `package.json`
- Fixed `harness.verifyChain` to call `verifyChain()` + `readEntries()` properly
- Flipped `autoStart` + `injectEnv` defaults to `false`
- Decoupled `harnessRoot` resolution from `ProxyController` in `extension.ts`
- Rewrote `README.md` with correct install guide and feature description
- Deleted orphaned `out/proxyController.js.map`
- Rebuilt `harness-protocol-0.1.9.vsix` — 9 files, 14.04 KB, zero warnings

**Verification:** `tsc -p ./` clean. `vsce package` — 9 files, 14.04 KB. `src/` — 4 files only, all live.

**Reflection:**
- *Model claim:* The extension is now genuinely minimal. Every file in `src/` is used. Every default is safe for a fresh install. The remaining gap: verify ledger writes end-to-end on a live install.
- *Blind spot:* `ledgerWriter.ts` fail-closed contract never exercised under adversarial conditions (disk full, torn write, concurrent sessions).
- *Imagined reader pushback:* "You still haven't verified that `@harness` actually writes a ledger entry on a live install of `0.1.9.vsix`." Correct — that is the next falsifiable test.

**[!REALIZATION]** The "harness off" status bar that confused the operator was owned entirely by `ProxyController`. With `ProxyController` absent from compiled output, the status bar is gone. The confusion was a direct symptom of dead code shipping in the installed extension.

---
## [2026-05-19] Vision: foundational violation surfaced — architecture corrected
**Target:** harness-protocol — full project arc
**Operator ask:** "Run the vision skill. Make sure this is recorded."

**Signal gathered:**
- Operator confirmed: Observable Autonomy means every autonomous action, thought, reasoning — everything — is logged. Word-by-word, thought-by-thought. Like a git history for its domain.
- Operator confirmed: the harness must sit **outside** the agent — the agent is structurally incapable of receiving a response unless the ledger is written first.
- Operator confirmed: if the chatParticipant is deciding what to record, we are violating the principle this exact application is trying to solve.
- Operator confirmed: the proxy was the correct architecture all along. The extension was a pragmatic detour that introduced the violation.
- Operator confirmed: ambient recording (silent always-on, no @harness invocation) is the real destination — not possible with current VS Code APIs, accepted for v1.

**[!VIOLATION CONFIRMED]** chatParticipant.ts is the agent AND the recorder in the same process. The reason field in every ledger entry is the final reply text, not reasoning. Tool calls, intermediate decisions, and thinking steps are discarded before the ledger sees them. The fail-closed guarantee is broken by architecture.

**[!DECISION]** Vision.md updated with:
1. The foundational principle stated explicitly — dumb pipe, append-only, zero decisions about what matters
2. Roles clarified — proxy is the enforcer, extension is the viewer
3. Corrected priority order — fix proxy first, then remove recording logic from chatParticipant.ts
4. Ambient recording noted as future milestone

**Reflection:**
- The proxy was abandoned because of Python dependency friction. But in abandoning it we silently abandoned the core guarantee. The extension was celebrated as working while the founding principle was being violated.
- The correct next move is: fix the proxy (ModuleNotFoundError: No module named harness_proxy), then reduce the extension to viewer-only.
- The chatParticipant.ts recorder is not just incomplete — it is architecturally wrong. Improving it would deepen the violation.

**What is still open:**
1. Fix proxy — harness_proxy module not found
2. Remove recording logic from chatParticipant.ts — extension becomes viewer only
3. Full stream capture in proxy — every token verbatim
4. Ambient recording — future VS Code middleware API
