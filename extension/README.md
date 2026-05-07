# Harness Protocol — VS Code Extension

The UX wrapper for the [Harness Protocol](https://github.com/ntholm86/harness-protocol/blob/master/SPEC.md). It manages the local
MITM proxy, injects `OPENAI_BASE_URL` / `ANTHROPIC_BASE_URL` into your
integrated terminals, and renders the cryptographic ledger live in the
explorer sidebar.

`@harness` in Copilot Chat works with **whatever model you have selected** — no API key required.

## Features
- **Auto-start** the local proxy when a workspace opens.
- **Status bar** indicator showing whether the harness is engaged.
- **Ledger view** in the explorer — one entry per LLM call, with hash-chain
  verification icons.
- **Environment injection** so any process you launch from the integrated
  terminal automatically routes through the harness.

## Build
```powershell
cd extension
npm install
npm run compile
```

Then press `F5` to launch an Extension Development Host.
