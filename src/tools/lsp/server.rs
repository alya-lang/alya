use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};

use super::analysis::{
    check_document, find_references_for_word, format_document, get_completions, get_definition_pos,
    get_document_symbols, get_folding_ranges, get_hover, get_inlay_hints, get_semantic_tokens,
    get_signature_help, get_word_at_pos, prepare_rename, rename_symbol,
};
use super::json::JsonValue;
use super::protocol::{make_error, make_notification, make_response, Position, Range, TextEdit};

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
                capabilities.insert("codeActionProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert("documentSymbolProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert(
                    "documentFormattingProvider".to_string(),
                    JsonValue::Bool(true),
                );
                capabilities.insert("referencesProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert("foldingRangeProvider".to_string(), JsonValue::Bool(true));

                // Milestone 3 capabilities
                let mut sig_provider = BTreeMap::new();
                sig_provider.insert(
                    "triggerCharacters".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::String("(".to_string()),
                        JsonValue::String(",".to_string()),
                    ]),
                );
                capabilities.insert(
                    "signatureHelpProvider".to_string(),
                    JsonValue::Object(sig_provider),
                );

                let mut rename_provider = BTreeMap::new();
                rename_provider.insert("prepareProvider".to_string(), JsonValue::Bool(true));
                capabilities.insert(
                    "renameProvider".to_string(),
                    JsonValue::Object(rename_provider),
                );

                capabilities.insert("inlayHintProvider".to_string(), JsonValue::Bool(true));

                let mut legend = BTreeMap::new();
                let types_json = super::protocol::SEMANTIC_TOKEN_TYPES
                    .iter()
                    .map(|s| JsonValue::String(s.to_string()))
                    .collect();
                legend.insert("tokenTypes".to_string(), JsonValue::Array(types_json));
                let mods_json = super::protocol::SEMANTIC_TOKEN_MODIFIERS
                    .iter()
                    .map(|s| JsonValue::String(s.to_string()))
                    .collect();
                legend.insert("tokenModifiers".to_string(), JsonValue::Array(mods_json));

                let mut sem_provider = BTreeMap::new();
                sem_provider.insert("legend".to_string(), JsonValue::Object(legend));
                sem_provider.insert("full".to_string(), JsonValue::Bool(true));
                capabilities.insert(
                    "semanticTokensProvider".to_string(),
                    JsonValue::Object(sem_provider),
                );

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
            "textDocument/codeAction" => {
                let actions = self.handle_code_action(params);
                make_response(
                    id,
                    JsonValue::Array(actions.into_iter().map(|a| a.to_json()).collect()),
                )
            }
            "textDocument/documentSymbol" => {
                let symbols = self.handle_document_symbols(params);
                make_response(
                    id,
                    JsonValue::Array(symbols.into_iter().map(|s| s.to_json()).collect()),
                )
            }
            "textDocument/formatting" => {
                let edits = self.handle_formatting(params);
                make_response(
                    id,
                    JsonValue::Array(edits.into_iter().map(|e| e.to_json()).collect()),
                )
            }
            "textDocument/references" => {
                let refs = self.handle_references(params);
                make_response(
                    id,
                    JsonValue::Array(refs.into_iter().map(|r| r.to_json()).collect()),
                )
            }
            "textDocument/foldingRange" => {
                let ranges = self.handle_folding_ranges(params);
                make_response(
                    id,
                    JsonValue::Array(ranges.into_iter().map(|r| r.to_json()).collect()),
                )
            }
            "textDocument/signatureHelp" => {
                let sig_opt = self.handle_signature_help(params);
                if let Some(sig) = sig_opt {
                    make_response(id, sig.to_json())
                } else {
                    make_response(id, JsonValue::Null)
                }
            }
            "textDocument/prepareRename" => {
                let range_opt = self.handle_prepare_rename(params);
                if let Some(range) = range_opt {
                    make_response(id, range.to_json())
                } else {
                    make_response(id, JsonValue::Null)
                }
            }
            "textDocument/rename" => {
                let edit_opt = self.handle_rename(params);
                if let Some(edit) = edit_opt {
                    make_response(id, edit.to_json())
                } else {
                    make_response(id, JsonValue::Null)
                }
            }
            "textDocument/inlayHint" => {
                let hints = self.handle_inlay_hint(params);
                make_response(
                    id,
                    JsonValue::Array(hints.into_iter().map(|h| h.to_json()).collect()),
                )
            }
            "textDocument/semanticTokens/full" => {
                let tokens = self.handle_semantic_tokens_full(params);
                make_response(id, tokens.to_json())
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
        let path = super::protocol::uri_to_path(uri);
        let diags = check_document(source, Some(&path));
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

    fn handle_code_action(&self, params: Option<&JsonValue>) -> Vec<super::protocol::CodeAction> {
        let params = match params {
            Some(p) => p,
            None => return Vec::new(),
        };

        let uri = match params
            .get("textDocument")
            .and_then(|doc| doc.get("uri"))
            .and_then(|u| u.as_str())
        {
            Some(u) => u,
            None => return Vec::new(),
        };

        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let path = super::protocol::uri_to_path(uri);
        let diags = match crate::tools::lint::lint_source(source, &path) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut actions = Vec::new();
        for d in diags {
            if let Some(fix) = d.fix {
                let start_line = if fix.start_line > 0 {
                    (fix.start_line - 1) as u32
                } else {
                    0
                };
                let start_col = if fix.start_col > 0 {
                    (fix.start_col - 1) as u32
                } else {
                    0
                };
                let end_line = if fix.end_line > 0 {
                    (fix.end_line - 1) as u32
                } else {
                    start_line
                };
                let end_col = if fix.end_col > 0 {
                    (fix.end_col - 1) as u32
                } else {
                    start_col + 1
                };

                let text_edit = super::protocol::TextEdit {
                    range: super::protocol::Range::new(
                        super::protocol::Position::new(start_line, start_col),
                        super::protocol::Position::new(end_line, end_col),
                    ),
                    new_text: fix.replacement,
                };

                actions.push(super::protocol::CodeAction {
                    title: fix.description,
                    kind: "quickfix".to_string(),
                    is_preferred: true,
                    edits: vec![(uri.to_string(), text_edit)],
                });
            }
        }

        actions
    }

    fn handle_document_symbols(
        &self,
        params: Option<&JsonValue>,
    ) -> Vec<super::protocol::DocumentSymbol> {
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

        if let Some(source) = self.documents.get(uri) {
            get_document_symbols(source)
        } else {
            Vec::new()
        }
    }

    fn handle_formatting(&self, params: Option<&JsonValue>) -> Vec<TextEdit> {
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

        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return Vec::new(),
        };

        if let Some(formatted) = format_document(source) {
            let lines: Vec<&str> = source.lines().collect();
            let (end_line, end_col) = if lines.is_empty() {
                (0, 0)
            } else if source.ends_with('\n') {
                (lines.len() as u32, 0)
            } else {
                ((lines.len() - 1) as u32, lines.last().unwrap().len() as u32)
            };

            vec![TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(end_line, end_col)),
                new_text: formatted,
            }]
        } else {
            Vec::new()
        }
    }

    fn handle_references(&self, params: Option<&JsonValue>) -> Vec<super::protocol::Location> {
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

        let source = match self.documents.get(uri) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let word = match get_word_at_pos(source, &pos) {
            Some(w) => w,
            None => return Vec::new(),
        };

        let mut all_refs = Vec::new();
        for (doc_uri, doc_source) in &self.documents {
            let doc_refs = find_references_for_word(doc_source, &word, doc_uri);
            all_refs.extend(doc_refs);
        }
        all_refs
    }

    fn handle_folding_ranges(
        &self,
        params: Option<&JsonValue>,
    ) -> Vec<super::protocol::FoldingRange> {
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

        if let Some(source) = self.documents.get(uri) {
            get_folding_ranges(source)
        } else {
            Vec::new()
        }
    }

    fn handle_signature_help(
        &self,
        params: Option<&JsonValue>,
    ) -> Option<super::protocol::SignatureHelp> {
        let params = params?;
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let pos = Position::from_json(params.get("position")?)?;
        let source = self.documents.get(uri)?;
        get_signature_help(source, &pos)
    }

    fn handle_prepare_rename(&self, params: Option<&JsonValue>) -> Option<Range> {
        let params = params?;
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let pos = Position::from_json(params.get("position")?)?;
        let source = self.documents.get(uri)?;
        prepare_rename(source, &pos)
    }

    fn handle_rename(&self, params: Option<&JsonValue>) -> Option<super::protocol::WorkspaceEdit> {
        let params = params?;
        let uri = params.get("textDocument")?.get("uri")?.as_str()?;
        let pos = Position::from_json(params.get("position")?)?;
        let new_name = params.get("newName")?.as_str()?;
        rename_symbol(&self.documents, uri, &pos, new_name)
    }

    fn handle_inlay_hint(&self, params: Option<&JsonValue>) -> Vec<super::protocol::InlayHint> {
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
        if let Some(source) = self.documents.get(uri) {
            get_inlay_hints(source)
        } else {
            Vec::new()
        }
    }

    fn handle_semantic_tokens_full(
        &self,
        params: Option<&JsonValue>,
    ) -> super::protocol::SemanticTokens {
        let default_empty = super::protocol::SemanticTokens { data: Vec::new() };
        let params = match params {
            Some(p) => p,
            None => return default_empty,
        };
        let uri = match params
            .get("textDocument")
            .and_then(|td| td.get("uri"))
            .and_then(|u| u.as_str())
        {
            Some(u) => u,
            None => return default_empty,
        };
        if let Some(source) = self.documents.get(uri) {
            get_semantic_tokens(source)
        } else {
            default_empty
        }
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
    loop {
        let mut content_length: Option<usize> = None;

        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                // True EOF reached on stdin
                if content_length.is_none() {
                    return Ok(None);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Unexpected EOF while reading LSP headers",
                    ));
                }
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                if content_length.is_some() {
                    // Empty line marks end of headers
                    break;
                }
                // Skip leading blank lines/newlines between messages
                continue;
            }

            let lower = trimmed.to_ascii_lowercase();
            if let Some(val_str) = lower.strip_prefix("content-length:") {
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
            Ok(val) => return Ok(Some(val)),
            Err(e) => {
                eprintln!("[alya-lsp] JSON parse error: {}", e);
                // Continue loop to read next message instead of terminating the server
                continue;
            }
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
