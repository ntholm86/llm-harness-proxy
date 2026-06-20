
## 2026-05-28 — vision-to-destination-rename

- target: harness-protocol
- operator: Nils Holmager
- agent: GitHub Copilot (Claude Opus 4.7 via vertex)
- skill: improve (intent at step 1, trail at step 7)
- session-file: (fleet sweep coordinated from autonomous-agent-skills repo; see that repo's .trail/sessions/2026-05-28-rename-vision-to-destination.md and audit-trail entry of the same date for cross-cutting rationale, rejected alternatives, and reversals)
- fidelity: reconstructed
- outcome: artifact `.trail/vision.md` renamed to `.trail/destination.md` to match the renamed Destination skill (was Vision; now at `destination/SKILL.md` v2.0.0 in the skills suite, commit e3d1577). H1 header updated to match; no other content in destination.md was modified — it remains operator-held.
- delta: artifact filename only; skill behaviour unchanged.

### Interpretation of the ask

Operator asked the skills-suite agent to find every repo carrying the legacy `.trail/vision.md` and migrate it to the canonical filename so the read-destination-then-fall-back-to-vision rule in `destination/SKILL.md` v2.0.0 stops being load-bearing across the active repos. Eight repos were found. This entry records the migration for **harness-protocol**.

### Decision

[!DECISION] Run the mechanical migration in harness-protocol: `git mv .trail/vision.md .trail/destination.md`, update the H1 header line only, leave the rest of the file untouched (operator-held content per the vision-management discipline), append this entry, regenerate derived artifacts, commit only the migration-related files, push.

Rejected alternatives (recorded in the skills-suite entry, not duplicated here): hard-rename without a fallback period (would have broken consumers), keep the legacy filename forever (permanent skill/artifact name mismatch), and the two sibling skill renames Retrospect→Plan and Improve→Execute (both would have imported PM vocabulary that contradicts what each skill produces).

### Prediction

Commit lands cleanly. Pre-existing uncommitted WIP (if any) is untouched. The next Destination, Retrospect, or Improve run on harness-protocol reads `.trail/destination.md` directly without invoking the fallback path.

### Action

1. `git mv .trail/vision.md .trail/destination.md`.
2. Updated the H1 header via UTF-8-safe .NET `File.ReadAllText` / `File.WriteAllText` to avoid the PowerShell 5.1 Get-Content/Set-Content mojibake on em-dashes (logged in skills-suite userMemory `append-only-trails.md`).
3. Appended this trail entry via `Add-Content -Encoding UTF8` (append-only rule).
4. Regenerated `.trail/history.md` and `.trail/learning.md` via the skills-suite `record.py` invoked with this repo as cwd.
5. Staged and committed only the migration-related files (`.trail/destination.md`, `.trail/audit-trail.md`, `.trail/history.md`, `.trail/learning.md`). Any pre-existing uncommitted WIP in harness-protocol was left in the working tree untouched. Pushed.

### Reflection

**Falsifiable model-claim:** harness-protocol's operator-held destination now lives at the canonical filename. A future agent does not need the legacy-fallback path to read it. If a future entry in this trail references reading `.trail/vision.md`, something has regressed.

**Named blind spot:** this migration was mechanical and did not evaluate whether the *content* of harness-protocol's destination is still accurate. A stale destination is a different problem from a stale filename; this run fixed only the filename.

**Imagined-reader pushback:** "You touched my repo without doing the work I had open in it." Counter: the rename is the minimum needed to drop the deprecation clock attached to the legacy filename, the only edit inside `destination.md` was the H1 line (the suffix and the rest of the operator content were preserved verbatim), the commit only stages the four migration-related files, and any pre-existing uncommitted WIP remains in the working tree exactly as it was.

**Across-trail trigger evaluation:**

- *Recurring finding-class:* not fired — first fleet rename in this repo's trail; no pattern.
- *About to declare silence:* not fired — substantive action taken.
- *Contradicts prior [!REALIZATION]:* not fired — no prior realisation in this repo argued for or against the artifact filename.
- *Operator explicitly asked:* FIRED — operator explicitly asked for the migration after the skill rename was committed in the skills suite.

### Candidate Next Moves

1. **Run the Destination skill on harness-protocol** to check whether the operator-held destination is still current; this migration only fixed the filename, not the substance.
2. **Run Retrospect on harness-protocol's trail** — the migration changes nothing structural, but a Retrospect pass would surface any arc-level claim that had become stale while attention was elsewhere.
3. **Confirm no other tooling in harness-protocol still hard-codes the path `.trail/vision.md`** (e.g., a checked-in workflow, a script, a doc) — `record.py` and the skill prose already read the new name, but harness-protocol-local tooling has not been audited in this run.

## 2026-06-19 — ARF probe dataset administered through harness in production

**Skill:** trail
**Target:** harness-protocol (LLM Harness Protocol v2.0.0)
**Operator:** nils-holmager
**Model:** Claude Opus 4.5 (Copilot)

### What this session accomplished

The harness was used in production for the first time as the measurement substrate for the ARF probe dataset. This is the practical validation of the claim that the harness "lifts ARF from cannot be measured to can be measured."

**Evidence:**

Six harness sessions created across three probes:

| Probe | Case A Session | Case B Session | Result |
|-------|----------------|----------------|--------|
| code-review-offline-constraint | 01KVEYTRBX5RHRAZX0QYYR2NJN | 01KVEYV31SRY0KW4V48BBD5BR8 | PASS |
| instruction-stakeholder-shift | 01KVF0YYA01S9AKWVRTTQWFSAX | 01KVF0ZA7J8QEDG7F0EY623TQY | INDETERMINATE |
| ambiguous-deadline-handling | 01KVF1F4DJHQTJWTT3S2NWVE00 | 01KVF1F798Z05RVHHMFVESN7D6 | PASS |

All sessions: C:\git\harness-protocol\.harness\sessions\<ULID>.jsonl
Model: claude-haiku-4-5-20251001 via Anthropic API
Endpoint: 127.0.0.1:8474 (harness proxy)
Harness version: 2.0.0

The HARNESS_DEFAULT_SESSION feature (added this session's predecessor, commit 6ff743f) was NOT required — each probe run created fresh sessions per call. Session independence was verified by the runner.

**What the production run confirmed:**

- Anthropic path (/v1/messages) works correctly — request/response captured, ULID assigned, JSONL written
- Session independence: each 	ools/arf-runner.py invocation creates independent sessions
- Ledger files are readable post-run for scoring (no corruption, correct encoding)
- Accept-Encoding: identity header required to prevent gzip stripping (documented in runner)

**What this establishes for the manifesto:**

The harness removes the instrument-inheritance problem for ARF probes. Results are in C:\git\manifesto\probes\results\ and are cited in the manifesto as evidence (PROOF.md Reference Implementation B now links to the initial dataset).

### Trail note

The harness-protocol trail has been sparse. This is the second entry since the vision.md rename (2026-05-28). The HARNESS_DEFAULT_SESSION feature (commit 6ff743f) and the production binary deployment and self-hosting work done in prior sessions were not trailed. This is a known gap — retroactive reconstruction is not reliable; only sessions with direct evidence are trailed here.

### Candidate Next Moves

1. **Cross-model probe run when additional API access is available.** The harness is ready; the runner supports any Anthropic/OpenAI model via env vars. Running the same 3 probes against a second model family would produce the first cross-model comparison.
2. **Zenodo upload for manifesto v2.3.0.** Operator-scope — GitHub Release from the v2.3.0 tag triggers this.
3. **Trail gap audit.** The HARNESS_DEFAULT_SESSION feature and self-hosting gate work are untrailed. Consider whether a reconstruction entry is warranted.

## 2026-06-19 — AAT compliance mapping document created

**Skill:** improve
**Target:** harness-protocol (documentation layer)
**Operator:** nils-holmager
**Model:** Claude Opus 4.5 (Copilot)

### Interpretation of the ask

Operator asked for strategic positioning work after competitive landscape analysis against 9 autonomy-governance frameworks. The analysis identified three priorities:

1. AAT compliance mapping (low effort, high interop value)
2. AAS-1 assertion mapping (medium effort)
3. Skills-suite deployment case study (medium effort)

This session executes priority 1.

### Decision

[!DECISION] Create `docs/AAT-MAPPING.md` — a field-by-field mapping document showing how the Harness Protocol relates to the IETF Agent Audit Trail specification (draft-sharif-agent-audit-trail-00). This is documentation work, not protocol change.

### Prediction

The document will answer the question "how does your format relate to the IETF draft?" without requiring readers to cross-reference both specs. It will **not** make the harness AAT-compliant (that would require adding missing fields like `action_type`, `outcome`, `trust_level`); it documents the relationship as-is.

### Action

1. Read both specs (harness SPEC.md and draft-sharif-agent-audit-trail-00 via IETF datatracker)
2. Created `docs/AAT-MAPPING.md` with:
   - Field mapping tables (Harness → AAT and AAT → Harness)
   - Structural mapping (session lifecycle, hash chain)
   - Taxonomy gap analysis (harness `act` is untyped; AAT has `action_type` vocabulary)
   - What harness adds beyond AAT (enforcement, streaming, full reasoning capture, MITM deployment)
   - What harness omits from AAT (agent identity, action taxonomy, outcome, trust levels, signatures, tombstones)
   - Conversion example
   - Regulatory alignment table (EU AI Act, SOC 2, ISO 42001, PCI DSS)
3. Updated README.md to link the new document under a Documentation section

### Outcome vs prediction

Prediction held. The document explains the relationship without claiming full compliance.

### Reflection

**Falsifiable model-claim:** The harness implements AAT's core cryptographic guarantees (hash chain via SHA-256 + JCS over RFC 8785) and differs primarily in scope (enforcement vs observation, reasoning capture vs reasoning hash, untyped actions vs taxonomy). A future AAT validator that checks only chain integrity would accept harness sessions with trivial field renaming (sid→session_id, seq addressing→record_id chain).

**Named blind spot:** The conversion example in the document assumes `outcome: "success"` and `trust_level: "L1"` because the harness doesn't capture these. A real AAT export tool would need to either omit these fields (if AAT ever relaxes them to optional) or inject placeholders that don't reflect reality.

**Imagined-reader pushback:** "You say harness aligns with AAT but it doesn't implement half the mandatory fields." Counter: the mapping document explicitly calls these out in the "What the Harness Omits from AAT" table. The claim is not compliance; it's that the core cryptographic primitive is identical and the gaps are documented.

**Across-trail trigger evaluation:**
- *Recurring finding-class:* not fired — first interop mapping in this trail
- *About to declare silence:* not fired — substantive artifact created
- *Contradicts prior [!REALIZATION]:* not fired — no prior realization about AAT relationship
- *Operator explicitly asked:* FIRED — operator asked for strategic work after competitive analysis

### Candidate Next Moves

1. **AAS-1 assertion mapping** — map PEA's three principles to AAS-1's audit assertion vocabulary. Second priority from the competitive analysis.
2. **Skills-suite deployment case study** — package the existing trail as an "incident narrative" for institutional credibility. Third priority.
3. **Add `action_type` field to harness** — would increase AAT alignment but is a protocol change (v2 breaking). Deferred pending demand.


---

## 2026-06-20 -- fix: X-Harness-Root per-request header for dynamic session routing

**Trigger:** ai-steward's harness session capture was broken. The proxy's `harness_root` was fixed at startup from `HARNESS_ROOT` env var. ai-steward ran against multiple target repos; each needed sessions written to the target repo's `.trail/sessions/`. The env var override approach never reached the already-running proxy process.

**[!DECISION]** Add `ROOT_HEADER = "x-harness-root"` as a per-request override. The pattern already existed: `X-Harness-Session` and `X-Harness-Upstream` were per-request overrides in the same codebase. `X-Harness-Root` follows the same shape.

**Changes:**
- `const ROOT_HEADER: &str = "x-harness-root"` added alongside existing header constants
- All three handlers (`openai_handler`, `anthropic_handler`, `gemini_handler`) extract the header at the top: if present, use as `root: PathBuf`; else fall back to `state.harness_root`
- Both SSE streaming paths (which previously captured `state.harness_root.clone()` into the async task) now capture the per-request `root` instead
- Both buffered paths pass `&root` to `SessionLedger::append_entry` instead of `&state.harness_root`
- `ROOT_HEADER` added to the strip list in all `send_upstream` calls — it must never leak to upstream APIs

**Invariant preserved:** When `X-Harness-Root` is absent, behaviour is identical to previous — `state.harness_root` used unchanged. No breaking change for existing callers.

**Verification:** `cargo build --release` succeeded. Companion test fixes in ai-steward confirmed 66/66 pass.

**Companion commit in ai-steward:** `anthropic_client(config, harness_root=None)` — adds `X-Harness-Root` header when provided.

---

## 2026-06-20 — [RECONSTRUCTION] HARNESS_DEFAULT_SESSION feature (originally 2026-06-19)

**Note:** This entry reconstructs .harness/trail.md (written 2026-06-19) into the canonical trail.
The original entry was written to .harness/trail.md in error — .harness/ is the proxy session
directory, not the trail directory. Original content follows verbatim.

---

## 2026-06-19 — Add HARNESS_DEFAULT_SESSION for multi-call session continuity

**Skill:** improve
**Target:** harness-protocol proxy-rust/src/main.rs

**Interpretation of ask:** Currently every API call with no x-harness-session header creates a new ULID and a new file. For multi-turn agentic runs, all calls should accumulate in one .jsonl. Mechanism already exists (x-harness-session header), but requires every client to pass it. Add a server-side default so the operator can declare a session at startup.

**Examination:**
- Purpose: the harness is a MITM proxy that ledgers each call. One file per call is correct for ARF probe isolation, but wrong for multi-turn sessions.
- Inconsistency: the session concept already exists (sid field, x-harness-session header, ULID-named files) but has no server-side default.
- The fix: HARNESS_DEFAULT_SESSION env var -> AppState.default_session: Option<String> -> .or_else(|| state.default_session.clone()) in all three handlers.

**[!DECISION]** Env var approach, not a /session/start endpoint. Simpler, no new HTTP surface, operator-controlled.

**Pre-commit prediction:** Three handler lines changed uniformly. No ARF tension: probe runner unaffected (never sets env var). Code compiles once linker is available.

**Actions:**
1. Added default_session: Option<String> to AppState
2. Added HARNESS_DEFAULT_SESSION env var read at startup with info! log
3. Added .or_else(|| state.default_session.clone()) to all three handlers
4. Added proxy-rust/.cargo/config.toml to use rust-lld for msvc target

**[!REVERSAL]** Build blocked: attempted to build with rust-lld, ScopeCppSDK link.exe -- neither worked without the full Windows SDK libs. Stopped before becoming a toolchain rabbit hole. Code change is committed; rebuild requires installing MSVC 'Desktop development with C++' workload via VS Installer.

**Reflection:**
- Model: The harness session concept is now complete at the protocol level: x-harness-session header (caller-controlled continuity), HARNESS_DEFAULT_SESSION env var (operator-controlled default), fresh ULID fallback (isolation default). All three modes compose correctly.
- Blind spot: no test for the new env var path. The existing 33 tests exercise fresh-ULID behavior; a test that sets HARNESS_DEFAULT_SESSION and sends two requests should confirm they land in the same file.
- Reader pushback: the rebuild requirement is a blocker for today's session. The user has to install the C++ workload before the feature is live.

**Across-trail triggers:** None fired.

---

## 2026-06-20 — Trail reconstruction and write-path audit

**Skill:** improve (skills-suite v3.10.0)
**Target:** llm-harness-proxy trail integrity

### Interpretation of the ask

Operator noticed .harness/trail.md -- a trail entry written to the wrong directory in a prior session.
Asked to reconstruct into .trail/audit-trail.md and establish that trail entries always go to the
target repo root .trail/ directory.

### Examination

- **Purpose:** .harness/ is the proxy session directory (JSONL evidence files). Trail entries belong
  in .trail/audit-trail.md. Two trail files existing for one repo is an inconsistency.
- **Write-path audit:**
  - audit-trail.md: improve/trail skill (AI assistant) -- correct
  - retrospect.md: retrospect skill -- correct
  - destination.md: destination skill / operator -- correct
  - history.md, learning.md: skills-suite tools/record.py (generated artifacts) -- correct
  - log.md: legacy format trail (115KB, May 2026) -- legacy, read-only
  - .harness/trail.md: improve skill entry 2026-06-19 -- WRONG LOCATION
- **ai-steward record.py:** hardcodes repo / ".trail" / "audit-trail.md" -- cannot write to wrong
  place unless repo arg is wrong. Structurally sound.

### Decision

[!DECISION] Append .harness/trail.md content into .trail/audit-trail.md as a marked reconstruction
entry. Append an operational rule to destination.md. No deletes -- APPEND-ONLY rule holds.

### Prediction

audit-trail.md gains ~35 lines. destination.md gains ~5 lines. .harness/trail.md unchanged.
The "untrailed HARNESS_DEFAULT_SESSION" note in audit-trail.md is resolved.

### Reflection

*Current model of target:* Trail write paths are now documented and the misplacement is corrected
in the canonical trail. The structural guarantee for ai-steward was already in code; the gap was
AI-assistant discipline only.

*Blind spot:* The operational rule added to destination.md relies on future AI sessions reading
destination.md before writing. Sessions that skip orientation will still make the same mistake.

*Across-trail triggers:*
- Recurring finding-class: not fired -- single misplacement, not a pattern.
- About to declare silence: not fired -- change made.
- Operator explicitly asked: fired.
