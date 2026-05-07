"""Harness Protocol ledger writer.

Implements SPEC §4 (entry format), §5 (hash chain), §8 (storage layout),
§9.1 (fail-closed write semantics).

Single-writer-per-session enforced via msvcrt/fcntl file lock. The action
returned by `append()` is only safe to release if `append()` returned without
raising — this is the fail-closed contract.
"""

from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import sys
from pathlib import Path
from typing import Any

from .jcs import canonicalize

PROTOCOL_VERSION = 1
GENESIS_PREV = "sha256:" + ("0" * 64)


def _utc_now_iso() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.") + \
        f"{int(_dt.datetime.now(_dt.timezone.utc).microsecond / 1000):03d}Z"


def hash_entry(entry: dict[str, Any]) -> str:
    digest = hashlib.sha256(canonicalize(entry)).hexdigest()
    return f"sha256:{digest}"


def hash_input(system: str | None, messages: list[Any], tools: list[Any] | None) -> str:
    payload = {"system": system, "messages": messages, "tools": tools or []}
    digest = hashlib.sha256(canonicalize(payload)).hexdigest()
    return f"sha256:{digest}"


class LedgerError(Exception):
    """Raised when a fail-closed precondition fails. Action MUST NOT release."""


class SessionLedger:
    """Writer for one session's JSONL file. Holds an exclusive lock for life."""

    def __init__(self, root: Path, sid: str):
        self.root = root
        self.sid = sid
        self.path = root / "sessions" / f"{sid}.jsonl"
        self.path.parent.mkdir(parents=True, exist_ok=True)
        # Open in append+read binary, then take exclusive lock.
        self._fh = open(self.path, "a+b")
        self._lock_exclusive()
        self._seq, self._last_hash = self._scan_tail()

    def _lock_exclusive(self) -> None:
        if sys.platform == "win32":
            import msvcrt
            try:
                msvcrt.locking(self._fh.fileno(), msvcrt.LK_NBLCK, 1)
            except OSError as e:
                self._fh.close()
                raise LedgerError(f"session {self.sid} is locked by another writer") from e
        else:
            import fcntl
            try:
                fcntl.flock(self._fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as e:
                self._fh.close()
                raise LedgerError(f"session {self.sid} is locked by another writer") from e

    def _scan_tail(self) -> tuple[int, str]:
        """Return (next_seq, prev_hash) from the existing file, if any."""
        self._fh.seek(0)
        last_entry: dict[str, Any] | None = None
        last_seq = -1
        for line in self._fh:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                # Torn final line per §9.2: truncate and stop.
                pos = self._fh.tell() - len(line) - 1
                self._fh.seek(pos)
                self._fh.truncate()
                break
            last_entry = obj
            last_seq = obj.get("seq", last_seq)
        if last_entry is None:
            return 0, GENESIS_PREV
        return last_seq + 1, hash_entry(last_entry)

    def append(
        self,
        *,
        model: str,
        in_hash: str,
        reason: str,
        act: dict[str, Any] | None,
        cont: str | None = None,
    ) -> dict[str, Any]:
        """Append one entry. Returns the entry. Raises LedgerError on any failure
        before action release. Caller MUST NOT release `act` if this raises."""
        if cont == "open" and act is not None:
            raise LedgerError("§7.3 violation: open entry MUST have act=null")

        entry: dict[str, Any] = {
            "v": PROTOCOL_VERSION,
            "seq": self._seq,
            "sid": self.sid,
            "ts": _utc_now_iso(),
            "model": model,
            "in": in_hash,
            "reason": reason,
            "act": act,
            "prev": self._last_hash,
        }
        if cont is not None:
            entry["cont"] = cont

        line = json.dumps(entry, separators=(",", ":"), ensure_ascii=False) + "\n"
        data = line.encode("utf-8")

        # FAIL-CLOSED: write + fsync MUST succeed before caller may release act.
        try:
            self._fh.seek(0, os.SEEK_END)
            self._fh.write(data)
            self._fh.flush()
            os.fsync(self._fh.fileno())
        except OSError as e:
            raise LedgerError(f"durable write failed: {e}") from e

        self._seq += 1
        self._last_hash = hash_entry(entry)
        return entry

    def close(self) -> None:
        if not self._fh.closed:
            try:
                self._fh.flush()
                os.fsync(self._fh.fileno())
            except OSError:
                pass
            self._fh.close()
