/// RFC 8785 (JCS) JSON Canonicalization — minimal implementation.
use serde_json::Value;

pub fn canonicalize(value: &Value) -> Vec<u8> {
    emit(value).into_bytes()
}

fn emit(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => emit_str(s),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(emit).collect();
            format!("[{}]", items.join(","))
        }
        Value::Object(map) => {
            // JCS: sort keys by UTF-16 code units (BMP-only = codepoint order)
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let items: Vec<String> = pairs.iter()
                .map(|(k, val)| format!("{}:{}", emit_str(k), emit(val)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
    }
}

fn emit_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"'  => out.push_str("\\\""),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
