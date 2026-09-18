use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut idx = 0;
        skip_ws(&chars, &mut idx);
        let val = parse_value(&chars, &mut idx)?;
        skip_ws(&chars, &mut idx);
        Ok(val)
    }

    pub fn to_string(&self) -> String {
        match self {
            JsonValue::Null => "null".to_string(),
            JsonValue::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            JsonValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            JsonValue::String(s) => {
                let mut out = String::with_capacity(s.len() + 2);
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        _ => out.push(c),
                    }
                }
                out.push('"');
                out
            }
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(","))
            }
            JsonValue::Object(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", escape_str(k), v.to_string()))
                    .collect();
                format!("{{{}}}", items.join(","))
            }
        }
    }
}

fn escape_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn skip_ws(chars: &[char], idx: &mut usize) {
    while *idx < chars.len() && chars[*idx].is_whitespace() {
        *idx += 1;
    }
}

fn parse_value(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    skip_ws(chars, idx);
    if *idx >= chars.len() {
        return Err("Unexpected end of input".to_string());
    }

    match chars[*idx] {
        'n' => parse_null(chars, idx),
        't' | 'f' => parse_bool(chars, idx),
        '"' => parse_string(chars, idx).map(JsonValue::String),
        '[' => parse_array(chars, idx),
        '{' => parse_object(chars, idx),
        '-' | '0'..='9' => parse_number(chars, idx),
        other => Err(format!("Unexpected character '{}' at offset {}", other, *idx)),
    }
}

fn parse_null(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    if *idx + 4 <= chars.len() && &chars[*idx..*idx + 4] == ['n', 'u', 'l', 'l'] {
        *idx += 4;
        Ok(JsonValue::Null)
    } else {
        Err("Expected 'null'".to_string())
    }
}

fn parse_bool(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    if *idx + 4 <= chars.len() && &chars[*idx..*idx + 4] == ['t', 'r', 'u', 'e'] {
        *idx += 4;
        Ok(JsonValue::Bool(true))
    } else if *idx + 5 <= chars.len() && &chars[*idx..*idx + 5] == ['f', 'a', 'l', 's', 'e'] {
        *idx += 5;
        Ok(JsonValue::Bool(false))
    } else {
        Err("Expected boolean".to_string())
    }
}

fn parse_string(chars: &[char], idx: &mut usize) -> Result<String, String> {
    *idx += 1; // skip opening quote
    let mut s = String::new();
    while *idx < chars.len() {
        let c = chars[*idx];
        *idx += 1;
        if c == '"' {
            return Ok(s);
        }
        if c == '\\' {
            if *idx >= chars.len() {
                return Err("Unterminated escape sequence in string".to_string());
            }
            let esc = chars[*idx];
            *idx += 1;
            match esc {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push('\x08'),
                'f' => s.push('\x0C'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                'u' => {
                    if *idx + 4 <= chars.len() {
                        let hex: String = chars[*idx..*idx + 4].iter().collect();
                        *idx += 4;
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                s.push(ch);
                                continue;
                            }
                        }
                    }
                    s.push('?');
                }
                _ => s.push(esc),
            }
        } else {
            s.push(c);
        }
    }
    Err("Unterminated string".to_string())
}

fn parse_number(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    let start = *idx;
    if chars[*idx] == '-' {
        *idx += 1;
    }
    while *idx < chars.len() && chars[*idx].is_ascii_digit() {
        *idx += 1;
    }
    if *idx < chars.len() && chars[*idx] == '.' {
        *idx += 1;
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }
    if *idx < chars.len() && (chars[*idx] == 'e' || chars[*idx] == 'E') {
        *idx += 1;
        if *idx < chars.len() && (chars[*idx] == '+' || chars[*idx] == '-') {
            *idx += 1;
        }
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }
    let num_str: String = chars[start..*idx].iter().collect();
    num_str
        .parse::<f64>()
        .map(JsonValue::Number)
        .map_err(|e| format!("Invalid number '{}': {}", num_str, e))
}

fn parse_array(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    *idx += 1; // skip '['
    let mut arr = Vec::new();
    skip_ws(chars, idx);
    if *idx < chars.len() && chars[*idx] == ']' {
        *idx += 1;
        return Ok(JsonValue::Array(arr));
    }

    loop {
        skip_ws(chars, idx);
        let val = parse_value(chars, idx)?;
        arr.push(val);
        skip_ws(chars, idx);
        if *idx >= chars.len() {
            return Err("Unterminated array".to_string());
        }
        if chars[*idx] == ']' {
            *idx += 1;
            break;
        }
        if chars[*idx] == ',' {
            *idx += 1;
        } else {
            return Err(format!("Expected ',' or ']' in array, got '{}'", chars[*idx]));
        }
    }

    Ok(JsonValue::Array(arr))
}

fn parse_object(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    *idx += 1; // skip '{'
    let mut map = BTreeMap::new();
    skip_ws(chars, idx);
    if *idx < chars.len() && chars[*idx] == '}' {
        *idx += 1;
        return Ok(JsonValue::Object(map));
    }

    loop {
        skip_ws(chars, idx);
        if *idx >= chars.len() || chars[*idx] != '"' {
            return Err("Expected string key in object".to_string());
        }
        let key = parse_string(chars, idx)?;
        skip_ws(chars, idx);
        if *idx >= chars.len() || chars[*idx] != ':' {
            return Err("Expected ':' after key in object".to_string());
        }
        *idx += 1; // skip ':'
        skip_ws(chars, idx);
        let val = parse_value(chars, idx)?;
        map.insert(key, val);
        skip_ws(chars, idx);
        if *idx >= chars.len() {
            return Err("Unterminated object".to_string());
        }
        if chars[*idx] == '}' {
            *idx += 1;
            break;
        }
        if chars[*idx] == ',' {
            *idx += 1;
        } else {
            return Err(format!("Expected ',' or '}}' in object, got '{}'", chars[*idx]));
        }
    }

    Ok(JsonValue::Object(map))
}
