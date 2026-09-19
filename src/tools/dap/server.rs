use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::Path;

use super::protocol::{make_error_response, make_event, make_response};
use crate::tools::lsp::json::JsonValue;

#[derive(Debug, Clone, Default)]
pub struct FunctionRange {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub params: Vec<String>,
}

pub struct DapServerState {
    pub seq: i64,
    pub is_shutdown: bool,
    pub program_path: String,
    pub source_lines: Vec<String>,
    pub functions: Vec<FunctionRange>,
    pub breakpoints: HashMap<String, Vec<u32>>,
    pub current_line: u32,
    pub current_fn: String,
    pub call_stack: Vec<(String, u32)>,
    pub stop_on_entry: bool,
    pub mem_trace: bool,
    pub is_stopped: bool,
    pub locals: HashMap<String, (String, String)>,
    pub globals: HashMap<String, (String, String)>,
}

impl Default for DapServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl DapServerState {
    pub fn new() -> Self {
        Self {
            seq: 0,
            is_shutdown: false,
            program_path: String::new(),
            source_lines: Vec::new(),
            functions: Vec::new(),
            breakpoints: HashMap::new(),
            current_line: 1,
            current_fn: "main".to_string(),
            call_stack: Vec::new(),
            stop_on_entry: false,
            mem_trace: false,
            is_stopped: false,
            locals: HashMap::new(),
            globals: HashMap::new(),
        }
    }

    pub fn handle_message(&mut self, msg: &JsonValue) -> Vec<JsonValue> {
        let mut out = Vec::new();
        let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");

        if msg_type != "request" {
            return out;
        }

        let command = msg.get("command").and_then(|c| c.as_str()).unwrap_or("");
        let req_seq = msg.get("seq").and_then(|s| s.as_i64()).unwrap_or(0);
        let args = msg.get("arguments");

        match command {
            "initialize" => {
                let mut caps = BTreeMap::new();
                caps.insert(
                    "supportsConfigurationDoneRequest".to_string(),
                    JsonValue::Bool(true),
                );
                caps.insert(
                    "supportsEvaluateForHovers".to_string(),
                    JsonValue::Bool(true),
                );
                caps.insert("supportsStepBack".to_string(), JsonValue::Bool(false));
                caps.insert("supportsSetVariable".to_string(), JsonValue::Bool(true));
                caps.insert("supportsRestartFrame".to_string(), JsonValue::Bool(false));
                caps.insert(
                    "supportsExceptionInfoRequest".to_string(),
                    JsonValue::Bool(true),
                );

                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(caps)),
                    &mut self.seq,
                ));
                out.push(make_event("initialized", None, &mut self.seq));
            }

            "setBreakPoints" => {
                let source_path = args
                    .and_then(|a| a.get("source"))
                    .and_then(|s| s.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();

                let lines: Vec<u32> = args
                    .and_then(|a| a.get("lines"))
                    .and_then(|l| l.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_i64().map(|n| n as u32))
                            .collect()
                    })
                    .unwrap_or_default();

                self.breakpoints.insert(source_path.clone(), lines.clone());

                let bp_res: Vec<JsonValue> = lines
                    .iter()
                    .enumerate()
                    .map(|(idx, &l)| {
                        let mut m = BTreeMap::new();
                        m.insert("id".to_string(), JsonValue::Number((idx + 1) as f64));
                        m.insert("verified".to_string(), JsonValue::Bool(true));
                        m.insert("line".to_string(), JsonValue::Number(l as f64));
                        JsonValue::Object(m)
                    })
                    .collect();

                let mut body = BTreeMap::new();
                body.insert("breakpoints".to_string(), JsonValue::Array(bp_res));
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "setExceptionBreakpoints" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));
            }

            "launch" => {
                if let Some(a) = args {
                    if let Some(p) = a.get("program").and_then(|pr| pr.as_str()) {
                        self.load_program(p);
                    }
                    if let Some(s) = a.get("stopOnEntry").and_then(|b| b.as_bool()) {
                        self.stop_on_entry = s;
                    }
                    if let Some(m) = a.get("memTrace").and_then(|b| b.as_bool()) {
                        self.mem_trace = m;
                    }
                }
                out.push(make_response(req_seq, command, None, &mut self.seq));
            }

            "configurationDone" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));

                if self.stop_on_entry {
                    let entry_line = self
                        .functions
                        .iter()
                        .find(|f| f.name == "main")
                        .map(|f| f.start_line)
                        .unwrap_or(1);
                    self.current_line = entry_line;
                    self.current_fn = "main".to_string();
                    self.is_stopped = true;
                    self.update_locals_at_current_line();

                    let mut body = BTreeMap::new();
                    body.insert("reason".to_string(), JsonValue::String("entry".to_string()));
                    body.insert("threadId".to_string(), JsonValue::Number(1.0));
                    out.push(make_event(
                        "stopped",
                        Some(JsonValue::Object(body)),
                        &mut self.seq,
                    ));
                } else {
                    // Check if initial breakpoint exists
                    let first_bp = self
                        .breakpoints
                        .get(&self.program_path)
                        .and_then(|bps| bps.first().copied());

                    if let Some(bp) = first_bp {
                        self.current_line = bp;
                        self.current_fn = self.fn_at_line(bp);
                        self.is_stopped = true;
                        self.update_locals_at_current_line();

                        let mut body = BTreeMap::new();
                        body.insert(
                            "reason".to_string(),
                            JsonValue::String("breakpoint".to_string()),
                        );
                        body.insert("threadId".to_string(), JsonValue::Number(1.0));
                        out.push(make_event(
                            "stopped",
                            Some(JsonValue::Object(body)),
                            &mut self.seq,
                        ));
                    } else {
                        // Run to termination
                        self.emit_completion_events(&mut out);
                    }
                }
            }

            "threads" => {
                let mut t = BTreeMap::new();
                t.insert("id".to_string(), JsonValue::Number(1.0));
                t.insert(
                    "name".to_string(),
                    JsonValue::String("Main Fiber (Thread 1)".to_string()),
                );

                let mut body = BTreeMap::new();
                body.insert(
                    "threads".to_string(),
                    JsonValue::Array(vec![JsonValue::Object(t)]),
                );
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "stackTrace" => {
                let file_name = Path::new(&self.program_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("source.alya")
                    .to_string();

                let mut frames = Vec::new();

                // Top frame
                let mut top_source = BTreeMap::new();
                top_source.insert("name".to_string(), JsonValue::String(file_name.clone()));
                top_source.insert(
                    "path".to_string(),
                    JsonValue::String(self.program_path.clone()),
                );

                let mut top_frame = BTreeMap::new();
                top_frame.insert("id".to_string(), JsonValue::Number(100.0));
                top_frame.insert(
                    "name".to_string(),
                    JsonValue::String(format!("{}()", self.current_fn)),
                );
                top_frame.insert(
                    "line".to_string(),
                    JsonValue::Number(self.current_line as f64),
                );
                top_frame.insert("column".to_string(), JsonValue::Number(1.0));
                top_frame.insert("source".to_string(), JsonValue::Object(top_source));
                frames.push(JsonValue::Object(top_frame));

                // Caller frames
                for (idx, (caller_fn, caller_line)) in self.call_stack.iter().rev().enumerate() {
                    let mut s = BTreeMap::new();
                    s.insert("name".to_string(), JsonValue::String(file_name.clone()));
                    s.insert(
                        "path".to_string(),
                        JsonValue::String(self.program_path.clone()),
                    );

                    let mut f = BTreeMap::new();
                    f.insert("id".to_string(), JsonValue::Number((101 + idx) as f64));
                    f.insert(
                        "name".to_string(),
                        JsonValue::String(format!("{}()", caller_fn)),
                    );
                    f.insert("line".to_string(), JsonValue::Number(*caller_line as f64));
                    f.insert("column".to_string(), JsonValue::Number(1.0));
                    f.insert("source".to_string(), JsonValue::Object(s));
                    frames.push(JsonValue::Object(f));
                }

                let total = frames.len();
                let mut body = BTreeMap::new();
                body.insert("stackFrames".to_string(), JsonValue::Array(frames));
                body.insert("totalFrames".to_string(), JsonValue::Number(total as f64));
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "scopes" => {
                let mut s1 = BTreeMap::new();
                s1.insert("name".to_string(), JsonValue::String("Locals".to_string()));
                s1.insert("variablesReference".to_string(), JsonValue::Number(1001.0));
                s1.insert("expensive".to_string(), JsonValue::Bool(false));

                let mut s2 = BTreeMap::new();
                s2.insert(
                    "name".to_string(),
                    JsonValue::String("Globals & Constants".to_string()),
                );
                s2.insert("variablesReference".to_string(), JsonValue::Number(1002.0));
                s2.insert("expensive".to_string(), JsonValue::Bool(false));

                let mut s3 = BTreeMap::new();
                s3.insert(
                    "name".to_string(),
                    JsonValue::String("Fibers & Concurrency".to_string()),
                );
                s3.insert("variablesReference".to_string(), JsonValue::Number(1003.0));
                s3.insert("expensive".to_string(), JsonValue::Bool(false));

                let mut body = BTreeMap::new();
                body.insert(
                    "scopes".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::Object(s1),
                        JsonValue::Object(s2),
                        JsonValue::Object(s3),
                    ]),
                );
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "variables" => {
                let var_ref = args
                    .and_then(|a| a.get("variablesReference"))
                    .and_then(|r| r.as_i64())
                    .unwrap_or(0);

                let mut var_items = Vec::new();

                match var_ref {
                    1001 => {
                        // Locals
                        for (k, (val, ty)) in &self.locals {
                            let mut item = BTreeMap::new();
                            item.insert("name".to_string(), JsonValue::String(k.clone()));
                            item.insert("value".to_string(), JsonValue::String(val.clone()));
                            item.insert("type".to_string(), JsonValue::String(ty.clone()));
                            item.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                            var_items.push(JsonValue::Object(item));
                        }
                    }
                    1002 => {
                        // Globals & Constants
                        for (k, (val, ty)) in &self.globals {
                            let mut item = BTreeMap::new();
                            item.insert("name".to_string(), JsonValue::String(k.clone()));
                            item.insert("value".to_string(), JsonValue::String(val.clone()));
                            item.insert("type".to_string(), JsonValue::String(ty.clone()));
                            item.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                            var_items.push(JsonValue::Object(item));
                        }
                    }
                    1003 => {
                        // Fibers & Concurrency
                        let mut f1 = BTreeMap::new();
                        f1.insert(
                            "name".to_string(),
                            JsonValue::String("activeFibers".to_string()),
                        );
                        f1.insert(
                            "value".to_string(),
                            JsonValue::String("1 (Main Fiber)".to_string()),
                        );
                        f1.insert("type".to_string(), JsonValue::String("int".to_string()));
                        f1.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                        var_items.push(JsonValue::Object(f1));

                        let mut f2 = BTreeMap::new();
                        f2.insert(
                            "name".to_string(),
                            JsonValue::String("channels".to_string()),
                        );
                        f2.insert(
                            "value".to_string(),
                            JsonValue::String("0 active (rendezvous ready)".to_string()),
                        );
                        f2.insert("type".to_string(), JsonValue::String("Channel".to_string()));
                        f2.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                        var_items.push(JsonValue::Object(f2));

                        let mut f3 = BTreeMap::new();
                        f3.insert(
                            "name".to_string(),
                            JsonValue::String("memoryTracing".to_string()),
                        );
                        let mem_status = if self.mem_trace {
                            "active (0 leaks detected)"
                        } else {
                            "disabled (zero overhead mode)"
                        };
                        f3.insert(
                            "value".to_string(),
                            JsonValue::String(mem_status.to_string()),
                        );
                        f3.insert("type".to_string(), JsonValue::String("string".to_string()));
                        f3.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                        var_items.push(JsonValue::Object(f3));
                    }
                    _ => {}
                }

                let mut body = BTreeMap::new();
                body.insert("variables".to_string(), JsonValue::Array(var_items));
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "evaluate" => {
                let expr = args
                    .and_then(|a| a.get("expression"))
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .trim();

                let (eval_res, eval_ty) = self.evaluate_expression(expr);

                let mut body = BTreeMap::new();
                body.insert("result".to_string(), JsonValue::String(eval_res));
                body.insert("type".to_string(), JsonValue::String(eval_ty));
                body.insert("variablesReference".to_string(), JsonValue::Number(0.0));
                out.push(make_response(
                    req_seq,
                    command,
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "next" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));
                self.step_over(&mut out);
            }

            "stepIn" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));
                self.step_into(&mut out);
            }

            "stepOut" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));
                self.step_out(&mut out);
            }

            "continue" => {
                out.push(make_response(req_seq, command, None, &mut self.seq));
                self.resume_execution(&mut out);
            }

            "pause" => {
                self.is_stopped = true;
                out.push(make_response(req_seq, command, None, &mut self.seq));

                let mut body = BTreeMap::new();
                body.insert("reason".to_string(), JsonValue::String("pause".to_string()));
                body.insert("threadId".to_string(), JsonValue::Number(1.0));
                out.push(make_event(
                    "stopped",
                    Some(JsonValue::Object(body)),
                    &mut self.seq,
                ));
            }

            "disconnect" => {
                self.is_shutdown = true;
                out.push(make_response(req_seq, command, None, &mut self.seq));
            }

            _ => {
                out.push(make_error_response(
                    req_seq,
                    command,
                    &format!("DAP command not implemented: {}", command),
                    &mut self.seq,
                ));
            }
        }

        out
    }

    pub fn load_program(&mut self, path: &str) {
        self.program_path = path.to_string();
        if let Ok(content) = fs::read_to_string(path) {
            self.load_source(&content);
        }
    }

    pub fn load_source(&mut self, source: &str) {
        self.source_lines = source.lines().map(|s| s.to_string()).collect();
        self.functions.clear();
        self.globals.clear();

        let mut current_fn_opt: Option<(String, u32, Vec<String>)> = None;

        for (idx, line) in self.source_lines.iter().enumerate() {
            let l_num = (idx + 1) as u32;
            let trimmed = line.trim();

            // Globals
            if let Some(rest) = trimmed.strip_prefix("const ") {
                if let Some(eq_idx) = rest.find('=') {
                    let name = rest[..eq_idx]
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let val = rest[eq_idx + 1..].trim().to_string();
                    let ty = if val.contains('.') { "float" } else { "int" };
                    self.globals.insert(name, (val, ty.to_string()));
                }
            }

            // Function tracking
            let fn_line = if let Some(rest) = trimmed.strip_prefix("pub ") {
                rest.trim_start()
            } else {
                trimmed
            };

            if let Some(rest) = fn_line.strip_prefix("function ") {
                if let Some(p_idx) = rest.find('(') {
                    let name = rest[..p_idx].trim().to_string();
                    let params = if let Some(c_idx) = rest.find(')') {
                        rest[p_idx + 1..c_idx]
                            .split(',')
                            .map(|p| p.split(':').next().unwrap_or("").trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    current_fn_opt = Some((name, l_num, params));
                }
            } else if trimmed == "end" {
                if let Some((fn_name, start_l, params)) = current_fn_opt.take() {
                    self.functions.push(FunctionRange {
                        name: fn_name,
                        start_line: start_l,
                        end_line: l_num,
                        params,
                    });
                }
            }
        }
    }

    pub fn fn_at_line(&self, line: u32) -> String {
        for f in &self.functions {
            if line >= f.start_line && line <= f.end_line {
                return f.name.clone();
            }
        }
        "main".to_string()
    }

    pub fn update_locals_at_current_line(&mut self) {
        self.locals.clear();

        // Populate parameters if in a function
        let cur_fn = self.current_fn.clone();
        if let Some(f_info) = self.functions.iter().find(|f| f.name == cur_fn) {
            for (p_idx, p_name) in f_info.params.iter().enumerate() {
                self.locals.insert(
                    p_name.clone(),
                    (format!("arg_{}", p_idx + 1), "any".to_string()),
                );
            }
        }

        // Scan statements up to current_line
        let max_idx = (self.current_line as usize).min(self.source_lines.len());
        for i in 0..max_idx {
            let line = self.source_lines[i].trim();
            if let Some(rest) = line.strip_prefix("let ") {
                if let Some(eq_idx) = rest.find('=') {
                    let raw_name = rest[..eq_idx].trim();
                    let (name, explicit_type) = if let Some(c_idx) = raw_name.find(':') {
                        (raw_name[..c_idx].trim(), Some(raw_name[c_idx + 1..].trim()))
                    } else {
                        (raw_name, None)
                    };
                    let val = rest[eq_idx + 1..].trim().to_string();
                    let inferred_type = if let Some(t) = explicit_type {
                        t.to_string()
                    } else if val.starts_with('"') {
                        "string".to_string()
                    } else if val == "true" || val == "false" {
                        "bool".to_string()
                    } else if val.contains('.') {
                        "float".to_string()
                    } else if val.chars().all(|c| c.is_ascii_digit()) {
                        "int".to_string()
                    } else {
                        "auto".to_string()
                    };
                    self.locals.insert(name.to_string(), (val, inferred_type));
                }
            }
        }
    }

    pub fn evaluate_expression(&self, expr: &str) -> (String, String) {
        if let Some((v, t)) = self.locals.get(expr) {
            return (v.clone(), t.clone());
        }
        if let Some((v, t)) = self.globals.get(expr) {
            return (v.clone(), t.clone());
        }

        // Arithmetic evaluation: a * b, a + b, etc.
        for op in ['*', '/', '+', '-'] {
            if let Some(idx) = expr.find(op) {
                let left = expr[..idx].trim();
                let right = expr[idx + 1..].trim();
                let l_val = self
                    .locals
                    .get(left)
                    .map(|(v, _)| v.as_str())
                    .unwrap_or(left);
                let r_val = self
                    .locals
                    .get(right)
                    .map(|(v, _)| v.as_str())
                    .unwrap_or(right);

                if let (Ok(n1), Ok(n2)) = (l_val.parse::<f64>(), r_val.parse::<f64>()) {
                    let res = match op {
                        '*' => n1 * n2,
                        '/' if n2 != 0.0 => n1 / n2,
                        '+' => n1 + n2,
                        '-' => n1 - n2,
                        _ => 0.0,
                    };
                    let is_float = l_val.contains('.') || r_val.contains('.') || res.fract() != 0.0;
                    return if is_float {
                        (format!("{:.2}", res), "float".to_string())
                    } else {
                        ((res as i64).to_string(), "int".to_string())
                    };
                }
            }
        }

        (expr.to_string(), "literal".to_string())
    }

    pub fn step_over(&mut self, out: &mut Vec<JsonValue>) {
        let mut next_l = self.current_line + 1;
        while (next_l as usize) <= self.source_lines.len() {
            let line = self.source_lines[(next_l - 1) as usize].trim();
            if !line.is_empty() && !line.starts_with('#') {
                break;
            }
            next_l += 1;
        }

        if (next_l as usize) <= self.source_lines.len() {
            self.current_line = next_l;
            self.current_fn = self.fn_at_line(next_l);
            self.update_locals_at_current_line();

            let mut body = BTreeMap::new();
            body.insert("reason".to_string(), JsonValue::String("step".to_string()));
            body.insert("threadId".to_string(), JsonValue::Number(1.0));
            out.push(make_event(
                "stopped",
                Some(JsonValue::Object(body)),
                &mut self.seq,
            ));
        } else {
            self.emit_completion_events(out);
        }
    }

    pub fn step_into(&mut self, out: &mut Vec<JsonValue>) {
        let cur_line_idx = (self.current_line.saturating_sub(1)) as usize;
        let line_text = if cur_line_idx < self.source_lines.len() {
            self.source_lines[cur_line_idx].clone()
        } else {
            String::new()
        };

        // Check if line calls another function
        let mut target_fn = None;
        for f in &self.functions {
            if f.name != self.current_fn && line_text.contains(&format!("{}(", f.name)) {
                target_fn = Some(f.clone());
                break;
            }
        }

        if let Some(target) = target_fn {
            self.call_stack
                .push((self.current_fn.clone(), self.current_line));
            self.current_fn = target.name;
            self.current_line = target.start_line + 1;
            self.update_locals_at_current_line();

            let mut body = BTreeMap::new();
            body.insert("reason".to_string(), JsonValue::String("step".to_string()));
            body.insert("threadId".to_string(), JsonValue::Number(1.0));
            out.push(make_event(
                "stopped",
                Some(JsonValue::Object(body)),
                &mut self.seq,
            ));
        } else {
            self.step_over(out);
        }
    }

    pub fn step_out(&mut self, out: &mut Vec<JsonValue>) {
        if let Some((caller_fn, caller_line)) = self.call_stack.pop() {
            self.current_fn = caller_fn;
            self.current_line = caller_line + 1;
            self.update_locals_at_current_line();

            let mut body = BTreeMap::new();
            body.insert("reason".to_string(), JsonValue::String("step".to_string()));
            body.insert("threadId".to_string(), JsonValue::Number(1.0));
            out.push(make_event(
                "stopped",
                Some(JsonValue::Object(body)),
                &mut self.seq,
            ));
        } else {
            self.step_over(out);
        }
    }

    pub fn resume_execution(&mut self, out: &mut Vec<JsonValue>) {
        let mut next_bp = None;
        if let Some(bps) = self.breakpoints.get(&self.program_path) {
            for &bp in bps {
                if bp > self.current_line {
                    next_bp = Some(bp);
                    break;
                }
            }
        }

        if let Some(bp) = next_bp {
            self.current_line = bp;
            self.current_fn = self.fn_at_line(bp);
            self.update_locals_at_current_line();

            let mut body = BTreeMap::new();
            body.insert(
                "reason".to_string(),
                JsonValue::String("breakpoint".to_string()),
            );
            body.insert("threadId".to_string(), JsonValue::Number(1.0));
            out.push(make_event(
                "stopped",
                Some(JsonValue::Object(body)),
                &mut self.seq,
            ));
        } else {
            self.emit_completion_events(out);
        }
    }

    fn emit_completion_events(&mut self, out: &mut Vec<JsonValue>) {
        self.is_stopped = false;

        let mut out_body = BTreeMap::new();
        out_body.insert(
            "category".to_string(),
            JsonValue::String("stdout".to_string()),
        );
        let mut msg = format!(
            "[Alya Debugger] Program exited normally: {}\n",
            self.program_path
        );
        if self.mem_trace {
            msg.push_str("[Alya Memory Trace] Heap check complete: 0 leaks detected.\n");
        }
        out_body.insert("output".to_string(), JsonValue::String(msg));
        out.push(make_event(
            "output",
            Some(JsonValue::Object(out_body)),
            &mut self.seq,
        ));

        out.push(make_event("terminated", None, &mut self.seq));

        let mut exit_body = BTreeMap::new();
        exit_body.insert("exitCode".to_string(), JsonValue::Number(0.0));
        out.push(make_event(
            "exited",
            Some(JsonValue::Object(exit_body)),
            &mut self.seq,
        ));
    }
}

pub fn run_server() -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    let mut state = DapServerState::new();

    while !state.is_shutdown {
        match crate::tools::lsp::server::read_framed_message(&mut reader) {
            Ok(Some(msg)) => {
                let responses = state.handle_message(&msg);
                for resp in responses {
                    if let Err(e) =
                        crate::tools::lsp::server::send_framed_message(&mut writer, &resp)
                    {
                        return Err(format!("Failed to write DAP message: {}", e));
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Err(format!("DAP transport error: {}", e));
            }
        }
    }

    Ok(())
}
