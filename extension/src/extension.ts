import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { ProxyController } from './proxyController';
import { LedgerProvider } from './ledgerProvider';
import { registerChatParticipant } from './chatParticipant';

export async function activate(context: vscode.ExtensionContext) {
  // Resolve workspace root — fall back to extension's parent (repo root) so
  // the extension still functions when opened in a multi-root or no-folder
  // Extension Development Host window.
  const ws = vscode.workspace.workspaceFolders?.[0];
  const root = ws?.uri.fsPath ?? path.join(context.extensionPath, '..');

  const proxy = new ProxyController(root, context.extensionPath);

  const cfg = proxy.config;
  const siblingHarnessRoot = path.join(context.extensionPath, '..', '.harness');
  const harnessRoot = path.isAbsolute(cfg.root)
    ? cfg.root
    : path.join(root, cfg.root);
  const resolvedHarnessRoot = fs.existsSync(harnessRoot) ? harnessRoot : siblingHarnessRoot;
  const sessionsDir = path.join(resolvedHarnessRoot, 'sessions');
  const ledger = new LedgerProvider(sessionsDir);

  // Always register every command so VS Code never emits 'command not found'.
  context.subscriptions.push(
    proxy,
    ledger,
    vscode.window.registerTreeDataProvider('harnessLedger', ledger),
    vscode.commands.registerCommand('harness.start', () => proxy.start()),
    vscode.commands.registerCommand('harness.stop', () => proxy.stop()),
    vscode.commands.registerCommand('harness.refresh', () => ledger.refresh()),
    vscode.commands.registerCommand('harness.toggle', () =>
      proxy.running ? proxy.stop() : proxy.start(),
    ),
    vscode.commands.registerCommand('harness.openSession', (file: string) =>
      vscode.window.showTextDocument(vscode.Uri.file(file)),
    ),
    vscode.commands.registerCommand('harness.verifyChain', async () => {
      const files = await vscode.workspace.findFiles(
        new vscode.RelativePattern(sessionsDir, '*.jsonl'),
      );
      vscode.window.showInformationMessage(
        `Harness: ${files.length} session(s) on disk.`,
      );
    }),
    proxy.onChanged(() => ledger.refresh()),
    registerChatParticipant(context, resolvedHarnessRoot, sessionsDir),
  );

  if (cfg.injectEnv) {
    context.environmentVariableCollection.persistent = false;
    context.environmentVariableCollection.replace(
      'OPENAI_BASE_URL',
      `http://${cfg.host}:${cfg.port}/v1`,
    );
    context.environmentVariableCollection.replace(
      'ANTHROPIC_BASE_URL',
      `http://${cfg.host}:${cfg.port}`,
    );
  }

  if (cfg.autoStart) {
    void proxy.start();
  }
}

export function deactivate() {
  /* disposables handle cleanup */
}
