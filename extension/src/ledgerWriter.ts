/**
 * TypeScript port of the Harness Protocol ledger primitives (mirrors
 * proxy/harness_proxy/{jcs,ulid,ledger}.py).
 *
 * Writes entries to <root>/sessions/<sid>.jsonl in the same format as the
 * Python proxy so both paths share a single auditable ledger directory.
 */

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as path from 'path';

// ── ULID ──────────────────────────────────────────────────────────────────────

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

export function newUlid(): string {
  const tsMs = Date.now();
  // 10-byte time (48-bit ms) + 10-byte random
  let t = tsMs;
  const chars: string[] = new Array(26);
  // 10 random chars (80 bits)
  const rand = crypto.randomBytes(10);
  for (let i = 25; i >= 16; i--) {
    chars[i] = CROCKFORD[rand[25 - i] & 0x1f];
  }
  // 10 time chars (50 bits, we use 48)
  for (let i = 9; i >= 0; i--) {
    chars[i] = CROCKFORD[t % 32];
    t = Math.floor(t / 32);
  }
  return chars.join('');
}

// ── JCS (RFC 8785 minimal) ────────────────────────────────────────────────────

export function jcsCanonicalize(value: unknown): Buffer {
  return Buffer.from(emit(value), 'utf8');
}

function emit(value: unknown): string {
  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new Error('JCS: NaN/Infinity not allowed');
    // Match Python's repr for integers: no decimal point
    return Object.is(value, -0) ? '0' : String(value);
  }
  if (typeof value === 'string') return emitString(value);
  if (Array.isArray(value)) return '[' + value.map(emit).join(',') + ']';
  if (typeof value === 'object' && value !== null) {
    const obj = value as Record<string, unknown>;
    const sorted = Object.keys(obj)
      .sort()
      .map((k) => emitString(k) + ':' + emit(obj[k]))
      .join(',');
    return '{' + sorted + '}';
  }
  throw new Error(`JCS: unsupported type ${typeof value}`);
}

function emitString(s: string): string {
  let out = '"';
  for (const ch of s) {
    const cp = ch.codePointAt(0)!;
    if (cp === 0x22) { out += '\\"'; continue; }
    if (cp === 0x5c) { out += '\\\\'; continue; }
    if (cp < 0x20) {
      out += '\\u' + cp.toString(16).padStart(4, '0');
      continue;
    }
    out += ch;
  }
  return out + '"';
}

// ── Ledger entry ──────────────────────────────────────────────────────────────

const PROTOCOL_VERSION = 1;
const GENESIS_PREV = 'sha256:' + '0'.repeat(64);

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

export class LedgerError extends Error {}

/**
 * Append a new entry to <root>/sessions/<sid>.jsonl under fail-closed
 * semantics: the file is fsynced before this function returns. If the fsync
 * fails the caller must NOT deliver the upstream response to the user.
 */
export function appendEntry(
  root: string,
  sid: string,
  model: string,
  inputHash: string,
  reason: string,
  act: unknown,
): LedgerEntry {
  const sessionsDir = path.join(root, 'sessions');
  fs.mkdirSync(sessionsDir, { recursive: true });
  const file = path.join(sessionsDir, sid + '.jsonl');

  // Recover seq + prev from existing file
  let seq = 0;
  let prevHash = GENESIS_PREV;
  if (fs.existsSync(file)) {
    const lines = fs.readFileSync(file, 'utf8').split('\n').filter(Boolean);
    for (let i = lines.length - 1; i >= 0; i--) {
      try {
        const last = JSON.parse(lines[i]) as LedgerEntry;
        seq = last.seq + 1;
        prevHash = hashEntry(last);
        break;
      } catch {
        // torn line — skip
      }
    }
  }

  const entry: LedgerEntry = {
    v: PROTOCOL_VERSION,
    seq,
    sid,
    ts: new Date().toISOString(),
    model,
    in: inputHash,
    reason,
    act: act ?? null,
    prev: prevHash,
  };

  // Fail-closed write
  try {
    const line = JSON.stringify(entry) + '\n';
    const fd = fs.openSync(file, 'a');
    try {
      fs.writeSync(fd, line);
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
  } catch (e) {
    throw new LedgerError(`ledger write failed: ${e}`);
  }

  return entry;
}

export function hashEntry(entry: LedgerEntry): string {
  const canonical = jcsCanonicalize(entry);
  return 'sha256:' + crypto.createHash('sha256').update(canonical).digest('hex');
}

export function hashInput(
  system: string | null | undefined,
  messages: unknown[],
  tools?: unknown[] | null,
): string {
  const obj = {
    system: system ?? null,
    messages,
    tools: tools ?? [],
  };
  const canonical = jcsCanonicalize(obj);
  return 'sha256:' + crypto.createHash('sha256').update(canonical).digest('hex');
}
