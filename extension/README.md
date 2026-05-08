# Harness Protocol — VS Code Extension

A [GitHub Copilot Chat](https://marketplace.visualstudio.com/items?itemName=GitHub.copilot-chat) participant that writes a **fail-closed cryptographic ledger** of every model interaction — so nothing is lost and every decision is traceable.

Just install and type `@harness` in Copilot Chat. No Python. No API keys. No configuration required.

## Install

1. Download `harness-protocol-x.x.x.vsix` from the [releases page](https://github.com/ntholm86/harness-protocol/releases)
2. Open VS Code → Extensions (`Ctrl+Shift+X`) → `···` → **Install from VSIX**
3. Open Copilot Chat and type `@harness`

## Features

- **`@harness` chat participant** — works with any model you have selected in Copilot Chat
- **Ledger view** in the explorer sidebar — one entry per session, with hash-chain verification icons
- **Fail-closed** — if the ledger cannot be written, the session is blocked

## Build from source

```powershell
cd extension
npm install
npm run compile
npx vsce package --no-dependencies
```
