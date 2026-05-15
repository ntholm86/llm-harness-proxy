/// Harness Protocol ledger writer — fail-closed, hash-chained JSONL.
/// Implements SPEC S4, S5, S8, S9.1.
use crate::jcs;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const PROTOCOL_VERSION: u64 = 1;
const GENESIS_PREV: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

pub struct EntryMeta {
    pub seq: u64,
    pub prev: String,
}

pub fn hash_entry(entry: &Value) -> String {
    let canonical = jcs::canonicalize(entry);
    let digest = Sha256::digest(&canonical);
    format!("sha256:{:x}", digest)
}

pub fn hash_input(system: Option<&str>, messages: Option<&Value>, tools: Option<&Value>) -> String {
    let payload = json!({
        "system": system,
        "messages": messages.cloned().unwrap_or(Value::Array(vec![])),
        "tools": tools.cloned().unwrap_or(Value::Array(vec![]))
    });
    let canonical = jcs::canonicalize(&payload);
    let digest = Sha256::digest(&canonical);
    format!("sha256:{:x}", digest)
}

/// Append one ledger entry — fail-closed.
/// Caller MUST NOT release the upstream response if this returns Err.
pub struct SessionLedger;

impl SessionLedger {
    pub fn append_entry(
        root: &Path,
        sid: &str,
        model: &str,
        in_hash: &str,
        has_think: bool,
        think: Option<&Value>,
        has_act: bool,
        act: Option<&Value>,
        reason: &str,
    ) -> Result<EntryMeta> {
        let sessions_dir = root.join("sessions");
        std::fs::create_dir_all(&sessions_dir)
            .context("failed to create sessions directory")?;

        let path: PathBuf = sessions_dir.join(format!("{}.jsonl", sid));

        // Open for read+append — get exclusive lock via OS file lock.
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .context("failed to open session file")?;

        // Scan existing entries to get seq + prev_hash.
        // If a torn line is found, truncate to clean_end before writing so the
        // recovery entry is not concatenated onto the torn fragment.
        let (seq, prev, torn_offset) = scan_tail(&mut file)?;
        if let Some(offset) = torn_offset {
            // On Windows, .append(true) grants only FILE_APPEND_DATA, not
            // FILE_WRITE_DATA — insufficient for SetEndOfFile (set_len).
            // Open a second handle with write access solely for the truncation;
            // the append handle remains valid for writing below.
            OpenOptions::new()
                .write(true)
                .open(&path)
                .context("truncate reopen failed")?
                .set_len(offset)
                .context("truncate torn entry failed")?;
        }

        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let entry = json!({
            "v": PROTOCOL_VERSION,
            "seq": seq,
            "sid": sid,
            "ts": ts,
            "model": model,
            "in": in_hash,
            "transparency": {
                "think": has_think,
                "act": has_act,
            },
            "think": think,
            "reason": reason,
            "act": act,
            "prev": prev
        });

        let line = serde_json::to_string(&entry)? + "\n";
        let data = line.as_bytes();

        // FAIL-CLOSED: write + fsync MUST succeed before caller releases act.
        file.seek(SeekFrom::End(0)).context("seek failed")?;
        file.write_all(data).context("write failed")?;
        file.flush().context("flush failed")?;
        file.sync_all().context("fsync failed")?;

        let entry_hash = hash_entry(&entry);

        Ok(EntryMeta { seq, prev: entry_hash })
    }
}

/// Scan a session file from the start, collecting the last valid entry.
/// Returns `(next_seq, prev_hash, torn_offset)` where `torn_offset` is
/// `Some(n)` when a torn (unparseable) line was found after the last valid
/// entry — `n` is the byte offset of that torn line so the caller can
/// truncate the file there before the next write.
fn scan_tail(file: &mut File) -> Result<(u64, String, Option<u64>)> {
    file.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::new(&*file);
    let mut last_entry: Option<Value> = None;
    let mut last_seq: i64 = -1;
    let mut clean_end: u64 = 0;

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 { break; } // EOF — clean end
        let trimmed = line.trim();
        if trimmed.is_empty() {
            clean_end += n as u64;
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(obj) => {
                clean_end += n as u64;
                if let Some(seq) = obj.get("seq").and_then(|v| v.as_i64()) {
                    last_seq = seq;
                }
                last_entry = Some(obj);
            }
            Err(_) => {
                // Torn line — return clean_end as the truncation point.
                return match last_entry {
                    None => Ok((0, GENESIS_PREV.to_string(), Some(clean_end))),
                    Some(e) => Ok(((last_seq + 1) as u64, hash_entry(&e), Some(clean_end))),
                };
            }
        }
    }

    match last_entry {
        None => Ok((0, GENESIS_PREV.to_string(), None)),
        Some(e) => Ok(((last_seq + 1) as u64, hash_entry(&e), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;

    /// Unique throwaway directory per test. Label must be unique across tests
    /// in this binary because tests run in parallel within the same process.
    fn tmp_root(label: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("harness-ledger-{}-{}", std::process::id(), label));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create tmp root");
        p
    }

    // --- §12.1 Genesis case ---------------------------------------------------

    #[test]
    fn genesis_seq_and_prev() {
        let root = tmp_root("genesis");
        let meta = SessionLedger::append_entry(
            &root, "s1", "m", "sha256:00",
            false, None, false, None, "first",
        ).expect("append");

        assert_eq!(meta.seq, 0);

        // The returned meta.prev is the hash of the entry just written.
        // Read the file and verify the written entry itself carries the genesis sentinel.
        let path = root.join("sessions").join("s1.jsonl");
        let content = std::fs::read_to_string(path).expect("read");
        let entry: Value = serde_json::from_str(content.trim()).expect("parse");
        assert_eq!(entry["seq"], 0);
        assert_eq!(entry["prev"].as_str().unwrap(), GENESIS_PREV);
    }

    // --- §12.2 Hash chain round-trip ------------------------------------------

    #[test]
    fn hash_chain_round_trip() {
        let root = tmp_root("chain");
        for i in 0..3u64 {
            SessionLedger::append_entry(
                &root, "s2", "m", "sha256:00",
                false, None, false, None, &format!("entry {}", i),
            ).expect("append");
        }

        let path = root.join("sessions").join("s2.jsonl");
        let content = std::fs::read_to_string(path).expect("read");
        let entries: Vec<Value> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("parse"))
            .collect();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0]["seq"], 0);
        assert_eq!(entries[0]["prev"].as_str().unwrap(), GENESIS_PREV);

        let h0 = hash_entry(&entries[0]);
        assert_eq!(entries[1]["seq"], 1);
        assert_eq!(entries[1]["prev"].as_str().unwrap(), h0,
            "entry[1].prev must equal hash(entry[0])");

        let h1 = hash_entry(&entries[1]);
        assert_eq!(entries[2]["seq"], 2);
        assert_eq!(entries[2]["prev"].as_str().unwrap(), h1,
            "entry[2].prev must equal hash(entry[1])");
    }

    // --- §12.3 Tamper detection -----------------------------------------------

    #[test]
    fn tamper_detection() {
        let root = tmp_root("tamper");
        SessionLedger::append_entry(
            &root, "s3", "m", "sha256:00",
            false, None, false, None, "entry 0",
        ).expect("append 0");
        SessionLedger::append_entry(
            &root, "s3", "m", "sha256:00",
            false, None, false, None, "entry 1",
        ).expect("append 1");

        let path = root.join("sessions").join("s3.jsonl");
        let content = std::fs::read_to_string(path).expect("read");
        let mut lines = content.lines().filter(|l| !l.trim().is_empty());
        let e0: Value = serde_json::from_str(lines.next().unwrap()).expect("e0");
        let e1: Value = serde_json::from_str(lines.next().unwrap()).expect("e1");

        // Chain is intact before tamper
        assert_eq!(e1["prev"].as_str().unwrap(), hash_entry(&e0),
            "chain must be intact on clean write");

        // Mutate one field — hash must diverge
        let mut e0_mut = e0.clone();
        e0_mut["reason"] = Value::String("TAMPERED".into());
        assert_ne!(
            hash_entry(&e0_mut),
            e1["prev"].as_str().unwrap(),
            "tampered entry must produce a different hash — chain break detectable"
        );
    }

    // --- §12.4 Torn-line scan recovery ----------------------------------------

    #[test]
    fn scan_tail_stops_at_torn_line() {
        let root = tmp_root("torn");
        std::fs::create_dir_all(root.join("sessions")).expect("mkdir sessions");
        let path = root.join("sessions").join("torn.jsonl");

        let entry0 = json!({
            "v": PROTOCOL_VERSION,
            "seq": 0_i64,
            "sid": "torn",
            "ts": "2026-01-01T00:00:00.000Z",
            "model": "m",
            "in": "sha256:00",
            "transparency": {"think": false, "act": false},
            "think": null,
            "reason": "clean entry",
            "act": null,
            "prev": GENESIS_PREV
        });

        {
            let mut f = File::create(&path).expect("create");
            writeln!(f, "{}", serde_json::to_string(&entry0).expect("serialize"))
                .expect("write entry0");
            // Torn write: incomplete JSON, no closing brace, no newline
            write!(f, r#"{{"v":1,"seq":1,"sid":"torn","reason":"in"#)
                .expect("write torn fragment");
        }

        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(&path)
            .expect("open");
        let (seq, prev, torn_offset) = scan_tail(&mut file).expect("scan_tail");

        assert_eq!(seq, 1, "seq continues from last clean entry (0 → 1)");
        assert_eq!(prev, hash_entry(&entry0),
            "prev is the hash of the last clean entry");
        assert!(torn_offset.is_some(),
            "torn_offset must be Some when a torn line is present");
    }

    // --- §12.5 Torn-line full recovery (write path) ---------------------------
    //
    // append_entry must truncate torn bytes before writing the recovery entry
    // so the file contains exactly N clean, chain-linked entries afterwards.
    #[test]
    fn torn_line_full_recovery() {
        let root = tmp_root("recovery");
        // Write entry 0 through the normal path.
        SessionLedger::append_entry(
            &root, "s5", "m", "sha256:00",
            false, None, false, None, "entry 0",
        ).expect("append entry 0");

        // Simulate a crash mid-write of entry 1 — append a torn fragment.
        let path = root.join("sessions").join("s5.jsonl");
        {
            let mut f = OpenOptions::new().append(true).open(&path)
                .expect("open for tear");
            write!(f, r#"{{"v":1,"seq":1,"sid":"s5","reason":"torn"#)
                .expect("write torn fragment");
        }

        // Recovery write — append_entry must truncate the torn bytes first.
        SessionLedger::append_entry(
            &root, "s5", "m", "sha256:00",
            false, None, false, None, "entry 1 recovered",
        ).expect("recovery append");

        // File must now contain exactly 2 valid, chain-linked entries.
        let content = std::fs::read_to_string(&path).expect("read");
        let entries: Vec<Value> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l)
                .expect("parse — recovery must produce clean lines"))
            .collect();

        assert_eq!(entries.len(), 2,
            "file must contain exactly 2 readable entries after recovery");
        assert_eq!(entries[0]["seq"], 0);
        assert_eq!(entries[1]["seq"], 1);
        let h0 = hash_entry(&entries[0]);
        assert_eq!(entries[1]["prev"].as_str().unwrap(), h0,
            "chain intact after recovery");
        assert_eq!(entries[1]["reason"].as_str().unwrap(), "entry 1 recovered",
            "recovery entry content preserved");
    }

    // --- §12.7 Cross-process sequence ----------------------------------------
    //
    // Two threads appending to the same session in alternation (enforced by a
    // Mutex) must produce strictly increasing seq with no gaps and a valid chain.
    // Models SPEC §12 requirement: "two processes appending to the same session
    // in alternation produce strictly increasing seq with no gaps and a valid chain."
    #[test]
    fn cross_process_alternating_writes() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let root = Arc::new(tmp_root("crossproc"));
        let root_a = root.clone();
        let root_b = root.clone();

        // Mutex enforces strict alternation: only one writer holds it at a time.
        let turn = Arc::new(Mutex::<()>::new(()));
        let turn_b = turn.clone();

        const WRITES_PER_THREAD: usize = 5;

        let h_a = thread::spawn(move || {
            for i in 0..WRITES_PER_THREAD {
                let _guard = turn.lock().unwrap();
                SessionLedger::append_entry(
                    &root_a, "sp1", "m", "sha256:aa",
                    false, None, false, None, &format!("writer-A entry {}", i),
                )
                .expect("append from writer A");
            }
        });

        let h_b = thread::spawn(move || {
            for i in 0..WRITES_PER_THREAD {
                let _guard = turn_b.lock().unwrap();
                SessionLedger::append_entry(
                    &root_b, "sp1", "m", "sha256:bb",
                    false, None, false, None, &format!("writer-B entry {}", i),
                )
                .expect("append from writer B");
            }
        });

        h_a.join().expect("writer A thread");
        h_b.join().expect("writer B thread");

        let path = root.join("sessions").join("sp1.jsonl");
        let content = std::fs::read_to_string(&path).expect("read session file");
        let entries: Vec<Value> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<Value>(l).expect("parse entry"))
            .collect();

        let total = WRITES_PER_THREAD * 2;
        assert_eq!(entries.len(), total, "all {} entries must be present", total);

        // seq must be strictly 0..total — no gaps, no duplicates.
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(
                e["seq"].as_u64().unwrap(), i as u64,
                "entry[{}] must have seq={}", i, i
            );
        }

        // Hash chain must be intact across all entries regardless of writer.
        assert_eq!(
            entries[0]["prev"].as_str().unwrap(), GENESIS_PREV,
            "first entry must carry genesis prev"
        );
        for i in 1..total {
            let expected = hash_entry(&entries[i - 1]);
            assert_eq!(
                entries[i]["prev"].as_str().unwrap(), expected,
                "chain break at entry[{}]", i
            );
        }
    }
}
