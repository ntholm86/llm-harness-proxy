# Harness Protocol

**The Reference Implementation for the Autonomous Agent Principles**

This repository contains the practical, mechanical implementation of the "Immune System against Revisionism" defined in the [manifesto](../manifesto/README.md). Where the manifesto dictates pure theory, this repository provides the physical architecture requiring AI systems to operate under **Architectural Constraint** rather than relying on fragile "Behavioral Alignment."

## The Three-Tier Architecture

To achieve widespread adoption—from single local developers to enterprise CI/CD pipelines—the Harness Protocol is designed across three interoperable layers:

### 1. The Protocol Specification
A strict, language-agnostic standard defining exactly how an environment must intercept and log an LLM's raw cognitive exhaust to achieve true **Observable Autonomy**. It dictates:
* The JSON schema required from the LLM (separating `internal_reasoning` from `action`).
* The mandatory **stream fork**: the environment must write the reasoning to an append-only ledger *before* evaluating the action.

### 2. The Proxy Binary (The Core Engine)
A standalone, high-performance API Gateway (intended for Rust or Go). 
* Developers point their AI tools (Python scripts, JS web apps, curl commands) to this local proxy instead of directly to OpenAI/Anthropic APIs.
* The proxy intercepts the network response holding the tokens, strips the `internal_reasoning`, writes it directly to the local `.trail/log.md` immutable ledger, and passes the clean output back to the original application.
* **Benefits:** 100% language-agnostic, zero friction, highly scalable.

### 3. The VS Code Extension (The Trojan Horse)
A developer-friendly UX wrapper around the Proxy Binary.
* Colleagues can install the extension in one click. It quietly runs the proxy in the background.
* It provides a real-time UI sidebar to watch the Immutable Ledger populate during AI operations.
* **Benefits:** Instant viral adoption, local visualization of the audit trail, no complex environment setup required.

---

*Note: This repository is the "Practice". The "Theory" remains purely domain-agnostic and lives in the [`manifesto`](../manifesto) repository.*