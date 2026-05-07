# Vision — harness-protocol

_Operator-held. Updated by Vision run 2026-05-07 (session: post-extension-milestone)._

## The Destination
This repository is the practical **delivery mechanism** for the Autonomous Agent Principles defined in the manifesto repository. The goal is to build the "Immune System against Revisionism" so any developer can adopt Architectural Constraint with zero friction.

## Two Peer Delivery Paths
Both paths implement the same ledger contract and are governed by the same SPEC.md. They differ only in delivery context.

1. **The VS Code Extension (primary path for VS Code + Copilot Chat users):** A chat participant (`@harness`) that works with whatever model is selected in GitHub Copilot Chat — no API key, no proxy required. Installs from the VS Code Marketplace or a `.vsix` file. The end-state is: colleague opens VS Code, installs one file, `@harness` just works.

2. **The Proxy Server (path for scripts, agents, and non-VS-Code clients):** A lightweight local API gateway (Python/FastAPI) that acts as a true man-in-the-middle. Requires zero custom client libraries — developers simply override their `base_url`. It intercepts traffic, forks the stream to `.harness/sessions`, and returns clean data to the unaware client. Natively supports OpenAI and Anthropic schemas; dynamically routes to any lab via `x-harness-upstream`.

## The Protocol Specification
SPEC.md is the single authoritative document for **both** paths. It must formally specify: the append-only ledger format, the JSON payload schema (separating thoughts from actions), the JCS canonicalization and SHA-256 hash-chain rules, the fsync/fail-closed contract, and the direct-write path (used by the extension). Any future harness client — JetBrains, CLI, Rust — must be implementable from SPEC.md alone.

## The Method & Self-Hosting
We build this using the autonomous-agent-skills suite. The self-hosting pledge — using `@harness` to develop harness — is active. The proxy MVP is complete. The next non-trivial development session must run inside `@harness`.

## What is still open
- Packaging the extension as a `.vsix` and submitting to the VS Code Marketplace.
- Handling server-sent events (`stream: true`) within the fail-closed paradigm (proxy path).
- Expanding SPEC.md to formally cover the direct-write path with the same rigour as the proxy path.
- Tests for `ledgerWriter.ts` (the extension has 0 tests; the proxy has 10).
- Upgrading from the Python MVP to a compiled language (Rust) if performance eventually demands it.
