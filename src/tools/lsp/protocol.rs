use super::json::JsonValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("line".to_string(), JsonValue::Number(self.line as f64));
        map.insert(
            "character".to_string(),
            JsonValue::Number(self.character as f64),
        );
        JsonValue::Object(map)
    }

    pub fn from_json(json: &JsonValue) -> Option<Self> {
        let line = json.get("line")?.as_i64()? as u32;
        let character = json.get("character")?.as_i64()? as u32;
        Some(Self { line, character })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self {
            start: Position::new(line, start_col),
            end: Position::new(line, end_col),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("start".to_string(), self.start.to_json());
        map.insert("end".to_string(), self.end.to_json());
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: u32, // 1 = Error, 2 = Warning, 3 = Info
    pub code: Option<String>,
    pub message: String,
    pub source: String,
}

impl Diagnostic {
    pub fn error(range: Range, message: String) -> Self {
        Self {
            range,
            severity: 1,
            code: None,
            message,
            source: "alya".to_string(),
        }
    }

    pub fn warning(range: Range, code: Option<String>, message: String) -> Self {
        Self {
            range,
            severity: 2,
            code,
            message,
            source: "alya-lint".to_string(),
        }
    }

    pub fn info(range: Range, code: Option<String>, message: String) -> Self {
        Self {
            range,
            severity: 3,
            code,
            message,
            source: "alya-lint".to_string(),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("range".to_string(), self.range.to_json());
        map.insert(
            "severity".to_string(),
            JsonValue::Number(self.severity as f64),
        );
        if let Some(code) = &self.code {
            map.insert("code".to_string(), JsonValue::String(code.clone()));
        }
        map.insert(
            "message".to_string(),
            JsonValue::String(self.message.clone()),
        );
        map.insert("source".to_string(), JsonValue::String(self.source.clone()));
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

impl TextEdit {
    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("range".to_string(), self.range.to_json());
        map.insert(
            "newText".to_string(),
            JsonValue::String(self.new_text.clone()),
        );
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub kind: String,
    pub is_preferred: bool,
    pub edits: Vec<(String, TextEdit)>,
}

impl CodeAction {
    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("title".to_string(), JsonValue::String(self.title.clone()));
        map.insert("kind".to_string(), JsonValue::String(self.kind.clone()));
        map.insert(
            "isPreferred".to_string(),
            JsonValue::Bool(self.is_preferred),
        );

        let mut changes = BTreeMap::new();
        for (uri, edit) in &self.edits {
            changes
                .entry(uri.clone())
                .or_insert_with(Vec::new)
                .push(edit.to_json());
        }

        let mut changes_obj = BTreeMap::new();
        for (uri, list) in changes {
            changes_obj.insert(uri, JsonValue::Array(list));
        }

        let mut edit_obj = BTreeMap::new();
        edit_obj.insert("changes".to_string(), JsonValue::Object(changes_obj));
        map.insert("edit".to_string(), JsonValue::Object(edit_obj));

        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: u32,
    pub detail: Option<String>,
    pub doc: Option<String>,
}

impl CompletionItem {
    pub fn new(label: &str, kind: u32, detail: Option<&str>, doc: Option<&str>) -> Self {
        Self {
            label: label.to_string(),
            kind,
            detail: detail.map(|s| s.to_string()),
            doc: doc.map(|s| s.to_string()),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("label".to_string(), JsonValue::String(self.label.clone()));
        map.insert("kind".to_string(), JsonValue::Number(self.kind as f64));
        if let Some(d) = &self.detail {
            map.insert("detail".to_string(), JsonValue::String(d.clone()));
        }
        if let Some(doc) = &self.doc {
            let mut doc_map = BTreeMap::new();
            doc_map.insert(
                "kind".to_string(),
                JsonValue::String("markdown".to_string()),
            );
            doc_map.insert("value".to_string(), JsonValue::String(doc.clone()));
            map.insert("documentation".to_string(), JsonValue::Object(doc_map));
        }
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(
        name: &str,
        detail: Option<&str>,
        kind: u32,
        range: Range,
        selection_range: Range,
    ) -> Self {
        Self {
            name: name.to_string(),
            detail: detail.map(|s| s.to_string()),
            kind,
            range,
            selection_range,
            children: Vec::new(),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("name".to_string(), JsonValue::String(self.name.clone()));
        if let Some(detail) = &self.detail {
            map.insert("detail".to_string(), JsonValue::String(detail.clone()));
        }
        map.insert("kind".to_string(), JsonValue::Number(self.kind as f64));
        map.insert("range".to_string(), self.range.to_json());
        map.insert("selectionRange".to_string(), self.selection_range.to_json());
        if !self.children.is_empty() {
            let ch_json = self.children.iter().map(|c| c.to_json()).collect();
            map.insert("children".to_string(), JsonValue::Array(ch_json));
        }
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: String, range: Range) -> Self {
        Self { uri, range }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("uri".to_string(), JsonValue::String(self.uri.clone()));
        map.insert("range".to_string(), self.range.to_json());
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldingRange {
    pub start_line: u32,
    pub start_character: Option<u32>,
    pub end_line: u32,
    pub end_character: Option<u32>,
    pub kind: Option<String>,
}

impl FoldingRange {
    pub fn new(start_line: u32, end_line: u32, kind: Option<&str>) -> Self {
        Self {
            start_line,
            start_character: None,
            end_line,
            end_character: None,
            kind: kind.map(|s| s.to_string()),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert(
            "startLine".to_string(),
            JsonValue::Number(self.start_line as f64),
        );
        if let Some(sc) = self.start_character {
            map.insert("startCharacter".to_string(), JsonValue::Number(sc as f64));
        }
        map.insert(
            "endLine".to_string(),
            JsonValue::Number(self.end_line as f64),
        );
        if let Some(ec) = self.end_character {
            map.insert("endCharacter".to_string(), JsonValue::Number(ec as f64));
        }
        if let Some(kind) = &self.kind {
            map.insert("kind".to_string(), JsonValue::String(kind.clone()));
        }
        JsonValue::Object(map)
    }
}

pub fn make_response(id: &JsonValue, result: JsonValue) -> JsonValue {
    let mut map = BTreeMap::new();
    map.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    map.insert("id".to_string(), id.clone());
    map.insert("result".to_string(), result);
    JsonValue::Object(map)
}

pub fn make_error(id: &JsonValue, code: i64, message: &str) -> JsonValue {
    let mut map = BTreeMap::new();
    map.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    map.insert("id".to_string(), id.clone());
    let mut err = BTreeMap::new();
    err.insert("code".to_string(), JsonValue::Number(code as f64));
    err.insert(
        "message".to_string(),
        JsonValue::String(message.to_string()),
    );
    map.insert("error".to_string(), JsonValue::Object(err));
    JsonValue::Object(map)
}

pub fn make_notification(method: &str, params: JsonValue) -> JsonValue {
    let mut map = BTreeMap::new();
    map.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    map.insert("method".to_string(), JsonValue::String(method.to_string()));
    map.insert("params".to_string(), params);
    JsonValue::Object(map)
}

/// Converts an LSP URI (e.g. `file:///C:/foo/bar.alya`, `file:///c%3A/foo/bar.alya`, or `file:///home/user/bar.alya`)
/// to a normalized, percent-decoded `std::path::PathBuf`.
pub fn uri_to_path(uri: &str) -> std::path::PathBuf {
    let mut s = uri;
    if let Some(stripped) = s.strip_prefix("file://localhost") {
        s = stripped;
    } else if let Some(stripped) = s.strip_prefix("file://") {
        s = stripped;
    }

    // Percent-decode (%20 -> ' ', %3A -> ':', etc.)
    let mut decoded = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte_val) = u8::from_str_radix(hex_str, 16) {
                    decoded.push(byte_val as char);
                    i += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[i] as char);
        i += 1;
    }

    // On Windows, file URIs usually start with `/` before the drive letter (e.g. `/C:/path` or `/c:/path`).
    // Strip leading slash if followed by an ASCII alphabetic drive letter and colon.
    let path_str = if decoded.starts_with('/')
        && decoded.len() > 2
        && decoded.as_bytes()[1].is_ascii_alphabetic()
        && decoded.as_bytes()[2] == b':'
    {
        &decoded[1..]
    } else {
        &decoded
    };

    std::path::PathBuf::from(path_str)
}
