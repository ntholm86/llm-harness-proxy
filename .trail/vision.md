# Vision — harness-protocol

_Operator-held. Confirmed by Vision alignment 2026-05-07._

## The Destination
This repository is the practical **delivery mechanism** for the Autonomous Agent Principles defined in the manifesto repository. The goal is to build the "Immune System against Revisionism" so any developer can adopt Architectural Constraint with zero friction.

## The Architecture Constraints
1. **The Protocol Specification (Start Here):** A strict, language-agnostic mathematical definition of the append-only ledger, the JSON payload schema (separating thoughts from actions), and the stream-forking rules. This must be established first as the blueprint for the code.
2. **The Proxy Binary (Invisible MITM):** A high-performance local API gateway (Rust/Go) that acts as a true man-in-the-middle. It requires zero custom client libraries—developers simply override their application's ase_url. It intercepts the traffic, forks the stream to .trail/log.md, and returns clean data to the unaware LLM/client.
3. **The VS Code Extension:** A UX wrapper that quietly runs the proxy and visualizes the immutable ledger as it is written in real-time.

## The Method & Self-Hosting
We will build this using the utonomous-agent-skills suite itself. 
**The immediate strategic goal** is to reach a "minimum viable proxy" (MVP) as fast as possible. Once the MVP proxy exists, we will immediately pivot to using the proxy to build the rest of itself, establishing true Architectural Constraint and ending our reliance on unconstrained chat.

## What is still open
- The precise JSON layout for the Protocol Specification (e.g., standardizing the names of the internal_reasoning and ction fields).
- Whether we use Rust or Go for the Proxy Binary (though Rust is currently the strong preference for performance).
