"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.LedgerProvider = exports.HarnessNode = void 0;
const vscode = __importStar(require("vscode"));
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const harnessRoot_1 = require("./harnessRoot");
class HarnessNode extends vscode.TreeItem {
    constructor(kind, label, collapsible, data, sessionFile, entryIndex) {
        super(label, collapsible);
        this.kind = kind;
        this.data = data;
        this.sessionFile = sessionFile;
        this.entryIndex = entryIndex;
        this.contextValue = kind;
    }
}
exports.HarnessNode = HarnessNode;
// ── Provider ───────────────────────────────────────────────────────────────────
class LedgerProvider {
    constructor() {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(node) {
        return node;
    }
    getChildren(node) {
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
    _getSessions() {
        const root = (0, harnessRoot_1.resolveHarnessRoot)();
        if (!root) {
            return [new HarnessNode('message', '(no workspace folder)', vscode.TreeItemCollapsibleState.None)];
        }
        const dir = (0, harnessRoot_1.sessionsDir)(root);
        if (!fs.existsSync(dir)) {
            return [new HarnessNode('message', '(no .harness/sessions/ found)', vscode.TreeItemCollapsibleState.None)];
        }
        let files;
        try {
            files = fs.readdirSync(dir)
                .filter(f => f.endsWith('.jsonl'))
                .sort()
                .reverse(); // newest first
        }
        catch {
            return [new HarnessNode('message', '(cannot read sessions directory)', vscode.TreeItemCollapsibleState.None)];
        }
        if (files.length === 0) {
            return [new HarnessNode('message', '(no sessions yet)', vscode.TreeItemCollapsibleState.None)];
        }
        return files.map(f => {
            const fullPath = path.join(dir, f);
            const entryCount = this._countLines(fullPath);
            const node = new HarnessNode('session', f.replace('.jsonl', ''), vscode.TreeItemCollapsibleState.Collapsed, undefined, fullPath);
            node.description = `${entryCount} entr${entryCount === 1 ? 'y' : 'ies'}`;
            node.tooltip = fullPath;
            return node;
        });
    }
    // ── Entries within a session ─────────────────────────────────────────────────
    _getEntries(sessionFile) {
        const lines = this._readLines(sessionFile);
        return lines.map((line, i) => {
            let entry;
            try {
                entry = JSON.parse(line);
            }
            catch {
                const node = new HarnessNode('entry', `#${i} (parse error)`, vscode.TreeItemCollapsibleState.None, undefined, sessionFile, i);
                node.iconPath = new vscode.ThemeIcon('error');
                return node;
            }
            const label = `#${entry.seq} · ${entry.ts}`;
            const hasAction = entry.act !== null && entry.act !== undefined;
            const node = new HarnessNode('entry', label, vscode.TreeItemCollapsibleState.Collapsed, entry, sessionFile, i);
            node.description = entry.model ?? '';
            node.iconPath = new vscode.ThemeIcon(hasAction ? 'zap' : 'comment');
            if (entry.cont === 'open') {
                node.iconPath = new vscode.ThemeIcon('ellipsis');
            }
            return node;
        });
    }
    // ── Fields within an entry ───────────────────────────────────────────────────
    _getEntryFields(sessionFile, entryIndex) {
        const lines = this._readLines(sessionFile);
        const line = lines[entryIndex];
        if (!line) {
            return [];
        }
        let entry;
        try {
            entry = JSON.parse(line);
        }
        catch {
            return [new HarnessNode('message', '(parse error)', vscode.TreeItemCollapsibleState.None)];
        }
        const fields = [];
        const addField = (key, value) => {
            const str = typeof value === 'string' ? value : JSON.stringify(value);
            const node = new HarnessNode('message', key, vscode.TreeItemCollapsibleState.None);
            node.description = str.length > 120 ? str.slice(0, 120) + '…' : str;
            node.tooltip = str;
            fields.push(node);
        };
        // Show fields in a readable order
        const ordered = ['seq', 'sid', 'ts', 'model', 'cont', 'reason', 'act', 'prev', 'in'];
        for (const k of ordered) {
            if (k in entry) {
                addField(k, entry[k]);
            }
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
    _readLines(filePath) {
        try {
            return fs.readFileSync(filePath, 'utf8')
                .split('\n')
                .filter(l => l.trim().length > 0);
        }
        catch {
            return [];
        }
    }
    _countLines(filePath) {
        return this._readLines(filePath).length;
    }
}
exports.LedgerProvider = LedgerProvider;
//# sourceMappingURL=ledgerProvider.js.map