use crate::ast::expr::Expr;
use crate::ast::stmt::Stmt;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct DocParam {
    pub name: String,
    pub type_ann: Option<String>,
    pub default_val: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocFunction {
    pub name: String,
    pub is_pub: bool,
    pub type_params: Vec<String>,
    pub params: Vec<DocParam>,
    pub return_type: Option<String>,
    pub doc: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocField {
    pub name: String,
    pub type_ann: Option<String>,
    pub default_val: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocStruct {
    pub name: String,
    pub is_pub: bool,
    pub fields: Vec<DocField>,
    pub methods: Vec<DocFunction>,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocMethod {
    pub name: String,
    pub params: Vec<DocParam>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocInterface {
    pub name: String,
    pub is_pub: bool,
    pub embedded: Vec<String>,
    pub methods: Vec<DocMethod>,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocVariant {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocEnum {
    pub name: String,
    pub is_pub: bool,
    pub variants: Vec<DocVariant>,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocConstant {
    pub name: String,
    pub is_pub: bool,
    pub value: String,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocModule {
    pub name: String,
    pub file_path: String,
    pub description: String,
    pub functions: Vec<DocFunction>,
    pub structs: Vec<DocStruct>,
    pub interfaces: Vec<DocInterface>,
    pub enums: Vec<DocEnum>,
    pub constants: Vec<DocConstant>,
}

impl DocModule {
    pub fn new(name: &str, file_path: &str) -> Self {
        Self {
            name: name.to_string(),
            file_path: file_path.to_string(),
            description: String::new(),
            functions: Vec::new(),
            structs: Vec::new(),
            interfaces: Vec::new(),
            enums: Vec::new(),
            constants: Vec::new(),
        }
    }
}

pub fn extract_module_docs(source: &str, file_path: &str) -> DocModule {
    let module_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");

    let mut module = DocModule::new(module_name, file_path);

    // 1. Scan doc comments line-by-line
    let (module_doc, symbol_docs) = parse_doc_comments(source);
    module.description = module_doc;

    // 2. Parse AST
    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => return module, // fallback if syntax error
    };

    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(a) => a,
        Err(_) => return module,
    };

    let mut standalone_functions = Vec::new();
    let mut struct_methods: HashMap<String, Vec<DocFunction>> = HashMap::new();

    for stmt in &ast.statements {
        let is_pub = stmt.is_pub();
        match stmt.inner_stmt() {
            Stmt::Function {
                name,
                params,
                param_types,
                return_type,
                defaults,
                type_params,
                ..
            } => {
                if is_internal_name(name) {
                    continue;
                }
                let doc = symbol_docs
                    .get(name)
                    .or_else(|| symbol_docs.get(&name.replace("__", ".")))
                    .cloned()
                    .unwrap_or_default();
                let mut doc_params = Vec::new();
                for (i, p) in params.iter().enumerate() {
                    let ty = param_types.get(i).and_then(|t| t.clone());
                    let def = defaults.get(i).and_then(|d| d.as_ref()).map(expr_to_string);
                    doc_params.push(DocParam {
                        name: p.clone(),
                        type_ann: ty,
                        default_val: def,
                    });
                }

                let display_name = name.replace("__", ".");
                let sig = format_function_signature(
                    is_pub,
                    &display_name,
                    type_params,
                    &doc_params,
                    return_type.as_deref(),
                );

                let doc_fn = DocFunction {
                    name: display_name,
                    is_pub,
                    type_params: type_params.clone(),
                    params: doc_params,
                    return_type: return_type.clone(),
                    doc,
                    signature: sig,
                };

                // Check if this is a method: e.g. "Point.distance" or "Point__distance"
                if let Some((st_name, m_name)) =
                    name.split_once('.').or_else(|| name.split_once("__"))
                {
                    let mut m_fn = doc_fn.clone();
                    m_fn.name = m_name.to_string();
                    struct_methods
                        .entry(st_name.to_string())
                        .or_default()
                        .push(m_fn);
                } else {
                    standalone_functions.push(doc_fn);
                }
            }
            Stmt::StructDef {
                name,
                fields,
                field_types,
                defaults,
                ..
            } => {
                if is_internal_name(name) {
                    continue;
                }
                let doc = symbol_docs.get(name).cloned().unwrap_or_default();
                let mut doc_fields = Vec::new();
                for (i, f) in fields.iter().enumerate() {
                    let ty = field_types.get(i).and_then(|t| t.clone());
                    let def = defaults.get(i).and_then(|d| d.as_ref()).map(expr_to_string);
                    doc_fields.push(DocField {
                        name: f.clone(),
                        type_ann: ty,
                        default_val: def,
                    });
                }

                module.structs.push(DocStruct {
                    name: name.clone(),
                    is_pub,
                    fields: doc_fields,
                    methods: Vec::new(),
                    doc,
                });
            }
            Stmt::InterfaceDef {
                name,
                methods,
                embedded,
            } => {
                if is_internal_name(name) {
                    continue;
                }
                let doc = symbol_docs.get(name).cloned().unwrap_or_default();
                let mut doc_methods = Vec::new();
                for m in methods {
                    let mut doc_params = Vec::new();
                    for (i, p) in m.params.iter().enumerate() {
                        let ty = m.param_types.get(i).and_then(|t| t.clone());
                        doc_params.push(DocParam {
                            name: p.clone(),
                            type_ann: ty,
                            default_val: None,
                        });
                    }
                    doc_methods.push(DocMethod {
                        name: m.name.clone(),
                        params: doc_params,
                        return_type: m.return_type.clone(),
                    });
                }

                module.interfaces.push(DocInterface {
                    name: name.clone(),
                    is_pub,
                    embedded: embedded.clone(),
                    methods: doc_methods,
                    doc,
                });
            }
            Stmt::EnumDef { name, variants } => {
                if is_internal_name(name) {
                    continue;
                }
                let doc = symbol_docs.get(name).cloned().unwrap_or_default();
                let doc_variants = variants
                    .iter()
                    .map(|(v_name, v_val)| DocVariant {
                        name: v_name.clone(),
                        value: v_val.as_ref().map(expr_to_string),
                    })
                    .collect();

                module.enums.push(DocEnum {
                    name: name.clone(),
                    is_pub,
                    variants: doc_variants,
                    doc,
                });
            }
            Stmt::Const { name, value } => {
                if is_internal_name(name) {
                    continue;
                }
                let doc = symbol_docs.get(name).cloned().unwrap_or_default();
                module.constants.push(DocConstant {
                    name: name.clone(),
                    is_pub,
                    value: expr_to_string(value),
                    doc,
                });
            }
            _ => {}
        }
    }

    // Attach struct methods
    for st in &mut module.structs {
        if let Some(methods) = struct_methods.remove(&st.name) {
            st.methods = methods;
        }
    }

    // Any remaining methods whose structs weren't defined in the same file stay
    // in standalone functions, keeping their qualified `Struct.method` display
    // name so anchors stay unique (e.g. `sync__spawn` renders as `sync.spawn`,
    // never colliding with a bare `spawn` on the same page).
    for (st_name, methods) in struct_methods {
        standalone_functions.extend(methods.into_iter().map(|mut m| {
            m.name = format!("{}.{}", st_name, m.name);
            m
        }));
    }

    module.functions = standalone_functions;

    module
}

fn parse_doc_comments(source: &str) -> (String, HashMap<String, String>) {
    let mut module_doc = Vec::new();
    let mut symbol_docs = HashMap::new();
    let mut current_doc = Vec::new();
    let mut seen_first_decl = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Section banners (`# --- ... ---`) are file layout, not documentation:
        // skip them transparently so they never leak into module or symbol docs.
        if is_section_banner(trimmed) {
            continue;
        }

        if trimmed.starts_with("##") {
            let doc_line = if let Some(rest) = trimmed.strip_prefix("## ") {
                rest
            } else {
                trimmed.strip_prefix("##").unwrap_or("")
            };
            current_doc.push(doc_line.to_string());
            continue;
        }

        if trimmed.starts_with('#') && !trimmed.starts_with("#!") {
            let doc_line = if let Some(rest) = trimmed.strip_prefix("# ") {
                rest
            } else {
                trimmed.strip_prefix('#').unwrap_or("")
            };
            current_doc.push(doc_line.to_string());
            continue;
        }

        if trimmed.is_empty() {
            if !seen_first_decl && !current_doc.is_empty() {
                // If comments are before any declaration and followed by blank lines,
                // consider them module documentation
                module_doc.append(&mut current_doc);
            }
            continue;
        }

        // Line is code / declaration
        if !seen_first_decl {
            seen_first_decl = true;
        }

        if let Some(symbol_name) = extract_decl_name(trimmed) {
            if !current_doc.is_empty() {
                let doc_text = current_doc.join("\n");
                symbol_docs.insert(symbol_name, doc_text);
                current_doc.clear();
            }
        } else {
            current_doc.clear();
        }
    }

    (module_doc.join("\n"), symbol_docs)
}

fn extract_decl_name(line: &str) -> Option<String> {
    let clean = if let Some(rest) = line.strip_prefix("pub ") {
        rest.trim_start()
    } else {
        line
    };

    let prefixes = ["function ", "struct ", "interface ", "enum ", "const "];
    for p in prefixes {
        if let Some(stripped) = clean.strip_prefix(p) {
            let rest = stripped.trim_start();
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}

fn format_function_signature(
    is_pub: bool,
    name: &str,
    type_params: &[String],
    params: &[DocParam],
    ret_type: Option<&str>,
) -> String {
    let mut sig = String::new();
    if is_pub {
        sig.push_str("pub ");
    }
    sig.push_str("function ");
    sig.push_str(name);
    if !type_params.is_empty() {
        sig.push('[');
        sig.push_str(&type_params.join(", "));
        sig.push(']');
    }
    sig.push('(');
    let param_strs: Vec<String> = params
        .iter()
        .map(|p| {
            let mut s = p.name.clone();
            if let Some(ref ty) = p.type_ann {
                s.push_str(": ");
                s.push_str(ty);
            }
            if let Some(ref def) = p.default_val {
                s.push_str(" = ");
                s.push_str(def);
            }
            s
        })
        .collect();
    sig.push_str(&param_strs.join(", "));
    sig.push(')');
    if let Some(ret) = ret_type {
        sig.push_str(" -> ");
        sig.push_str(ret);
    }
    sig
}

pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Expr::Float(f) => format!("{}", f),
        Expr::String(s) => format!("\"{}\"", s),
        Expr::Identifier(id) => id.clone(),
        Expr::Null => "null".to_string(),
        Expr::Array(items) => {
            let inner: Vec<String> = items.iter().map(expr_to_string).collect();
            format!("[{}]", inner.join(", "))
        }
        _ => "...".to_string(),
    }
}

/// Returns true for internal items (leading-underscore names, method-aware).
/// `alya doc` renders the public API reference, so internal helpers such as
/// `_cli_strip_dashes` or `Stack._helper` are skipped in module output.
fn is_internal_name(name: &str) -> bool {
    let short = name.rsplit('.').next().unwrap_or(name);
    // Cut at the FIRST double underscore: the parser mangles `A._b` to
    // `A___b` (separator plus the underscore), so only the first cut
    // preserves a leading underscore on the short name.
    let short = match short.find("__") {
        Some(idx) => &short[idx + 2..],
        None => short,
    };
    short.starts_with('_')
}

/// Returns true for section-banner comments (`# --- ... ---`), which are
/// file layout rather than documentation. The extractor skips them
/// transparently so they never leak into module or symbol docs.
fn is_section_banner(trimmed: &str) -> bool {
    let rest = match trimmed.strip_prefix('#') {
        Some(r) if !r.starts_with('#') => r.trim_start(),
        _ => return false,
    };
    rest.starts_with("---")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banner_above_doc_block_does_not_leak_into_symbol_doc() {
        let source = "# --- Raw Argument Accessors ---\n\n## Returns the raw argument.\n##\n## ### Parameters\npub function cli_arg_at(idx: int) -> string\n    return \"x\"\nend\n";
        let module = extract_module_docs(source, "cli.alya");
        assert_eq!(module.functions.len(), 1);
        assert!(
            !module.functions[0].doc.contains("Raw Argument Accessors"),
            "banner leaked: {}",
            module.functions[0].doc
        );
        assert!(module.functions[0]
            .doc
            .contains("Returns the raw argument."));
    }

    #[test]
    fn test_banner_after_header_does_not_leak_into_module_doc() {
        let source = "# Module header.\n# Second header line.\n\n# --- Private Helpers ---\n\n## Helper doc.\nfunction helper()\nend\n";
        let module = extract_module_docs(source, "m.alya");
        assert!(
            !module.description.contains("Private Helpers"),
            "banner leaked: {}",
            module.description
        );
        assert!(module.description.contains("Module header."));
    }

    #[test]
    fn test_banner_between_doc_and_decl_preserves_doc() {
        let source = "## Real doc line.\n# --- Next Section ---\npub function thing() -> int\n    return 1\nend\n";
        let module = extract_module_docs(source, "m.alya");
        assert_eq!(module.functions.len(), 1);
        assert!(module.functions[0].doc.contains("Real doc line."));
        assert!(!module.functions[0].doc.contains("Next Section"));
    }

    #[test]
    fn test_plain_hash_comments_still_collected() {
        // Non-banner single-# lines keep their long-standing behavior.
        let source =
            "# Behavior note.\n## Summary line.\npub function f() -> int\n    return 1\nend\n";
        let module = extract_module_docs(source, "m.alya");
        assert!(module.functions[0].doc.contains("Behavior note."));
        assert!(module.functions[0].doc.contains("Summary line."));
    }

    #[test]
    fn test_underscore_prefixed_items_hidden_from_docs() {
        let source = "## Private helper.\nfunction _helper() -> int\n    return 1\nend\n\n## Public API.\npub function api() -> int\n    return 2\nend\n\n## Method user.\nfunction Widget._internal() -> int\n    return 3\nend\n\n## Widget type.\npub struct Widget\n    x: int\nend\n";
        let module = extract_module_docs(source, "m.alya");
        assert!(
            module.functions.iter().all(|f| !f.name.starts_with('_')),
            "private helper leaked: {:?}",
            module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert!(module.functions.iter().any(|f| f.name == "api"));
        let st = module.structs.iter().find(|s| s.name == "Widget").unwrap();
        assert!(
            st.methods.iter().all(|m| !m.name.starts_with('_')),
            "private method leaked"
        );
    }
}
