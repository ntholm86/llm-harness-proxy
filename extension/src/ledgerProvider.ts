import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { resolveHarnessRoot, sessionsDir } from './harnessRoot';

// ── Tree node types ────────────────────────────────────────────────────────────

type NodeKind = 'session' | 'entry' | 'message';

export class HarnessNode extends vscode.TreeItem {
    constructor(
        public readonly kind: NodeKind,
        label: string,
        collapsible: vscode.TreeItemCollapsibleState,
        public readonly data?: unknown,
        public readonly sessionFile?: string,
        public readonly entryIndex?: number,
    ) {
        super(label, collapsible);
        this.contextValue = kind;
    }
}

// ── Raw ledger entry (only fields we display) ──────────────────────────────────

interface LedgerEntry {
    seq: number;
    sid: string;
    ts: string;
    model?: string;
    reason?: string;
    act?: unknown;
    cont?: string;
    prev?: string;
    [key: string]: unknown;
}

// ── Provider ───────────────────────────────────────────────────────────────────

export class LedgerProvider implements vscode.TreeDataProvider<HarnessNode> {

    private _onDidChangeTreeData = new vscode.EventEmitter<HarnessNode | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(node: HarnessNode): vscode.TreeItem {
        return node;
    }

    getChildren(node?: HarnessNode): HarnessNode[] {
        if (!node) {
            return this._getSessions();
        }
        if (node.kind === 'session' && node.sessionFile) {
            return this._getEntries(node.sessionFile);
        }
        if (node.kind === 'entry' && node.sessionFile && node.entryIndex !== undefined) {
            return this._getEntryFields(node.sessionFile, node.entryIndex);
        }
        return [];
    }

    // ── Session list ────────────────────────────────────────────────────────────

    private _getSessions(): HarnessNode[] {
        const root = resolveHarnessRoot();
        if (!root) {
            return [new HarnessNode('message', '(no workspace folder)', vscode.TreeItemCollapsibleState.None)];
        }

        const dir = sessionsDir(root);
        if (!fs.existsSync(dir)) {
            return [new HarnessNode('message', '(no .harness/sessions/ found)', vscode.TreeItemCollapsibleState.None)];
        }

        let files: string[];
        try {
            files = fs.readdirSync(dir)
                .filter(f => f.endsWith('.jsonl'))
                .sort()
                .reverse(); // newest first
        } catch {
            return [new HarnessNode('message', '(cannot read sessions directory)', vscode.TreeItemCollapsibleState.None)];
        }

        if (files.length === 0) {
            return [new HarnessNode('message', '(no sessions yet)', vscode.TreeItemCollapsibleState.None)];
        }

        return files.map(f => {
            const fullPath = path.join(dir, f);
            const entryCount = this._countLines(fullPath);
            const node = new HarnessNode(
                'session',
                f.replace('.jsonl', ''),
                vscode.TreeItemCollapsibleState.Collapsed,
                undefined,
                fullPath,
            );
            node.description = `${entryCount} entr${entryCount === 1 ? 'y' : 'ies'}`;
            node.tooltip = fullPath;
            return node;
        });
    }

    // ── Entries within a session ─────────────────────────────────────────────────

    private _getEntries(sessionFile: string): HarnessNode[] {
        const lines = this._readLines(sessionFile);
        return lines.map((line, i) => {
            let entry: LedgerEntry;
            try {
                entry = JSON.parse(line) as LedgerEntry;
            } catch {
                const node = new HarnessNode(
                    'entry',
                    `#${i} (parse error)`,
                    vscode.TreeItemCollapsibleState.None,
                    undefined,
                    sessionFile,
                    i,
                );
                node.iconPath = new vscode.ThemeIcon('error');
                return node;
            }

            const label = `#${entry.seq} · ${entry.ts}`;
            const hasAction = entry.act !== null && entry.act !== undefined;
            const node = new HarnessNode(
                'entry',
                label,
                vscode.TreeItemCollapsibleState.Collapsed,
                entry,
                sessionFile,
                i,
            );
            node.description = entry.model ?? '';
            node.iconPath = new vscode.ThemeIcon(hasAction ? 'zap' : 'comment');
            if (entry.cont === 'open') {
                node.iconPath = new vscode.ThemeIcon('ellipsis');
            }
            return node;
        });
    }

    // ── Fields within an entry ───────────────────────────────────────────────────

    private _getEntryFields(sessionFile: string, entryIndex: number): HarnessNode[] {
        const lines = this._readLines(sessionFile);
        const line = lines[entryIndex];
        if (!line) { return []; }

        let entry: LedgerEntry;
        try {
            entry = JSON.parse(line) as LedgerEntry;
        } catch {
            return [new HarnessNode('message', '(parse error)', vscode.TreeItemCollapsibleState.None)];
        }

        const fields: HarnessNode[] = [];

        const addField = (key: string, value: unknown) => {
            const str = typeof value === 'string' ? value : JSON.stringify(value);
            const node = new HarnessNode('message', key, vscode.TreeItemCollapsibleState.None);
            node.description = str.length > 120 ? str.slice(0, 120) + '…' : str;
            node.tooltip = str;
            fields.push(node);
        };

        // Show fields in a readable order
        const ordered: string[] = ['seq', 'sid', 'ts', 'model', 'cont', 'reason', 'act', 'prev', 'in'];
        for (const k of ordered) {
            if (k in entry) { addField(k, entry[k]); }
        }
        // Remaining unknown fields
        for (const k of Object.keys(entry)) {
            if (!ordered.includes(k)) {
                addField(k, entry[k]);
            }
        }

        return fields;
    }

    // ── Helpers ──────────────────────────────────────────────────────────────────

    private _readLines(filePath: string): string[] {
        try {
            return fs.readFileSync(filePath, 'utf8')
                .split('\n')
                .filter(l => l.trim().length > 0);
        } catch {
            return [];
        }
    }

    private _countLines(filePath: string): number {
        return this._readLines(filePath).length;
    }
}
