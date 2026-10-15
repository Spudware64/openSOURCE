//! Small KeyValues lexer/tree preserving duplicate keys (Source entity outputs).
#[derive(Clone, Debug)]
pub enum Value {
    Text(String),
    Block(Vec<(String, Value)>),
}
pub fn tokens(s: &str) -> Result<Vec<String>, String> {
    let b = s.as_bytes();
    let mut p = 0;
    let mut out = vec![];
    while p < b.len() {
        if b[p].is_ascii_whitespace() || b[p] == 0 {
            p += 1;
            continue;
        }
        if b[p..].starts_with(b"//") {
            while p < b.len() && b[p] != b'\n' {
                p += 1;
            }
            continue;
        }
        if matches!(b[p], b'{' | b'}') {
            out.push((b[p] as char).to_string());
            p += 1;
            continue;
        }
        let quoted = b[p] == b'"';
        if quoted {
            p += 1;
        }
        let start = p;
        while p < b.len()
            && if quoted {
                b[p] != b'"'
            } else {
                !b[p].is_ascii_whitespace() && !matches!(b[p], b'{' | b'}')
            }
        {
            p += 1;
        }
        if quoted && p == b.len() {
            return Err("unterminated KeyValues string".into());
        }
        out.push(s[start..p].to_owned());
        if quoted {
            p += 1;
        }
    }
    Ok(out)
}
pub fn parse(s: &str) -> Result<Vec<(String, Value)>, String> {
    fn block(
        t: &[String],
        p: &mut usize,
        nested: bool,
        depth: usize,
    ) -> Result<Vec<(String, Value)>, String> {
        if depth > 32 {
            return Err("KeyValues nesting limit".into());
        }
        let mut out = vec![];
        while *p < t.len() {
            let key = t[*p].clone();
            *p += 1;
            if key == "}" {
                return if nested {
                    Ok(out)
                } else {
                    Err("unexpected KeyValues close".into())
                };
            }
            if key == "{" {
                return Err("missing KeyValues key".into());
            }
            let value = t.get(*p).ok_or("missing KeyValues value")?.clone();
            *p += 1;
            let value = if value == "{" {
                Value::Block(block(t, p, true, depth + 1)?)
            } else if value == "}" {
                return Err("missing KeyValues value".into());
            } else {
                Value::Text(value)
            };
            out.push((key, value));
        }
        if nested {
            Err("unterminated KeyValues block".into())
        } else {
            Ok(out)
        }
    }
    let t = tokens(s)?;
    block(&t, &mut 0, false, 0)
}
pub fn text<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    pairs.iter().find_map(|(k, v)| {
        if k.eq_ignore_ascii_case(key) {
            if let Value::Text(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        } else {
            None
        }
    })
}
pub fn child<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a [(String, Value)]> {
    pairs.iter().find_map(|(k, v)| {
        if k.eq_ignore_ascii_case(key) {
            if let Value::Block(s) = v {
                Some(s.as_slice())
            } else {
                None
            }
        } else {
            None
        }
    })
}
