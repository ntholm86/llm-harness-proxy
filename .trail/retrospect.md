# retrospect.md — harness-protocol

_Last updated: 2026-05-07 (run: post-extension-milestone)_

## Current claims

**1. The primary deliverable (VS Code extension) has never been committed.**
The proxy received 3 commits and is traceable. The extension — 5 TypeScript source files including the `@harness` chat participant, the ledger tree view, the proxy controller, and the fail-closed TypeScript ledger writer — is entirely untracked. If the repo is cloned, the deliverable that the vision describes as the "UX wrapper that quietly runs the proxy and visualizes the immutable ledger" does not exist. A future run can falsify this by finding a commit containing `extension/src/`.

**2. The self-hosting pledge has not been enacted at any point in the arc.**
`vision.md` states the immediate strategic goal as: "Once the MVP proxy exists, we will immediately pivot to using the proxy to build the rest of itself, establishing true Architectural Constraint." The proxy reached MVP status in commit `1d95061`. Every line of code since then — all of `extension/` — was written outside the harness. The `.harness/sessions/` directory contains 3 ledger entries from test calls only, none from actual development sessions. A future run can falsify this by finding a session JSONL that records a development interaction.

**3. SPEC.md describes one write path; two now exist.**
The specification defines the ledger as written by the proxy intercepting HTTP traffic. `ledgerWriter.ts` introduces a second write path: direct TypeScript calls from the VS Code extension for `vscode.lm` interactions that never touch HTTP. The SPEC and the implementation have diverged. A future run can falsify this by finding a SPEC section covering the direct-write path with the same formal rigour as the proxy path.

**4. The trail has not recorded its own reversals.**
The arc contains one genuine architectural reversal: the chat participant was initially built using HTTP fetch + an API key, then was abandoned for `vscode.lm` after the key had zero credit. This reversal changed the extension's security model, its token-cost model, and its dependency on the proxy. The trail log has no entry for any of this — no `[!REVERSAL]`, no `[!REALIZATION]`. The trail is perfectly green for a project that changed architectural direction mid-session. This is a structural failure of Observable Autonomy applied to the harness's own development.

**5. The loop has built deeply but recorded shallowly.**
Attention has been concentrated on building (proxy internals, TypeScript type correctness, fail-closed semantics, `vscode.lm` API mechanics) and light on trail maintenance, spec alignment, and git hygiene. The result is a working system with weak provenance — the artefact the harness claims to provide to others.

---

## What the next runs should test

1. **Commit the extension.** The extension is the primary deliverable. Until it is committed it cannot be self-hosted, distributed, or iterated on with trail integrity. First action: `git add extension/ .vscode/ ; git commit`.

2. **Update SPEC.md to cover the direct-write path.** Add a section (§ Direct Write Path) that formally specifies `ledgerWriter.ts`'s contract: input canonicalization, entry format, fsync requirement, session file location. The proxy path and the extension path must share one spec.

3. **Start a self-hosted session.** Use `@harness` in VS Code to drive the next non-trivial change to harness-protocol itself. This is the concrete enactment of the self-hosting pledge. Until this happens the vision is unmet regardless of what is built.

4. **Write a trail entry for the architectural reversal.** Even retroactively, the `[!REVERSAL]` from HTTP+API-key to `vscode.lm` should be on record — it changed the security model, removed the external dependency, and is the reason the extension works without configuration.

5. **Verify `ledgerWriter.ts` under adversarial conditions.** The TypeScript ledger writer has compiled clean but has never been tested against the actual fail-closed contract (torn writes, concurrent sessions, disk-full). The proxy's equivalent logic has 10 passing tests. The extension's equivalent has 0.

---

## Active operational rules

- **Never commit proxy changes without also checking extension build.** The two share the ledger format; a schema change in one must update both.
- **Git status before trail write.** If `git status` shows untracked files in a directory the trail claims is built, the trail is lying.
- **The self-hosting gate:** Before declaring any feature "done," ask: has `@harness` been used to build or review it? If no, it is not self-hosted, and vision is not met.
- **Architectural reversals require a trail entry on the same day.** Do not let a reversal pass without a `[!REVERSAL]` marker. The trail's value is proportional to its honesty about failures.

---

## Loop-effectiveness notes

The loop has been highly effective at *building* and ineffective at *recording*. Every system-level property the harness is meant to provide to others (immutable trail, fail-closed writes, chain of custody) is absent from the harness's own development history. This is not a quality problem with the artefacts — it is a credibility problem with the project. The harness cannot claim to be an "immune system against revisionism" if its own development is unrecorded. The next phase must close this gap before extending features further.
