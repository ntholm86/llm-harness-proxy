import * as vscode from 'vscode';
import * as path from 'path';

/**
 * Resolve the harness root directory.
 *
 * Priority:
 *   1. `harness.root` config (absolute path)
 *   2. First workspace folder
 *
 * Returns undefined if neither is available.
 */
export function resolveHarnessRoot(): string | undefined {
    const cfg = vscode.workspace.getConfiguration('harness').get<string>('root');
    if (cfg && cfg.trim().length > 0) {
        return cfg.trim();
    }
    const folders = vscode.workspace.workspaceFolders;
    if (folders && folders.length > 0) {
        return folders[0].uri.fsPath;
    }
    return undefined;
}

/** Returns the .harness/sessions/ directory, or undefined. */
export function sessionsDir(root: string): string {
    return path.join(root, '.harness', 'sessions');
}
