import * as vscode from 'vscode';
import { LedgerProvider } from './ledgerProvider';

export function activate(context: vscode.ExtensionContext): void {
    const provider = new LedgerProvider();

    const treeView = vscode.window.createTreeView('harnessLedger', {
        treeDataProvider: provider,
        showCollapseAll: true,
    });

    context.subscriptions.push(treeView);

    context.subscriptions.push(
        vscode.commands.registerCommand('harness.refresh', () => provider.refresh()),
    );

    // Auto-refresh when workspace config changes (harness.root)
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(e => {
            if (e.affectsConfiguration('harness.root')) {
                provider.refresh();
            }
        }),
    );
}

export function deactivate(): void {
    // nothing
}
