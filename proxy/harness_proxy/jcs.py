"""RFC 8785 (JCS) JSON Canonicalization.

Minimal implementation sufficient for the Harness Protocol MVP.
Handles: null, bool, int, float (per ECMA-262 / RFC 8785), str, list, dict.
"""

from __future__ import annotations

import math
import re
from typing import Any


def canonicalize(value: Any) -> bytes:
    """Return the RFC 8785 canonical JSON encoding of `value` as UTF-8 bytes."""
    return _emit(value).encode("utf-8")


def _emit(v: Any) -> str:
    if v is None:
        return "null"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, str):
        return _emit_str(v)
    if isinstance(v, bool):  # already handled, but keep order safe
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return _emit_number(v)
    if isinstance(v, list):
        return "[" + ",".join(_emit(item) for item in v) + "]"
    if isinstance(v, dict):
        # JCS: sort by UTF-16 code units of the key. For BMP-only keys this
        # equals codepoint order, which Python's default sorted() produces.
        items = sorted(v.items(), key=lambda kv: kv[0])
        return "{" + ",".join(f"{_emit_str(k)}:{_emit(val)}" for k, val in items) + "}"
    raise TypeError(f"JCS: unsupported type {type(v).__name__}")


def _emit_number(n: float) -> str:
    if math.isnan(n) or math.isinf(n):
        raise ValueError("JCS: NaN/Inf not allowed")
    if n == 0:
        return "0"
    # ECMA-262 ToString(Number) — Python's repr is close but not identical;
    # this covers the cases we need for MVP. Negative-zero preserved as "0".
    if n.is_integer() and abs(n) < 1e21:
        return str(int(n))
    return repr(n)


_ESCAPE = {
    "\\": "\\\\",
    '"': '\\"',
    "\b": "\\b",
    "\f": "\\f",
    "\n": "\\n",
    "\r": "\\r",
    "\t": "\\t",
}
_NEEDS_ESCAPE = re.compile(r'[\x00-\x1f"\\]')


def _emit_str(s: str) -> str:
    def replace(match: re.Match[str]) -> str:
        ch = match.group(0)
        if ch in _ESCAPE:
            return _ESCAPE[ch]
        return f"\\u{ord(ch):04x}"

    return '"' + _NEEDS_ESCAPE.sub(replace, s) + '"'
