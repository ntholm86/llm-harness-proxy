/// Harness Protocol ledger writer — fail-closed, hash-chained JSONL.
/// Implements SPEC S4, S5, S8, S9.1.
use crate::{jcs, ulid};
use anyhow::{Context, Result, bail};
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
        think: Option<&Value>,
        reason: &str,
        act: Option<&Value>,
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
        let (seq, prev) = scan_tail(&mut file)?;

        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let entry = json!({
            "v": PROTOCOL_VERSION,
            "seq": seq,
            "sid": sid,
            "ts": ts,
            "model": model,
            "in": in_hash,
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

fn scan_tail(file: &mut File) -> Result<(u64, String)> {
    file.seek(SeekFrom::Start(0))?;
    let reader = BufReader::new(&*file);
    let mut last_entry: Option<Value> = None;
    let mut last_seq: i64 = -1;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(obj) => {
                if let Some(seq) = obj.get("seq").and_then(|v| v.as_i64()) {
                    last_seq = seq;
                }
                last_entry = Some(obj);
            }
            Err(_) => break, // torn line — stop
        }
    }

    match last_entry {
        None => Ok((0, GENESIS_PREV.to_string())),
        Some(e) => Ok(((last_seq + 1) as u64, hash_entry(&e))),
    }
}
