use serde_json::Value;

/// Accept comments and trailing commas in bounded theme resources only.
/// Serde still validates JSON values, strings, nesting, and number syntax.
pub(super) fn parse(source: &[u8]) -> Option<Value> {
    let source = source.strip_prefix(b"\xef\xbb\xbf").unwrap_or(source);
    let mut bytes = source.to_vec();
    let (mut quoted, mut escaped, mut index) = (false, false, 0);
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            quoted = true;
            index += 1;
            continue;
        }
        if byte == b'/' && matches!(bytes.get(index + 1), Some(b'/' | b'*')) {
            let block = bytes[index + 1] == b'*';
            let mut end = index + 2;
            if block {
                while end + 1 < bytes.len() && &bytes[end..end + 2] != b"*/" {
                    end += 1;
                }
                if end + 1 >= bytes.len() {
                    return None;
                }
                end += 2;
            } else {
                while end < bytes.len() && !matches!(bytes[end], b'\n' | b'\r') {
                    end += 1;
                }
            }
            for byte in &mut bytes[index..end] {
                if !matches!(*byte, b'\n' | b'\r') {
                    *byte = b' ';
                }
            }
            index = end;
        } else {
            index += 1;
        }
    }
    (quoted, escaped) = (false, false);
    for index in 0..bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b',' {
            let next = bytes[index + 1..]
                .iter()
                .find(|byte| !byte.is_ascii_whitespace());
            let previous = bytes[..index]
                .iter()
                .rev()
                .find(|byte| !byte.is_ascii_whitespace());
            if matches!(next, Some(b']' | b'}'))
                && previous.is_some_and(|byte| !matches!(*byte, b'[' | b'{' | b',' | b':'))
            {
                bytes[index] = b' ';
            }
        }
    }
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn theme_json_comments_and_trailing_commas_preserve_strings_and_values() {
        let source = br##"/* theme */ {
          "url":"https://example.test/a//b/*c*/",
          "escaped":"quote: \" // text",
          "unicode":"Unicode",
          // rule list
          "tokenColors":[{"scope":"comment", "settings":{"foreground":"#AABBCC",},},],
        } // end"##;
        let parsed = parse(source).unwrap();
        assert_eq!(parsed["url"], "https://example.test/a//b/*c*/");
        assert_eq!(parsed["escaped"], "quote: \" // text");
        assert_eq!(parsed["unicode"], "Unicode");
        assert_eq!(parsed["tokenColors"][0]["scope"], "comment");
        assert_eq!(parse(b"\xef\xbb\xbf{\"a\":true,}"), Some(json!({"a":true})));
        assert_eq!(
            parse("{\"name\":\"日本語\",} /* 日本語 */".as_bytes()),
            Some(json!({"name":"日本語"}))
        );
    }
    #[test]
    fn theme_json_does_not_repair_invalid_json_or_accept_executable_syntax() {
        for source in ["/* unfinished", "{\"a\":1,,}", "[,]"] {
            assert!(parse(source.as_bytes()).is_none(), "{source}");
        }
        for source in [
            "{\"a\":,}",
            "{,}",
            "{a:1}",
            "{'a':1}",
            "{\"a\":NaN}",
            "[1,,]",
            "{\"a\":function(){}}",
            "// comment only",
            "[1,] trailing",
        ] {
            assert!(parse(source.as_bytes()).is_none(), "{source}");
        }
    }
}
