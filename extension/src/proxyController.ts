import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';
import * as fs from 'fs';
import * as http from 'http';

export class ProxyController implements vscode.Disposable {
  private proc: cp.ChildProcess | null = null;
  private output: vscode.OutputChannel;
  private readonly statusBar: vscode.StatusBarItem;
  private readonly onDidChange = new vscode.EventEmitter<boolean>();
  public readonly onChanged = this.onDidChange.event;
  // Directory containing the proxy package — sibling to the extension folder.
  private readonly proxyDir: string;

  constructor(
    private readonly workspaceRoot: string,
    private readonly extensionPath: string,
  ) {
    // Prefer workspace-local proxy if it exists; otherwise fall back to the
    // repo-sibling proxy directory (covers mono-repo dev scenarios where
    // the opened workspace root is the parent git root).
    const localProxy = path.join(workspaceRoot, 'proxy');
    const siblingProxy = path.join(extensionPath, '..', 'proxy');
    this.proxyDir = fs.existsSync(path.join(localProxy, 'harness_proxy'))
      ? localProxy
      : siblingProxy;
    this.output = vscode.window.createOutputChannel('Harness Proxy');
    this.statusBar = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left,
      100,
    );
    this.statusBar.command = 'harness.toggle';
    this.updateStatusBar();
    this.statusBar.show();
  }

  public get running(): boolean {
    return this.proc !== null && !this.proc.killed;
  }

  public get config() {
    const cfg = vscode.workspace.getConfiguration('harness.proxy');
    return {
      python: cfg.get<string>('python') || this.defaultPython(),
      port: cfg.get<number>('port') ?? 8080,
      host: cfg.get<string>('host') ?? '127.0.0.1',
      root: cfg.get<string>('root') ?? '.harness',
      autoStart: cfg.get<boolean>('autoStart') ?? true,
      injectEnv: cfg.get<boolean>('injectEnv') ?? true,
    };
  }

  private defaultPython(): string {
    const candidate = path.join(
      this.proxyDir,
      '.venv',
      process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python',
    );
    return fs.existsSync(candidate) ? candidate : 'python';
  }

  public async start(): Promise<void> {
    if (this.running) {
      this.output.appendLine('Proxy already running.');
      return;
    }
    const cfg = this.config;
    const proxyDir = this.proxyDir;
    const harnessRoot = path.isAbsolute(cfg.root)
      ? cfg.root
      : path.join(this.workspaceRoot, cfg.root);

    // Pre-flight: if the resolved python path is an absolute path that doesn't
    // exist, bail early with a friendly message instead of spawning and then
    // hanging 5 s waiting for a health-check that will never pass.
    if (path.isAbsolute(cfg.python) && !fs.existsSync(cfg.python)) {
      this.output.appendLine(
        `Harness proxy not started: python not found at "${cfg.python}". ` +
        `The @harness chat participant works without the proxy — ` +
        `set up the proxy venv (see README) to also harness terminal/script traffic.`,
      );
      return;
    }

    this.output.show(true);
    this.output.appendLine(`Starting proxy: ${cfg.python}`);
    this.output.appendLine(`  app-dir: ${proxyDir}`);
    this.output.appendLine(`  listen:  ${cfg.host}:${cfg.port}`);
    this.output.appendLine(`  root:    ${harnessRoot}`);

    this.proc = cp.spawn(
      cfg.python,
      [
        '-m',
        'uvicorn',
        'harness_proxy.proxy:app',
        '--host',
        cfg.host,
        '--port',
        String(cfg.port),
        '--app-dir',
        proxyDir,
      ],
      {
        cwd: this.workspaceRoot,
        env: {
          ...process.env,
          HARNESS_ROOT: harnessRoot,
        },
      },
    );

    this.proc.stdout?.on('data', (b) => this.output.append(b.toString()));
    this.proc.stderr?.on('data', (b) => this.output.append(b.toString()));
    this.proc.on('error', (err) => {
      this.output.appendLine(`Proxy spawn error: ${err.message}`);
      this.output.appendLine(
        `The @harness chat participant works without the proxy — ` +
        `set up the proxy venv (see README) to also harness terminal/script traffic.`,
      );
      this.proc = null;
      this.updateStatusBar();
      void vscode.commands.executeCommand('setContext', 'harness.running', false);
      this.onDidChange.fire(false);
    });
    this.proc.on('exit', (code, signal) => {
      this.output.appendLine(`Proxy exited code=${code} signal=${signal}`);
      this.proc = null;
      this.updateStatusBar();
      void vscode.commands.executeCommand(
        'setContext',
        'harness.running',
        false,
      );
      this.onDidChange.fire(false);
    });

    await this.waitForHealth(cfg.host, cfg.port, 5000);
    this.updateStatusBar();
    void vscode.commands.executeCommand(
      'setContext',
      'harness.running',
      true,
    );
    this.onDidChange.fire(true);
  }

  public async stop(): Promise<void> {
    if (!this.proc) return;
    this.output.appendLine('Stopping proxy...');
    this.proc.kill();
    this.proc = null;
    this.updateStatusBar();
    await vscode.commands.executeCommand(
      'setContext',
      'harness.running',
      false,
    );
    this.onDidChange.fire(false);
  }

  private async waitForHealth(
    host: string,
    port: number,
    timeoutMs: number,
  ): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      try {
        await this.healthCheck(host, port);
        return;
      } catch {
        await new Promise((r) => setTimeout(r, 200));
      }
    }
    this.output.appendLine('Proxy health check timed out.');
  }

  private healthCheck(host: string, port: number): Promise<void> {
    return new Promise((resolve, reject) => {
      const req = http.get(
        { host, port, path: '/healthz', timeout: 1000 },
        (res) => {
          if (res.statusCode === 200) resolve();
          else reject(new Error(`status=${res.statusCode}`));
          res.resume();
        },
      );
      req.on('error', reject);
      req.on('timeout', () => req.destroy(new Error('timeout')));
    });
  }

  private updateStatusBar() {
    if (this.running) {
      this.statusBar.text = `$(law) Harness :${this.config.port}`;
      this.statusBar.tooltip = 'Harness proxy is running. Click to stop.';
      this.statusBar.backgroundColor = undefined;
    } else {
      this.statusBar.text = `$(circle-slash) Harness off`;
      this.statusBar.tooltip = 'Harness proxy is stopped. Click to start.';
      this.statusBar.backgroundColor = new vscode.ThemeColor(
        'statusBarItem.warningBackground',
      );
    }
  }

  dispose(): void {
    void this.stop();
    this.statusBar.dispose();
    this.output.dispose();
    this.onDidChange.dispose();
  }
}
