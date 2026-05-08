import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { LedgerProvider, readEntries, verifyChain } from './ledgerProvider';
import { registerChatParticipant } from './chatParticipant';

export async function activate(context: vscode.ExtensionContext) {
  // Resolve workspace root — fall back to extension's parent (repo root) so
  // the extension still functions when opened in a multi-root or no-folder
  // Extension Development Host window.
  const ws = vscode.workspace.workspaceFolders?.[0];
  const root = ws?.uri.fsPath ?? path.join(context.extensionPath, '..');

  // ── Core path: resolve harnessRoot directly from config ──────────────────
  // This must NOT depend on ProxyController so @harness works with zero proxy
  // setup. ProxyController is only needed for the optional proxy path.
  const rawRoot = vscode.workspace.getConfiguration('harness').get<string>('root') ?? '.harness';
  const harnessRoot = path.isAbsolute(rawRoot)
    ? rawRoot
    : path.join(root, rawRoot);
  // Ensure the directory exists so ledger writes never fail on first use.
  fs.mkdirSync(harnessRoot, { recursive: true });
  const sessionsDir = path.join(harnessRoot, 'sessions');

  const ledger = new LedgerProvider(sessionsDir);

  // Register the chat participant first — it is the core feature and must
  // activate regardless of proxy state.
  context.subscriptions.push(
    ledger,
    vscode.window.registerTreeDataProvider('harnessLedger', ledger),
    vscode.commands.registerCommand('harness.refresh', () => ledger.refresh()),
    vscode.commands.registerCommand('harness.openSession', (file: string) =>
      vscode.window.showTextDocument(vscode.Uri.file(file)),
    ),
    vscode.commands.registerCommand('harness.verifyChain', async () => {
      const files = await vscode.workspace.findFiles(
        new vscode.RelativePattern(sessionsDir, '*.jsonl'),
      );
      if (files.length === 0) {
        vscode.window.showInformationMessage('Harness: No session files found.');
        return;
      }
      let broken = 0;
      for (const f of files) {
        const entries = readEntries(f.fsPath);
        if (!verifyChain(entries)) { broken++; }
      }
      if (broken === 0) {
        vscode.window.showInformationMessage(
          `Harness: ✅ ${files.length} session file(s) — chain intact.`,
        );
      } else {
        vscode.window.showWarningMessage(
          `Harness: ⚠️ ${broken} of ${files.length} session file(s) have broken chains.`,
        );
      }
    }),
    registerChatParticipant(context, harnessRoot, sessionsDir),
  );
}

export function deactivate() {
  /* disposables handle cleanup */
}
