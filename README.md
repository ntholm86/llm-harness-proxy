# Harness Protocol

**The Reference Implementation for the Autonomous Agent Principles**

This repository contains the practical, mechanical implementation of the "Immune System against Revisionism" defined in the [manifesto](../manifesto/README.md). Where the manifesto dictates pure theory, this repository provides the physical architecture requiring AI systems to operate under **Architectural Constraint** rather than relying on fragile "Behavioral Alignment."

## Architecture

### The Proxy (`proxy-rust/`)

A standalone Rust binary — the actual harness. It sits **outside** any AI agent:

- Intercepts LLM API traffic by acting as a `base_url` override (MITM proxy on `127.0.0.1:8080`).
- Writes a cryptographically hash-chained ledger entry (RFC 8785 JCS + SHA-256) to `.harness/sessions/` **before** releasing the response.
- Fail-closed: if the write or `fsync` fails, the response is not forwarded.
- Single static binary. No runtime dependencies. Language-agnostic.

**Usage:** set your LLM client's `base_url` to `http://127.0.0.1:8080`. The proxy forwards to the real API.

**Build:** pre-built binaries are produced by GitHub Actions (`.github/workflows/build-proxy.yml`) for Windows x86_64, Linux x86_64, and macOS aarch64.

## Protocol

The ledger format is specified in [SPEC.md](./SPEC.md). Conformance tier: **L2** (RFC 8785 canonicalization, SHA-256 hash chain, fail-closed write semantics).

---

*Note: This repository is the "Practice". The "Theory" remains purely domain-agnostic and lives in the [`manifesto`](../manifesto) repository.*