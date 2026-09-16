use crate::ast::Stmt;
use crate::codegen::{Architecture, OperatingSystem};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;

use super::analyzer::{expr_contains_ask, stmt_contains_ask};
use super::executor::{execute_code_snippet, execute_code_snippet_interactive};

/// Persistent state of an interactive REPL session.
pub struct ReplSession {
    pub imports: Vec<String>,
    pub structs: Vec<(String, String)>,
    pub functions: Vec<(String, String)>,
    pub statements: Vec<String>,
    pub var_names: Vec<String>,
    pub arch: Architecture,
    pub os: OperatingSystem,
}

impl ReplSession {
    pub fn new(arch: Architecture, os: OperatingSystem) -> Self {
        Self {
            imports: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            statements: Vec::new(),
            var_names: Vec::new(),
            arch,
            os,
        }
    }

    /// Assembles all persistent definitions and statements into a single source string,
    /// optionally appending a trailing snippet to be evaluated.
    pub fn assemble_program(&self, trailing_code: &str) -> String {
        let mut code = String::new();

        for imp in &self.imports {
            code.push_str(imp);
            code.push('\n');
        }

        for (_, s_code) in &self.structs {
            code.push_str(s_code);
            code.push('\n');
        }

        for (_, f_code) in &self.functions {
            code.push_str(f_code);
            code.push('\n');
        }

        for stmt in &self.statements {
            code.push_str(stmt);
            code.push('\n');
        }

        if !trailing_code.is_empty() {
            code.push_str(trailing_code);
            code.push('\n');
        }

        code
    }

    /// Clears all session definitions and history.
    pub fn clear(&mut self) {
        self.imports.clear();
        self.structs.clear();
        self.functions.clear();
        self.statements.clear();
        self.var_names.clear();
    }

    /// Updates an existing variable statement if it exists in session statements,
    /// or appends it to the statements list.
    pub fn update_or_add_statement(&mut self, var_name: &str, new_stmt: &str) {
        let prefix = format!("let {}", var_name);
        if let Some(pos) = self.statements.iter().position(|s| {
            if let Some(rest) = s.strip_prefix(&prefix) {
                let rest_trim = rest.trim_start();
                rest_trim.starts_with('=')
            } else {
                false
            }
        }) {
            self.statements[pos] = new_stmt.to_string();
        } else {
            self.statements.push(new_stmt.to_string());
        }
    }

    /// Compiles and executes the current session combined with the provided snippet.
    pub fn execute_snippet(&self, snippet: &str) -> Result<(bool, String, String), String> {
        let source = self.assemble_program(snippet);
        execute_code_snippet(&source, self.arch, self.os)
    }

    /// Compiles and executes the current session combined with the provided snippet interactively,
    /// inheriting stdin, stdout, and stderr.
    pub fn execute_snippet_interactive(&self, snippet: &str) -> Result<bool, String> {
        let source = self.assemble_program(snippet);
        execute_code_snippet_interactive(&source, self.arch, self.os)
    }

    /// Processes a complete user input chunk.
    pub fn eval_input(&mut self, input: &str) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return;
        }

        // Handle REPL commands
        if trimmed.starts_with(':') {
            self.handle_command(trimmed);
            return;
        }

        // 1. Tokenize input chunk
        let mut lexer = Lexer::new(trimmed);
        let tokens = match lexer.tokenize() {
            Ok(toks) => toks,
            Err(err) => {
                eprintln!("\x1b[1;31mSyntax Error:\x1b[0m {}", err);
                return;
            }
        };

        // 2. Parse input chunk
        let mut parser = Parser::new(tokens);
        let parsed_program = match parser.parse() {
            Ok(prog) => prog,
            Err(err) => {
                eprintln!("\x1b[1;31mParse Error:\x1b[0m {}", err);
                return;
            }
        };

        // 3. Process statements
        if parsed_program.statements.is_empty() {
            return;
        }

        // Check if single statement is a bare expression
        if parsed_program.statements.len() == 1 {
            match &parsed_program.statements[0] {
                Stmt::Expr(expr) => {
                    if expr_contains_ask(expr, &self.functions) {
                        let pid = std::process::id();
                        let rand_id = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos()
                            % 1_000_000_000;
                        let temp_capture_file = format!("temp_repl_val_{}_{}.tmp", pid, rand_id);
                        let capture_stmt = format!(
                            "let _repl_ans = {}\nwrite_file(\"{}\", str(_repl_ans))",
                            trimmed, temp_capture_file
                        );
                        let full_source = self.assemble_program(&capture_stmt);
                        match execute_code_snippet_interactive(&full_source, self.arch, self.os) {
                            Ok(true) => {
                                let captured_val =
                                    fs::read_to_string(&temp_capture_file).unwrap_or_default();
                                let _ = fs::remove_file(&temp_capture_file);
                                let out = captured_val.trim();
                                if !out.is_empty() {
                                    println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", out);
                                }
                            }
                            Ok(false) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mRuntime Error\x1b[0m");
                            }
                            Err(err) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                            }
                        }
                        return;
                    }

                    // Evaluate as an expression wrapped in say (...)
                    let eval_code = format!("say ({})", trimmed);
                    match self.execute_snippet(&eval_code) {
                        Ok((true, stdout, stderr)) => {
                            let out = stdout.trim_end();
                            if !out.is_empty() {
                                println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", out);
                            }
                            if !stderr.trim().is_empty() {
                                eprintln!("\x1b[1;33m{}\x1b[0m", stderr.trim_end());
                            }
                        }
                        Ok((false, stdout, stderr)) => {
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mRuntime Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                Stmt::Import { path, .. } => {
                    let imp_str = trimmed.to_string();
                    if !self.imports.contains(&imp_str) {
                        self.imports.push(imp_str.clone());
                    }
                    match self.execute_snippet("") {
                        Ok((true, _, _)) => {
                            println!("\x1b[1;36m=>\x1b[0m module '{}' loaded", path);
                        }
                        Ok((false, stdout, stderr)) => {
                            self.imports.retain(|s| s != &imp_str);
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mImport Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            self.imports.retain(|s| s != &imp_str);
                            eprintln!("\x1b[1;31mImport Error:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                Stmt::Function { name, .. } => {
                    let func_name = name.clone();
                    let func_code = trimmed.to_string();

                    // Temporarily update function definition
                    let mut prev = None;
                    if let Some(pos) = self.functions.iter().position(|(n, _)| n == &func_name) {
                        prev = Some((pos, self.functions.remove(pos)));
                    }
                    self.functions.push((func_name.clone(), func_code));

                    match self.execute_snippet("") {
                        Ok((true, _, _)) => {
                            println!("\x1b[1;36m=>\x1b[0m function {} defined", func_name);
                        }
                        Ok((false, stdout, stderr)) => {
                            self.functions.pop();
                            if let Some((pos, entry)) = prev {
                                self.functions.insert(pos, entry);
                            }
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mFunction Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            self.functions.pop();
                            if let Some((pos, entry)) = prev {
                                self.functions.insert(pos, entry);
                            }
                            eprintln!("\x1b[1;31mFunction Error:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                Stmt::StructDef { name, .. } => {
                    let struct_name = name.clone();
                    let struct_code = trimmed.to_string();

                    let mut prev = None;
                    if let Some(pos) = self.structs.iter().position(|(n, _)| n == &struct_name) {
                        prev = Some((pos, self.structs.remove(pos)));
                    }
                    self.structs.push((struct_name.clone(), struct_code));

                    match self.execute_snippet("") {
                        Ok((true, _, _)) => {
                            println!("\x1b[1;36m=>\x1b[0m struct {} defined", struct_name);
                        }
                        Ok((false, stdout, stderr)) => {
                            self.structs.pop();
                            if let Some((pos, entry)) = prev {
                                self.structs.insert(pos, entry);
                            }
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mStruct Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            self.structs.pop();
                            if let Some((pos, entry)) = prev {
                                self.structs.insert(pos, entry);
                            }
                            eprintln!("\x1b[1;31mStruct Error:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                Stmt::Let { name, value, .. } => {
                    let var_name = name.clone();
                    if expr_contains_ask(value, &self.functions) {
                        let pid = std::process::id();
                        let rand_id = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos()
                            % 1_000_000_000;
                        let temp_capture_file = format!("temp_repl_val_{}_{}.tmp", pid, rand_id);
                        let is_str = crate::codegen::analysis::is_string_expr(
                            value,
                            &std::collections::HashMap::new(),
                        );
                        let write_call = if is_str {
                            format!("write_file(\"{}\", {})", temp_capture_file, var_name)
                        } else {
                            format!("write_file(\"{}\", str({}))", temp_capture_file, var_name)
                        };
                        let capture_stmt = format!("{}\n{}", trimmed, write_call);
                        let full_source = self.assemble_program(&capture_stmt);

                        match execute_code_snippet_interactive(&full_source, self.arch, self.os) {
                            Ok(true) => {
                                let captured_val =
                                    fs::read_to_string(&temp_capture_file).unwrap_or_default();
                                let _ = fs::remove_file(&temp_capture_file);

                                let is_flt = crate::codegen::analysis::is_float_expr(
                                    value,
                                    &std::collections::HashMap::new(),
                                );

                                let literal_repr = if is_str {
                                    let escaped = captured_val
                                        .replace('\\', "\\\\")
                                        .replace('"', "\\\"")
                                        .replace('\n', "\\n")
                                        .replace('\r', "\\r");
                                    format!("\"{}\"", escaped)
                                } else if is_flt {
                                    let t = captured_val.trim();
                                    if t.contains('.') {
                                        t.to_string()
                                    } else {
                                        format!("{}.0", t)
                                    }
                                } else {
                                    let t = captured_val.trim();
                                    if t.is_empty() {
                                        "\"\"".to_string()
                                    } else {
                                        t.to_string()
                                    }
                                };

                                let frozen_stmt = format!("let {} = {}", var_name, literal_repr);
                                self.update_or_add_statement(&var_name, &frozen_stmt);
                                if !self.var_names.contains(&var_name) {
                                    self.var_names.push(var_name);
                                }

                                println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", literal_repr);
                            }
                            Ok(false) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mRuntime Error\x1b[0m");
                            }
                            Err(err) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                            }
                        }
                        return;
                    }

                    // Execute let statement and inspect its value
                    let eval_code = format!("{}\nsay ({})", trimmed, var_name);
                    match self.execute_snippet(&eval_code) {
                        Ok((true, stdout, stderr)) => {
                            self.update_or_add_statement(&var_name, trimmed);
                            if !self.var_names.contains(&var_name) {
                                self.var_names.push(var_name);
                            }
                            let out = stdout.trim_end();
                            if !out.is_empty() {
                                println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", out);
                            }
                            if !stderr.trim().is_empty() {
                                eprintln!("\x1b[1;33m{}\x1b[0m", stderr.trim_end());
                            }
                        }
                        Ok((false, stdout, stderr)) => {
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mRuntime Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                Stmt::Assign { name, value } => {
                    let var_name = name.clone();
                    if expr_contains_ask(value, &self.functions) {
                        let pid = std::process::id();
                        let rand_id = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos()
                            % 1_000_000_000;
                        let temp_capture_file = format!("temp_repl_val_{}_{}.tmp", pid, rand_id);
                        let is_str = crate::codegen::analysis::is_string_expr(
                            value,
                            &std::collections::HashMap::new(),
                        );
                        let write_call = if is_str {
                            format!("write_file(\"{}\", {})", temp_capture_file, var_name)
                        } else {
                            format!("write_file(\"{}\", str({}))", temp_capture_file, var_name)
                        };
                        let capture_stmt = format!("{}\n{}", trimmed, write_call);
                        let full_source = self.assemble_program(&capture_stmt);

                        match execute_code_snippet_interactive(&full_source, self.arch, self.os) {
                            Ok(true) => {
                                let captured_val =
                                    fs::read_to_string(&temp_capture_file).unwrap_or_default();
                                let _ = fs::remove_file(&temp_capture_file);

                                let is_flt = crate::codegen::analysis::is_float_expr(
                                    value,
                                    &std::collections::HashMap::new(),
                                );

                                let literal_repr = if is_str {
                                    let escaped = captured_val
                                        .replace('\\', "\\\\")
                                        .replace('"', "\\\"")
                                        .replace('\n', "\\n")
                                        .replace('\r', "\\r");
                                    format!("\"{}\"", escaped)
                                } else if is_flt {
                                    let t = captured_val.trim();
                                    if t.contains('.') {
                                        t.to_string()
                                    } else {
                                        format!("{}.0", t)
                                    }
                                } else {
                                    let t = captured_val.trim();
                                    if t.is_empty() {
                                        "\"\"".to_string()
                                    } else {
                                        t.to_string()
                                    }
                                };

                                let frozen_stmt = format!("let {} = {}", var_name, literal_repr);
                                self.update_or_add_statement(&var_name, &frozen_stmt);

                                println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", literal_repr);
                            }
                            Ok(false) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mRuntime Error\x1b[0m");
                            }
                            Err(err) => {
                                let _ = fs::remove_file(&temp_capture_file);
                                eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                            }
                        }
                        return;
                    }

                    let eval_code = format!("{}\nsay ({})", trimmed, var_name);
                    match self.execute_snippet(&eval_code) {
                        Ok((true, stdout, stderr)) => {
                            self.statements.push(trimmed.to_string());
                            let out = stdout.trim_end();
                            if !out.is_empty() {
                                println!("\x1b[1;36m=>\x1b[0m \x1b[1;32m{}\x1b[0m", out);
                            }
                            if !stderr.trim().is_empty() {
                                eprintln!("\x1b[1;33m{}\x1b[0m", stderr.trim_end());
                            }
                        }
                        Ok((false, stdout, stderr)) => {
                            let err_msg = if !stderr.trim().is_empty() {
                                stderr.trim()
                            } else {
                                stdout.trim()
                            };
                            eprintln!("\x1b[1;31mRuntime Error:\x1b[0m {}", err_msg);
                        }
                        Err(err) => {
                            eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                        }
                    }
                    return;
                }
                _ => {}
            }
        }

        // For other statements or multi-statement blocks (loops, ifs, says)
        let has_mutations = parsed_program.statements.iter().any(|s| {
            matches!(
                s,
                Stmt::Let { .. }
                    | Stmt::Assign { .. }
                    | Stmt::IndexAssign { .. }
                    | Stmt::FieldAssign { .. }
            )
        });

        let has_ask = parsed_program
            .statements
            .iter()
            .any(|s| stmt_contains_ask(s, &self.functions));

        if has_ask {
            let full_source = self.assemble_program(trimmed);
            match execute_code_snippet_interactive(&full_source, self.arch, self.os) {
                Ok(true) => {
                    if has_mutations {
                        self.statements.push(trimmed.to_string());
                        for s in &parsed_program.statements {
                            if let Stmt::Let { name, .. } = s {
                                if !self.var_names.contains(name) {
                                    self.var_names.push(name.clone());
                                }
                            }
                        }
                    }
                }
                Ok(false) => {
                    eprintln!("\x1b[1;31mRuntime Error\x1b[0m");
                }
                Err(err) => {
                    eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
                }
            }
            return;
        }

        match self.execute_snippet(trimmed) {
            Ok((true, stdout, stderr)) => {
                if has_mutations {
                    self.statements.push(trimmed.to_string());
                    for s in &parsed_program.statements {
                        if let Stmt::Let { name, .. } = s {
                            if !self.var_names.contains(name) {
                                self.var_names.push(name.clone());
                            }
                        }
                    }
                }
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
            }
            Ok((false, stdout, stderr)) => {
                let err_msg = if !stderr.trim().is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                eprintln!("\x1b[1;31mRuntime Error:\x1b[0m {}", err_msg);
            }
            Err(err) => {
                eprintln!("\x1b[1;31mError:\x1b[0m {}", err);
            }
        }
    }

    fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let name = parts[0];

        match name {
            ":exit" | ":quit" | ":q" => {
                println!("\x1b[1;33mGoodbye!\x1b[0m");
                std::process::exit(0);
            }
            ":clear" | ":c" => {
                self.clear();
                println!("\x1b[1;32m✓ Session state cleared.\x1b[0m");
            }
            ":vars" | ":v" => {
                println!("\x1b[1;36m=== Active Session State ===\x1b[0m");
                if self.imports.is_empty()
                    && self.structs.is_empty()
                    && self.functions.is_empty()
                    && self.var_names.is_empty()
                {
                    println!("  (No variables or functions defined yet)");
                    return;
                }

                if !self.imports.is_empty() {
                    println!("\x1b[1;33mImports:\x1b[0m");
                    for imp in &self.imports {
                        println!("  {}", imp);
                    }
                }

                if !self.structs.is_empty() {
                    println!("\x1b[1;33mStructs:\x1b[0m");
                    for (name, _) in &self.structs {
                        println!("  struct {}", name);
                    }
                }

                if !self.functions.is_empty() {
                    println!("\x1b[1;33mFunctions:\x1b[0m");
                    for (name, _) in &self.functions {
                        println!("  function {}", name);
                    }
                }

                if !self.var_names.is_empty() {
                    println!("\x1b[1;33mVariables:\x1b[0m");
                    for name in &self.var_names {
                        println!("  {}", name);
                    }
                }
            }
            ":code" => {
                let code = self.assemble_program("");
                println!("\x1b[1;36m=== Current Session Code ===\x1b[0m");
                if code.trim().is_empty() {
                    println!("  (Empty session)");
                } else {
                    println!("{}", code.trim());
                }
                println!("\x1b[1;36m============================\x1b[0m");
            }
            ":help" | ":h" => {
                const W: usize = 56;
                let border = "═".repeat(W);
                println!("\x1b[1;36m╔{}╗\x1b[0m", border);
                crate::driver::console::print_box_row(
                    "                  \x1b[1;33mALYA REPL COMMANDS\x1b[0m",
                    W,
                );
                println!("\x1b[1;36m╠{}╣\x1b[0m", border);
                crate::driver::console::print_box_row(
                    "  \x1b[1;32m:help, :h\x1b[0m     Show this help guide",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  \x1b[1;32m:vars, :v\x1b[0m     List defined variables and functions",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  \x1b[1;32m:code\x1b[0m         View accumulated session code",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  \x1b[1;32m:clear, :c\x1b[0m    Reset session state",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  \x1b[1;32m:exit, :q\x1b[0m     Exit the REPL",
                    W,
                );
                println!("\x1b[1;36m╠{}╣\x1b[0m", border);
                crate::driver::console::print_box_row("  \x1b[1;33mFeatures:\x1b[0m", W);
                crate::driver::console::print_box_row(
                    "  • Expressions (e.g. 1 + 2, [1, 2, 3]) auto-evaluate",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  • Multi-line blocks continue until 'end' keyword",
                    W,
                );
                crate::driver::console::print_box_row(
                    "  • Standard library support (e.g. import \"std/math\")",
                    W,
                );
                println!("\x1b[1;36m╚{}╝\x1b[0m", border);
            }
            other => {
                eprintln!(
                    "\x1b[1;31mUnknown command '{}'. Type :help for available commands.\x1b[0m",
                    other
                );
            }
        }
    }
}
