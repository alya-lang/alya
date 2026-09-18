use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};

use super::analysis::{check_document, get_completions, get_definition_pos, get_hover};
use super::json::JsonValue;
use super::protocol::{make_error, make_notification, make_response, Position};

pub struct ServerState {
    pub documents: HashMap<String, String>,
    pub is_shutdown: bool,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            is_shutdown: false,
        }
    }

    pub fn handle_message(&mut self, msg: &JsonValue) -> Option<JsonValue> {
        let method = msg.get("method").and_then(|m| m.as_str());
        let id = msg.get("id");

        // Requests have both method and id
        if let (Some(method), Some(id)) = (method, id) {
            return Some(self.handle_request(method, id, msg.get("params")));
        }

        // Notifications have method but no id
        if let Some(method) = method {
            return self.handle_notification(method, msg.get("params"));
        }

        None
    }

    fn handle_request(
        &mut self,
        method: &str,
        id: &JsonValue,
        params: Option<&JsonValue>,
    ) -> JsonValue {
        match method {
            "initialize" => {
                let mut capabilities = BTreeMap::new();
                // 1 = Full sync
                capabilities.insert("textDocumentSync".to_string(), JsonValue::Number(1.0));

                let mut completion_provider = BTreeMap::new();
                completion_provider.insert("resolveProvider".to_string(), JsonValue::Bool(false));
                let triggers = vec![
                    JsonValue::String(".".to_string()),
                    JsonValue::String(":".to_string()),
                ];
                completion_provider
                    .insert("triggerCharacters".to_string(), JsonValue::Array(triggers));
                capabilities.insert(
                    "completionProvider".to_string(),
                    JsonValue::Object(completion_provider),
                );

                capabilities.insert("hoverProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert("definitionProvider".to_string(), JsonValue::Bool(true));

                let mut server_info = BTreeMap::new();
                server_info.insert(
                    "name".to_string(),
                    JsonValue::String("alya-lsp".to_string()),
                );
                server_info.insert(
                    "version".to_string(),
                    JsonValue::String("1.0.0".to_string()),
                );

                let mut result = BTreeMap::new();
                result.insert("capabilities".to_string(), JsonValue::Object(capabilities));
                result.insert("serverInfo".to_string(), JsonValue::Object(server_info));

                make_response(id, JsonValue::Object(result))
            }
            "shutdown" => {
                self.is_shutdown = true;
                make_response(id, JsonValue::Null)
            }
            "textDocument/completion" => {
                let items = self.handle_completion(params);
                make_response(
                    id,
                    JsonValue::Array(items.into_iter().map(|it| it.to_json()).collect()),
                )
            }
            "textDocument/hover" => {
                let hover_opt = self.handle_hover(params);
                if let Some(doc) = hover_opt {
                    let mut contents = BTreeMap::new();
                    contents.insert(
                        "kind".to_string(),
                        JsonValue::String("markdown".to_string()),
                    );
                    contents.insert("value".to_string(), JsonValue::String(doc));

                    let mut res = BTreeMap::new();
                    res.insert("contents".to_string(), JsonValue::Object(contents));
                    make_response(id, JsonValue::Object(res))
                } else {
                    make_response(id, JsonValue::Null)
                }
            }
            "textDocument/definition" => {
                let def_opt = self.handle_definition(params);
                if let Some((uri, pos)) = def_opt {
                    let mut res = BTreeMap::new();
                    res.insert("uri".to_string(), JsonValue::String(uri));
                    let mut range = BTreeMap::new();
                    range.insert("start".to_string(), pos.to_json());
                    range.insert("end".to_string(), pos.to_json());
                    res.insert("range".to_string(), JsonValue::Object(range));
                    make_response(id, JsonValue::Object(res))
                } else {
                    make_response(id, JsonValue::Null)
                }
            }
            _ => make_error(id, -32601, &format!("Method not found: {}", method)),
        }
    }

    fn handle_notification(
        &mut self,
        method: &str,
        params: Option<&JsonValue>,
    ) -> Option<JsonValue> {
        match method {
            "initialized" => None,
            "exit" => {
                self.is_shutdown = true;
                None
            }
            "textDocument/didOpen" => {
                let doc = params?.get("textDocument")?;
                let uri = doc.get("uri")?.as_str()?;
                let text = doc.get("text")?.as_str()?;
                self.documents.insert(uri.to_string(), text.to_string());
                Some(self.publish_diagnostics(uri, text))
            }
            "textDocument/didChange" => {
                let doc = params?.get("textDocument")?;
                let uri = doc.get("uri")?.as_str()?;
                let changes = params?.get("contentChanges")?.as_array()?;
                if let Some(first_change) = changes.last() {
                    if let Some(text) = first_change.get("text").and_then(|t| t.as_str()) {
                        self.documents.insert(uri.to_string(), text.to_string());
                        return Some(self.publish_diagnostics(uri, text));
                    }
                }
                None
            }
            "textDocument/didClose" => {
                let doc = params?.get("textDocument")?;
                if let Some(uri) = doc.get("uri").and_then(|u| u.as_str()) {
                    self.documents.remove(uri);
                    // Publish empty diagnostics to clear IDE markers
                    return Some(self.publish_empty_diagnostics(uri));
                }
                None
            }
            _ => None,
        }
    }

    pub fn publish_diagnostics(&self, uri: &str, source: &str) -> JsonValue {
        let diags = check_document(source);
        let mut params = BTreeMap::new();
        params.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        let diags_json = diags.into_iter().map(|d| d.to_json()).collect();
        params.insert("diagnostics".to_string(), JsonValue::Array(diags_json));
        make_notification("textDocument/publishDiagnostics", JsonValue::Object(params))
    }

    pub fn publish_empty_diagnostics(&self, uri: &str) -> JsonValue {
        let mut params = BTreeMap::new();
        params.insert("uri".to_string(), JsonValue::String(uri.to_string()));
        params.insert("diagnostics".to_string(), JsonValue::Array(Vec::new()));
        make_notification("textDocument/publishDiagnostics", JsonValue::Object(params))
    }

    fn handle_completion(
        &self,
        params: Option<&JsonValue>,
    ) -> Vec<super::protocol::CompletionItem> {
        let params = match params {
            Some(p) => p,
            None => return Vec::new(),
        };
        let uri = match params
            .get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
        {
            Some(u) => u,
            None => return Vec::new(),
        };
        let pos = match params.get("position").and_then(Position::from_json) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if let Some(source) = self.documents.get(uri) {
            get_completions(source, &pos)
        } else {
            Vec::new()
        }
    }

    fn handle_hover(&self, params: Option<&JsonValue>) -> Option<String> {
        let params = params?;
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let pos = Position::from_json(params.get("position")?)?;
        let source = self.documents.get(uri)?;
        get_hover(source, &pos)
    }

    fn handle_definition(&self, params: Option<&JsonValue>) -> Option<(String, Position)> {
        let params = params?;
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let pos = Position::from_json(params.get("position")?)?;
        let source = self.documents.get(uri)?;
        let def_pos = get_definition_pos(source, &pos)?;
        Some((uri.to_string(), def_pos))
    }
}

pub fn send_framed_message<W: Write>(writer: &mut W, msg: &JsonValue) -> io::Result<()> {
    let payload = msg.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(payload.as_bytes())?;
    writer.flush()
}

pub fn read_framed_message<R: BufRead>(reader: &mut R) -> io::Result<Option<JsonValue>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            // EOF
            return Ok(None);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            // Empty line marks end of headers
            break;
        }

        if let Some(val_str) = trimmed.strip_prefix("Content-Length:") {
            if let Ok(len) = val_str.trim().parse::<usize>() {
                content_length = Some(len);
            }
        }
    }

    let length = match content_length {
        Some(l) => l,
        None => return Ok(None),
    };

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;

    let text = String::from_utf8_lossy(&body);
    match JsonValue::parse(&text) {
        Ok(val) => Ok(Some(val)),
        Err(e) => {
            eprintln!("[alya-lsp] JSON parse error: {}", e);
            Ok(None)
        }
    }
}

pub fn run_server() -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    let mut state = ServerState::new();

    while !state.is_shutdown {
        match read_framed_message(&mut reader) {
            Ok(Some(msg)) => {
                if let Some(resp) = state.handle_message(&msg) {
                    if let Err(e) = send_framed_message(&mut writer, &resp) {
                        return Err(format!("Failed to write LSP message: {}", e));
                    }
                }
            }
            Ok(None) => break, // EOF reached
            Err(e) => {
                return Err(format!("LSP transport error: {}", e));
            }
        }
    }

    Ok(())
}
