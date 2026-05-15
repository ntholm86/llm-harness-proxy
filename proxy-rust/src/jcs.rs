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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn canon(v: Value) -> String {
        String::from_utf8(canonicalize(&v)).unwrap()
    }

    // --- Key ordering ---

    #[test]
    fn sorts_object_keys_alphabetically() {
        assert_eq!(canon(json!({"z": 2, "a": 1, "m": 0})), r#"{"a":1,"m":0,"z":2}"#);
    }

    #[test]
    fn key_ordering_is_insertion_order_independent() {
        // Both orderings must canonicalize to identical bytes.
        let fwd = canon(json!({"a": 1, "b": 2}));
        let rev = canon(json!({"b": 2, "a": 1}));
        assert_eq!(fwd, rev);
        assert_eq!(fwd, r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn sorts_nested_object_keys() {
        assert_eq!(
            canon(json!({"z": {"b": 2, "a": 1}})),
            r#"{"z":{"a":1,"b":2}}"#
        );
    }

    // --- Arrays ---

    #[test]
    fn array_preserves_insertion_order() {
        assert_eq!(canon(json!([3, 1, 2])), "[3,1,2]");
    }

    // --- Scalars ---

    #[test]
    fn scalars_null_bool_number_string() {
        assert_eq!(canon(json!(null)),    "null");
        assert_eq!(canon(json!(true)),    "true");
        assert_eq!(canon(json!(false)),   "false");
        assert_eq!(canon(json!(42)),      "42");
        assert_eq!(canon(json!("hello")), r#""hello""#);
    }

    // --- String escapes (RFC 8785 §3.2.2.2) ---

    #[test]
    fn escapes_mandatory_control_characters() {
        assert_eq!(canon(Value::String("\n".into())),   r#""\n""#);   // U+000A
        assert_eq!(canon(Value::String("\r".into())),   r#""\r""#);   // U+000D
        assert_eq!(canon(Value::String("\t".into())),   r#""\t""#);   // U+0009
        assert_eq!(canon(Value::String("\x08".into())), r#""\b""#);   // U+0008 backspace
        assert_eq!(canon(Value::String("\x0C".into())), r#""\f""#);   // U+000C form feed
        assert_eq!(canon(Value::String("\\".into())),   r#""\\""#);   // U+005C backslash
        assert_eq!(canon(Value::String("\"".into())),   r#""\"""#);   // U+0022 double quote
    }

    #[test]
    fn escapes_other_c0_control_characters_as_unicode() {
        // U+0001 (SOH) must become \u0001, not a bare byte.
        assert_eq!(canon(Value::String("\x01".into())), r#""\u0001""#);
        // U+001F (US) — last C0 control before the printable range
        assert_eq!(canon(Value::String("\x1F".into())), r#""\u001f""#);
    }
}
