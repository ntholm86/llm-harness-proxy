/// ULID generation (Crockford base32, 26 chars) per the ULID spec.
use std::time::{SystemTime, UNIX_EPOCH};

const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn new_ulid() -> String {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u128;
    let mut rand_bytes = [0u8; 10];
    for b in &mut rand_bytes { *b = rand::random(); }
    let rand_val = rand_bytes.iter().fold(0u128, |acc, &b| (acc << 8) | b as u128);
    let value: u128 = (ts_ms << 80) | (rand_val & ((1u128 << 80) - 1));
    let mut chars = [0u8; 26];
    let mut v = value;
    for i in (0..26).rev() {
        chars[i] = CROCKFORD[(v & 0x1F) as usize];
        v >>= 5;
    }
    String::from_utf8(chars.to_vec()).unwrap()
}
