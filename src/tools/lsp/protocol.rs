use super::json::JsonValue;
use std::collections::{BTreeMap, HashMap};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

impl ParameterInformation {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            documentation: None,
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("label".to_string(), JsonValue::String(self.label.clone()));
        if let Some(doc) = &self.documentation {
            map.insert("documentation".to_string(), JsonValue::String(doc.clone()));
        }
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformation>,
    pub active_parameter: Option<u32>,
}

impl SignatureInformation {
    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("label".to_string(), JsonValue::String(self.label.clone()));
        if let Some(doc) = &self.documentation {
            let mut doc_map = BTreeMap::new();
            doc_map.insert(
                "kind".to_string(),
                JsonValue::String("markdown".to_string()),
            );
            doc_map.insert("value".to_string(), JsonValue::String(doc.clone()));
            map.insert("documentation".to_string(), JsonValue::Object(doc_map));
        }
        let params_json = self.parameters.iter().map(|p| p.to_json()).collect();
        map.insert("parameters".to_string(), JsonValue::Array(params_json));
        if let Some(ap) = self.active_parameter {
            map.insert("activeParameter".to_string(), JsonValue::Number(ap as f64));
        }
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: u32,
    pub active_parameter: u32,
}

impl SignatureHelp {
    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        let sigs_json = self.signatures.iter().map(|s| s.to_json()).collect();
        map.insert("signatures".to_string(), JsonValue::Array(sigs_json));
        map.insert(
            "activeSignature".to_string(),
            JsonValue::Number(self.active_signature as f64),
        );
        map.insert(
            "activeParameter".to_string(),
            JsonValue::Number(self.active_parameter as f64),
        );
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

impl Default for WorkspaceEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        let mut changes_obj = BTreeMap::new();
        for (uri, edits) in &self.changes {
            let edits_json = edits.iter().map(|e| e.to_json()).collect();
            changes_obj.insert(uri.clone(), JsonValue::Array(edits_json));
        }
        map.insert("changes".to_string(), JsonValue::Object(changes_obj));
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlayHintKind {
    Type = 1,
    Parameter = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    pub position: Position,
    pub label: String,
    pub kind: Option<InlayHintKind>,
    pub padding_left: bool,
    pub padding_right: bool,
}

impl InlayHint {
    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        map.insert("position".to_string(), self.position.to_json());
        map.insert("label".to_string(), JsonValue::String(self.label.clone()));
        if let Some(kind) = &self.kind {
            let k_num = match kind {
                InlayHintKind::Type => 1.0,
                InlayHintKind::Parameter => 2.0,
            };
            map.insert("kind".to_string(), JsonValue::Number(k_num));
        }
        if self.padding_left {
            map.insert("paddingLeft".to_string(), JsonValue::Bool(true));
        }
        if self.padding_right {
            map.insert("paddingRight".to_string(), JsonValue::Bool(true));
        }
        JsonValue::Object(map)
    }
}

pub const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "type",          // 0
    "class",         // 1
    "enum",          // 2
    "interface",     // 3
    "struct",        // 4
    "typeParameter", // 5
    "parameter",     // 6
    "variable",      // 7
    "property",      // 8
    "enumMember",    // 9
    "function",      // 10
    "method",        // 11
    "keyword",       // 12
    "comment",       // 13
    "string",        // 14
    "number",        // 15
    "operator",      // 16
];

pub const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &[
    "declaration",    // 1 << 0 = 1
    "definition",     // 1 << 1 = 2
    "readonly",       // 1 << 2 = 4
    "defaultLibrary", // 1 << 3 = 8
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSemanticToken {
    pub line: u32,
    pub start_col: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTokens {
    pub data: Vec<u32>,
}

impl SemanticTokens {
    pub fn from_raw_tokens(mut raw: Vec<RawSemanticToken>) -> Self {
        raw.sort_by(|a, b| {
            if a.line != b.line {
                a.line.cmp(&b.line)
            } else {
                a.start_col.cmp(&b.start_col)
            }
        });

        let mut data = Vec::with_capacity(raw.len() * 5);
        let mut prev_line = 0;
        let mut prev_col = 0;

        for tok in raw {
            let delta_line = tok.line - prev_line;
            let delta_col = if delta_line == 0 {
                tok.start_col.saturating_sub(prev_col)
            } else {
                tok.start_col
            };

            data.push(delta_line);
            data.push(delta_col);
            data.push(tok.length);
            data.push(tok.token_type);
            data.push(tok.token_modifiers);

            prev_line = tok.line;
            prev_col = tok.start_col;
        }

        Self { data }
    }

    pub fn to_json(&self) -> JsonValue {
        let mut map = BTreeMap::new();
        let nums: Vec<JsonValue> = self
            .data
            .iter()
            .map(|&n| JsonValue::Number(n as f64))
            .collect();
        map.insert("data".to_string(), JsonValue::Array(nums));
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
