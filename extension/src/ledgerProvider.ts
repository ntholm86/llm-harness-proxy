import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';

export interface LedgerEntry {
  v: number;
  seq: number;
  sid: string;
  ts: string;
  model: string;
  in: string;
  reason: string;
  act: unknown;
  prev: string;
}

export class LedgerProvider
  implements vscode.TreeDataProvider<LedgerNode>, vscode.Disposable
{
  private readonly _onDidChange = new vscode.EventEmitter<
    LedgerNode | undefined
  >();
  readonly onDidChangeTreeData = this._onDidChange.event;
  private watcher: fs.FSWatcher | null = null;

  constructor(private readonly sessionsDir: string) {
    this.ensureWatch();
  }

  private ensureWatch() {
    try {
      fs.mkdirSync(this.sessionsDir, { recursive: true });
      this.watcher = fs.watch(
        this.sessionsDir,
        { persistent: false },
        () => this.refresh(),
      );
    } catch (e) {
      // workspace may not exist yet — ignore
    }
  }

  refresh(): void {
    this._onDidChange.fire(undefined);
  }

  getTreeItem(element: LedgerNode): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: LedgerNode): Promise<LedgerNode[]> {
    if (!element) {
      return this.listSessions();
    }
    if (element.kind === 'session') {
      return this.listEntries(element.sessionFile!);
    }
    return [];
  }

  private listSessions(): LedgerNode[] {
    if (!fs.existsSync(this.sessionsDir)) return [];
    const files = fs
      .readdirSync(this.sessionsDir)
      .filter((f) => f.endsWith('.jsonl'))
      .map((f) => path.join(this.sessionsDir, f))
      .sort(
        (a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs,
      );
    return files.map((f) => {
      const sid = path.basename(f, '.jsonl');
      const entries = readEntries(f);
      const node = new LedgerNode(
        `${sid}  (${entries.length})`,
        vscode.TreeItemCollapsibleState.Collapsed,
      );
      node.kind = 'session';
      node.sessionFile = f;
      node.iconPath = new vscode.ThemeIcon('book');
      node.tooltip = `${entries.length} entries`;
      node.contextValue = 'session';
      return node;
    });
  }

  private listEntries(file: string): LedgerNode[] {
    const entries = readEntries(file);
    const verify = verifyChain(entries);
    return entries.map((e, i) => {
      const ok = verify[i];
      const label = `#${e.seq} ${e.model} ${e.act ? '⚡ act' : '· reason'}`;
      const node = new LedgerNode(label, vscode.TreeItemCollapsibleState.None);
      node.kind = 'entry';
      node.tooltip = new vscode.MarkdownString(
        [
          `**ts**: ${e.ts}`,
          `**in**: \`${e.in}\``,
          `**prev**: \`${e.prev}\``,
          '',
          `**reason**: ${e.reason || '_(empty)_'}`,
          e.act ? `**act**: \`${JSON.stringify(e.act)}\`` : '',
        ].join('\n\n'),
      );
      node.iconPath = new vscode.ThemeIcon(
        ok ? 'pass-filled' : 'error',
        new vscode.ThemeColor(
          ok ? 'testing.iconPassed' : 'testing.iconFailed',
        ),
      );
      node.command = {
        command: 'vscode.open',
        title: 'Open',
        arguments: [vscode.Uri.file(file)],
      };
      return node;
    });
  }

  dispose() {
    this.watcher?.close();
    this._onDidChange.dispose();
  }
}

export class LedgerNode extends vscode.TreeItem {
  kind: 'session' | 'entry' = 'entry';
  sessionFile?: string;
}

export function readEntries(file: string): LedgerEntry[] {
  if (!fs.existsSync(file)) return [];
  const text = fs.readFileSync(file, 'utf8');
  const out: LedgerEntry[] = [];
  for (const line of text.split('\n')) {
    if (!line.trim()) continue;
    try {
      out.push(JSON.parse(line) as LedgerEntry);
    } catch {
      // torn final line — skip
    }
  }
  return out;
}

export function verifyChain(entries: LedgerEntry[]): boolean[] {
  const GENESIS = 'sha256:' + '0'.repeat(64);
  const result: boolean[] = [];
  let expectedPrev = GENESIS;
  for (const e of entries) {
    const ok = e.prev === expectedPrev;
    result.push(ok);
    expectedPrev = 'sha256:' + sha256OfEntry(e);
  }
  return result;
}

function sha256OfEntry(e: LedgerEntry): string {
  // We don't have JCS in TS here; the chain check uses the *recorded* prev
  // for now. A future iteration will add JCS in TS for full local re-hash.
  // We at least confirm linear continuity vs. the previously-recorded hash.
  return crypto
    .createHash('sha256')
    .update(JSON.stringify(e))
    .digest('hex');
}
