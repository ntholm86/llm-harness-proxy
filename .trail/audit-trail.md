
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
