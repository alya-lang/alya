use alya::tools::doc::extractor::extract_module_docs;
use alya::tools::doc::html::generate_html;
use alya::tools::doc::markdown::generate_markdown;
use alya::tools::doc::run_doc;
use alya::tools::lsp::json::JsonValue;
use alya::tools::lsp::protocol::Position;
use alya::tools::lsp::server::ServerState;
use alya::tools::pkg::hash::sha256_hex;
use alya::tools::pkg::lock::{format_git_source, parse_lockfile, serialize_lockfile};
use alya::tools::pkg::resolver::{resolve_package_spec, resolve_registry_url};
use alya::tools::pkg::types::{LockedPackage, PackageLock};
use std::collections::BTreeMap;
use std::env;
use std::fs;

// ============================================================================
// 1. LSP PROTOCOL & SERVER TESTS
// ============================================================================

#[test]
fn test_lsp_initialize() {
    let mut server = ServerState::new();

    let mut init_req = BTreeMap::new();
    init_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    init_req.insert("id".to_string(), JsonValue::Number(1.0));
    init_req.insert(
        "method".to_string(),
        JsonValue::String("initialize".to_string()),
    );
    init_req.insert("params".to_string(), JsonValue::Object(BTreeMap::new()));

    let resp = server
        .handle_message(&JsonValue::Object(init_req))
        .expect("Expected response");
    assert_eq!(resp.get("id").and_then(|i| i.as_i64()), Some(1));

    let result = resp.get("result").expect("Expected result");
    let capabilities = result.get("capabilities").expect("Expected capabilities");

    // Full sync = 1
    assert_eq!(
        capabilities
            .get("textDocumentSync")
            .and_then(|s| s.as_i64()),
        Some(1)
    );
    assert_eq!(
        capabilities.get("hoverProvider").and_then(|h| h.as_bool()),
        Some(true)
    );
    assert_eq!(
        capabilities
            .get("definitionProvider")
            .and_then(|d| d.as_bool()),
        Some(true)
    );

    let comp = capabilities
        .get("completionProvider")
        .expect("Expected completionProvider");
    assert_eq!(
        comp.get("resolveProvider").and_then(|r| r.as_bool()),
        Some(false)
    );
}

#[test]
fn test_lsp_diagnostics_did_open_and_did_change() {
    let mut server = ServerState::new();
    let uri = "file:///test.alya";

    // 1. Open invalid document
    let invalid_source = "function broken( { say 1 end";
    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert(
        "text".to_string(),
        JsonValue::String(invalid_source.to_string()),
    );
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));

    let notif = server
        .handle_message(&JsonValue::Object(did_open))
        .expect("Expected notification");
    assert_eq!(
        notif.get("method").and_then(|m| m.as_str()),
        Some("textDocument/publishDiagnostics")
    );

    let diags = notif
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_array())
        .expect("Expected diagnostics array");
    assert!(!diags.is_empty(), "Should report syntax error");

    // 2. Change to valid document
    let valid_source = "function fixed() -> int\n    return 42\nend";
    let mut did_change_params = BTreeMap::new();
    let mut doc_change_info = BTreeMap::new();
    doc_change_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    did_change_params.insert(
        "textDocument".to_string(),
        JsonValue::Object(doc_change_info),
    );

    let mut change_item = BTreeMap::new();
    change_item.insert(
        "text".to_string(),
        JsonValue::String(valid_source.to_string()),
    );
    did_change_params.insert(
        "contentChanges".to_string(),
        JsonValue::Array(vec![JsonValue::Object(change_item)]),
    );

    let mut did_change = BTreeMap::new();
    did_change.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_change.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didChange".to_string()),
    );
    did_change.insert("params".to_string(), JsonValue::Object(did_change_params));

    let fixed_notif = server
        .handle_message(&JsonValue::Object(did_change))
        .expect("Expected notification");
    let fixed_diags = fixed_notif
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_array())
        .expect("Expected diagnostics array");
    assert_eq!(fixed_diags.len(), 0, "Valid code must have 0 diagnostics");
}

#[test]
fn test_lsp_linter_diagnostics_and_code_action() {
    let mut server = ServerState::new();
    let uri = "file:///lint_test.alya";
    let source = "function test()\n    let unused_test_val = 123\n    say 42\nend";

    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert("text".to_string(), JsonValue::String(source.to_string()));
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));

    let notif = server
        .handle_message(&JsonValue::Object(did_open))
        .expect("Expected notif");
    let diags = notif
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_array())
        .unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("code").and_then(|c| c.as_str()),
        Some("unused-var")
    );
    assert_eq!(
        diags[0].get("source").and_then(|s| s.as_str()),
        Some("alya-lint")
    );

    // Request code action
    let mut ca_params = BTreeMap::new();
    let mut doc_ident = BTreeMap::new();
    doc_ident.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    ca_params.insert("textDocument".to_string(), JsonValue::Object(doc_ident));

    let mut code_action_req = BTreeMap::new();
    code_action_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    code_action_req.insert("id".to_string(), JsonValue::Number(1.0));
    code_action_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/codeAction".to_string()),
    );
    code_action_req.insert("params".to_string(), JsonValue::Object(ca_params));

    let resp = server
        .handle_message(&JsonValue::Object(code_action_req))
        .expect("Expected response");
    let actions = resp.get("result").and_then(|r| r.as_array()).unwrap();
    assert_eq!(actions.len(), 1);
    let title = actions[0].get("title").and_then(|t| t.as_str()).unwrap();
    assert!(title.contains("_unused_test_val"));
}

#[test]
fn test_lsp_uri_to_path_decoding() {
    use alya::tools::lsp::protocol::uri_to_path;

    // Windows standard URI
    let p1 = uri_to_path("file:///C:/project/foo.alya");
    assert_eq!(p1, std::path::PathBuf::from("C:/project/foo.alya"));

    // Windows percent-encoded drive & spaces
    let p2 = uri_to_path("file:///c%3A/my%20dir/test.alya");
    assert_eq!(p2, std::path::PathBuf::from("c:/my dir/test.alya"));

    // Linux URI
    let p3 = uri_to_path("file:///home/user/project/test.alya");
    assert_eq!(p3, std::path::PathBuf::from("/home/user/project/test.alya"));
}

#[test]
fn test_lsp_unused_import_relative_resolution() {
    let tmp = std::env::temp_dir().join(format!("alya_lsp_import_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let helper_path = tmp.join("helper.alya");
    std::fs::write(
        &helper_path,
        "pub function do_work() -> int\n    return 42\nend\n",
    )
    .unwrap();

    let caller_path = tmp.join("caller.alya");
    let caller_source =
        "import \"./helper.alya\"\nfunction main()\n    let res = do_work()\n    say res\nend\n";

    let mut server = ServerState::new();
    let uri = format!(
        "file:///{}",
        caller_path.to_string_lossy().replace('\\', "/")
    );

    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.clone()));
    doc_info.insert(
        "text".to_string(),
        JsonValue::String(caller_source.to_string()),
    );
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));

    let notif = server
        .handle_message(&JsonValue::Object(did_open))
        .expect("Expected publishDiagnostics notif");
    let diags = notif
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_array())
        .unwrap();

    // With proper path resolution, import "./helper.alya" must NOT be flagged as unused!
    let unused_imports: Vec<_> = diags
        .iter()
        .filter(|d| d.get("code").and_then(|c| c.as_str()) == Some("unused-import"))
        .collect();
    assert_eq!(unused_imports.len(), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_lsp_completion_and_hover_and_definition() {
    let mut server = ServerState::new();
    let uri = "file:///app.alya";
    let source = r#"
struct Point
    x: int
    y: int
end

interface Shape
    function area(self) -> float
end

pub function calculate_area(p: Point) -> int
    let result = 42
    return result
end
"#;

    // Load document
    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert("text".to_string(), JsonValue::String(source.to_string()));
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));
    server.handle_message(&JsonValue::Object(did_open));

    // 1. Completion Test
    let mut comp_params = BTreeMap::new();
    let mut comp_doc = BTreeMap::new();
    comp_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    comp_params.insert("textDocument".to_string(), JsonValue::Object(comp_doc));
    comp_params.insert("position".to_string(), Position::new(10, 5).to_json());

    let mut comp_req = BTreeMap::new();
    comp_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    comp_req.insert("id".to_string(), JsonValue::Number(2.0));
    comp_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/completion".to_string()),
    );
    comp_req.insert("params".to_string(), JsonValue::Object(comp_params));

    let comp_resp = server
        .handle_message(&JsonValue::Object(comp_req))
        .expect("Expected response");
    let items = comp_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected items array");

    let labels: Vec<&str> = items
        .iter()
        .filter_map(|it| it.get("label").and_then(|l| l.as_str()))
        .collect();
    assert!(
        labels.contains(&"function"),
        "Must contain 'function' keyword"
    );
    assert!(labels.contains(&"spawn"), "Must contain 'spawn' keyword");
    assert!(labels.contains(&"std/sync"), "Must contain std module");
    assert!(labels.contains(&"Point"), "Must contain AST struct Point");
    assert!(
        labels.contains(&"Shape"),
        "Must contain AST interface Shape"
    );
    assert!(
        labels.contains(&"calculate_area"),
        "Must contain AST function calculate_area"
    );
    assert!(labels.contains(&"std/simd"), "Must contain std/simd module");
    assert!(labels.contains(&"f64x4"), "Must contain f64x4 SIMD type");

    // 2. Hover Test
    let mut hover_params = BTreeMap::new();
    let mut hover_doc = BTreeMap::new();
    hover_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    hover_params.insert("textDocument".to_string(), JsonValue::Object(hover_doc));
    hover_params.insert("position".to_string(), Position::new(10, 15).to_json()); // on calculate_area

    let mut hover_req = BTreeMap::new();
    hover_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    hover_req.insert("id".to_string(), JsonValue::Number(3.0));
    hover_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/hover".to_string()),
    );
    hover_req.insert("params".to_string(), JsonValue::Object(hover_params));

    let hover_resp = server
        .handle_message(&JsonValue::Object(hover_req))
        .expect("Expected response");
    let contents = hover_resp
        .get("result")
        .and_then(|r| r.get("contents"))
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .expect("Expected hover markdown");
    assert!(
        contents.contains("calculate_area"),
        "Hover tooltip should contain function signature"
    );

    // 3. Go-to-Definition Test
    let mut def_params = BTreeMap::new();
    let mut def_doc = BTreeMap::new();
    def_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    def_params.insert("textDocument".to_string(), JsonValue::Object(def_doc));
    def_params.insert("position".to_string(), Position::new(10, 15).to_json());

    let mut def_req = BTreeMap::new();
    def_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    def_req.insert("id".to_string(), JsonValue::Number(4.0));
    def_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/definition".to_string()),
    );
    def_req.insert("params".to_string(), JsonValue::Object(def_params));

    let def_resp = server
        .handle_message(&JsonValue::Object(def_req))
        .expect("Expected response");
    let target_uri = def_resp
        .get("result")
        .and_then(|r| r.get("uri"))
        .and_then(|u| u.as_str())
        .expect("Expected uri");
    assert_eq!(target_uri, uri);

    // 4. Shutdown Test
    let mut shutdown_req = BTreeMap::new();
    shutdown_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    shutdown_req.insert("id".to_string(), JsonValue::Number(5.0));
    shutdown_req.insert(
        "method".to_string(),
        JsonValue::String("shutdown".to_string()),
    );

    let shut_resp = server
        .handle_message(&JsonValue::Object(shutdown_req))
        .expect("Expected response");
    assert_eq!(shut_resp.get("result"), Some(&JsonValue::Null));
    assert!(server.is_shutdown);
}

#[test]
fn test_lsp_milestone1_features() {
    let mut server = ServerState::new();
    let uri = "file:///workspace/milestone1.alya";
    let source = r#"# Block comment header
# Line 2 of comment
import "std/math"
import "std/io"

const MAX_LIMIT: int = 500

struct Vector2
    x: float
    y: float
end

enum Direction
    North
    South
end

interface Renderable
    function draw(self)
end

pub function compute_length(v: Vector2) -> float
    if true
        return 1.0
    end
    return 0.0
end
"#;

    // 1. Open document
    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert("text".to_string(), JsonValue::String(source.to_string()));
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));
    server.handle_message(&JsonValue::Object(did_open));

    // 2. Test textDocument/documentSymbol
    let mut sym_params = BTreeMap::new();
    let mut sym_doc = BTreeMap::new();
    sym_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    sym_params.insert("textDocument".to_string(), JsonValue::Object(sym_doc));

    let mut sym_req = BTreeMap::new();
    sym_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    sym_req.insert("id".to_string(), JsonValue::Number(101.0));
    sym_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/documentSymbol".to_string()),
    );
    sym_req.insert("params".to_string(), JsonValue::Object(sym_params));

    let sym_resp = server
        .handle_message(&JsonValue::Object(sym_req))
        .expect("Expected documentSymbol response");
    let symbols = sym_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected symbols array");

    let symbol_names: Vec<&str> = symbols
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        symbol_names.contains(&"MAX_LIMIT"),
        "Must contain constant MAX_LIMIT"
    );
    assert!(
        symbol_names.contains(&"Vector2"),
        "Must contain struct Vector2"
    );
    assert!(
        symbol_names.contains(&"Direction"),
        "Must contain enum Direction"
    );
    assert!(
        symbol_names.contains(&"Renderable"),
        "Must contain interface Renderable"
    );
    assert!(
        symbol_names.contains(&"compute_length"),
        "Must contain function compute_length"
    );

    // Check hierarchical children of Vector2
    let vector2_sym = symbols
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Vector2"))
        .expect("Vector2 symbol");
    let vec_children = vector2_sym
        .get("children")
        .and_then(|c| c.as_array())
        .expect("Vector2 children");
    let vec_child_names: Vec<&str> = vec_children
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        vec_child_names.contains(&"x"),
        "Vector2 must have child field 'x'"
    );
    assert!(
        vec_child_names.contains(&"y"),
        "Vector2 must have child field 'y'"
    );

    // Check hierarchical children of Direction enum
    let dir_sym = symbols
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("Direction"))
        .expect("Direction symbol");
    let dir_children = dir_sym
        .get("children")
        .and_then(|c| c.as_array())
        .expect("Direction children");
    let dir_child_names: Vec<&str> = dir_children
        .iter()
        .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        dir_child_names.contains(&"North"),
        "Direction must have variant North"
    );
    assert!(
        dir_child_names.contains(&"South"),
        "Direction must have variant South"
    );

    // 3. Test textDocument/formatting
    let unformatted_source = "function calc(a: int, b: int) -> int\nreturn a + b\nend\n";
    let fmt_uri = "file:///workspace/fmt_test.alya";
    let mut fmt_open_params = BTreeMap::new();
    let mut fmt_open_doc = BTreeMap::new();
    fmt_open_doc.insert("uri".to_string(), JsonValue::String(fmt_uri.to_string()));
    fmt_open_doc.insert(
        "text".to_string(),
        JsonValue::String(unformatted_source.to_string()),
    );
    fmt_open_params.insert("textDocument".to_string(), JsonValue::Object(fmt_open_doc));

    let mut fmt_open = BTreeMap::new();
    fmt_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    fmt_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    fmt_open.insert("params".to_string(), JsonValue::Object(fmt_open_params));
    server.handle_message(&JsonValue::Object(fmt_open));

    let mut fmt_params = BTreeMap::new();
    let mut fmt_doc = BTreeMap::new();
    fmt_doc.insert("uri".to_string(), JsonValue::String(fmt_uri.to_string()));
    fmt_params.insert("textDocument".to_string(), JsonValue::Object(fmt_doc));

    let mut fmt_req = BTreeMap::new();
    fmt_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    fmt_req.insert("id".to_string(), JsonValue::Number(102.0));
    fmt_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/formatting".to_string()),
    );
    fmt_req.insert("params".to_string(), JsonValue::Object(fmt_params));

    let fmt_resp = server
        .handle_message(&JsonValue::Object(fmt_req))
        .expect("Expected formatting response");
    let edits = fmt_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected edits array");
    assert_eq!(edits.len(), 1, "Should return 1 whole-document text edit");
    let new_text = edits[0]
        .get("newText")
        .and_then(|t| t.as_str())
        .expect("newText");
    assert_eq!(
        new_text,
        "function calc(a: int, b: int) -> int\n    return a + b\nend\n"
    );

    // 4. Test textDocument/references
    // Find references for 'Vector2' (line 21: 'pub function compute_length(v: Vector2) -> float')
    let mut ref_params = BTreeMap::new();
    let mut ref_doc = BTreeMap::new();
    ref_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    ref_params.insert("textDocument".to_string(), JsonValue::Object(ref_doc));
    // Line 21 is index 21 in source: "pub function compute_length(v: Vector2) -> float"
    ref_params.insert("position".to_string(), Position::new(21, 33).to_json());

    let mut ref_req = BTreeMap::new();
    ref_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    ref_req.insert("id".to_string(), JsonValue::Number(103.0));
    ref_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/references".to_string()),
    );
    ref_req.insert("params".to_string(), JsonValue::Object(ref_params));

    let ref_resp = server
        .handle_message(&JsonValue::Object(ref_req))
        .expect("Expected references response");
    let refs = ref_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected locations array");
    // Vector2 appears at declaration ("struct Vector2") and parameter usage ("v: Vector2")
    assert_eq!(refs.len(), 2, "Expected 2 references to Vector2");

    // 5. Test textDocument/foldingRange
    let mut fold_params = BTreeMap::new();
    let mut fold_doc = BTreeMap::new();
    fold_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    fold_params.insert("textDocument".to_string(), JsonValue::Object(fold_doc));

    let mut fold_req = BTreeMap::new();
    fold_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    fold_req.insert("id".to_string(), JsonValue::Number(104.0));
    fold_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/foldingRange".to_string()),
    );
    fold_req.insert("params".to_string(), JsonValue::Object(fold_params));

    let fold_resp = server
        .handle_message(&JsonValue::Object(fold_req))
        .expect("Expected foldingRange response");
    let fold_ranges = fold_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected folding ranges array");
    assert!(
        !fold_ranges.is_empty(),
        "Should have detected folding ranges"
    );

    let has_comment_fold = fold_ranges
        .iter()
        .any(|r| r.get("kind").and_then(|k| k.as_str()) == Some("comment"));
    assert!(has_comment_fold, "Should detect comment folding range");

    let has_imports_fold = fold_ranges
        .iter()
        .any(|r| r.get("kind").and_then(|k| k.as_str()) == Some("imports"));
    assert!(has_imports_fold, "Should detect imports folding range");
}

// ============================================================================
// 2. AUTOMATED DOCUMENTATION GENERATOR TESTS
// ============================================================================

#[test]
fn test_doc_extractor_and_generators() {
    let source = r#"
## Math utilities module for high-performance calculations.
## Provides trigonometry and vector operations.

## Mathematical constant PI (3.14159...)
pub const PI = 3.14159

## Represents a 2D geometric vector.
pub struct Vec2
    x: float = 0.0
    y: float = 0.0
end

## Computes Euclidean length of vector.
pub function Vec2.length(self) -> float
    return 1.0
end

## Renderable duck-typed interface protocol.
pub interface Renderable
    function render(self, ctx: int) -> void
end

## Available color palettes.
pub enum Color
    Red
    Green
    Blue
end

## Multiplies two 64-bit integers.
pub function multiply[T](a: int, b: int = 1) -> int
    return a * b
end
"#;

    let module = extract_module_docs(source, "math_utils.alya");
    assert_eq!(module.name, "math_utils");
    assert!(module.description.contains("Math utilities module"));

    // Check constants
    assert_eq!(module.constants.len(), 1);
    assert_eq!(module.constants[0].name, "PI");
    assert!(module.constants[0].doc.contains("Mathematical constant PI"));

    // Check structs & methods
    assert_eq!(module.structs.len(), 1);
    let st = &module.structs[0];
    assert_eq!(st.name, "Vec2");
    assert_eq!(st.fields.len(), 2);
    assert_eq!(st.methods.len(), 1);
    assert_eq!(st.methods[0].name, "length");

    // Check interfaces
    assert_eq!(module.interfaces.len(), 1);
    let iface = &module.interfaces[0];
    assert_eq!(iface.name, "Renderable");
    assert_eq!(iface.methods.len(), 1);

    // Check enums
    assert_eq!(module.enums.len(), 1);
    let en = &module.enums[0];
    assert_eq!(en.name, "Color");
    assert_eq!(en.variants.len(), 3);

    // Check functions
    assert_eq!(module.functions.len(), 1);
    let fn_item = &module.functions[0];
    assert_eq!(fn_item.name, "multiply");
    assert_eq!(fn_item.params.len(), 2);
    assert_eq!(fn_item.params[1].default_val.as_deref(), Some("1"));
    assert_eq!(fn_item.return_type.as_deref(), Some("int"));
    assert!(fn_item
        .signature
        .contains("function multiply[T](a: int, b: int = 1) -> int"));

    // Test Markdown Generator
    let md = generate_markdown(&module);
    assert!(md.contains("# Module `math_utils`"));
    assert!(md.contains("## Table of Contents"));
    assert!(md.contains("### Struct `Vec2`"));
    assert!(md.contains("### Interface `Renderable`"));
    assert!(md.contains("### Enum `Color`"));
    assert!(md.contains("### Function `multiply`"));

    // Test HTML Generator
    let html = generate_html(&module);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<title>Module math_utils - Alya Docs</title>"));
    assert!(html.contains("badge-pub"));
    assert!(html.contains("filterSymbols()"));
    assert!(html.contains("id=\"fn-multiply\""));
}

#[test]
fn test_doc_run_directory_e2e() {
    let tmp_dir = env::temp_dir().join(format!("alya_doc_test_{}", std::process::id()));
    let src_dir = tmp_dir.join("src");
    let out_dir = tmp_dir.join("docs");
    fs::create_dir_all(&src_dir).unwrap();

    let file1 = src_dir.join("core.alya");
    let file2 = src_dir.join("extra.alya");

    fs::write(
        &file1,
        "## Core system functions\npub function init() -> bool\n return true\nend\n",
    )
    .unwrap();
    fs::write(
        &file2,
        "## Extra helper functions\npub function helper() -> void\nend\n",
    )
    .unwrap();

    // Run doc generator over directory
    let res = run_doc(
        &src_dir.to_string_lossy(),
        Some(&out_dir.to_string_lossy()),
        true,
        true,
    );
    assert!(res.is_ok(), "run_doc should succeed: {:?}", res.err());

    assert!(out_dir.join("core.md").exists());
    assert!(out_dir.join("core.html").exists());
    assert!(out_dir.join("extra.md").exists());
    assert!(out_dir.join("extra.html").exists());
    assert!(out_dir.join("index.md").exists());
    assert!(out_dir.join("index.html").exists());

    let _ = fs::remove_dir_all(&tmp_dir);
}

// ============================================================================
// 3. DECOUPLED OFFICIAL PACKAGES REPOSITORY & SHA-256 TESTS
// ============================================================================

#[test]
fn test_official_packages_resolution() {
    // Official decoupled packages
    let (name1, auto_url1) = resolve_package_spec("alya-lang/http");
    assert_eq!(name1, "http");
    assert_eq!(auto_url1, None);
    assert_eq!(
        resolve_registry_url(&name1),
        "https://github.com/alya-lang/http.git"
    );

    let (name2, auto_url2) = resolve_package_spec("alya-lang/crypto");
    assert_eq!(name2, "crypto");
    assert_eq!(auto_url2, None);
    assert_eq!(
        resolve_registry_url(&name2),
        "https://github.com/alya-lang/crypto.git"
    );

    let (name3, auto_url3) = resolve_package_spec("alya-lang/sqlite");
    assert_eq!(name3, "sqlite");
    assert_eq!(auto_url3, None);
    assert_eq!(
        resolve_registry_url(&name3),
        "https://github.com/alya-lang/sqlite.git"
    );

    let (name4, auto_url4) = resolve_package_spec("alya-lang/cli");
    assert_eq!(name4, "cli");
    assert_eq!(auto_url4, None);
    assert_eq!(
        resolve_registry_url(&name4),
        "https://github.com/alya-lang/cli.git"
    );

    let (name5, auto_url5) = resolve_package_spec("alya-lang/logger");
    assert_eq!(name5, "logger");
    assert_eq!(auto_url5, None);
    assert_eq!(
        resolve_registry_url(&name5),
        "https://github.com/alya-lang/logger.git"
    );

    let (name6, auto_url6) = resolve_package_spec("alya-lang/json");
    assert_eq!(name6, "json");
    assert_eq!(auto_url6, None);
    assert_eq!(
        resolve_registry_url(&name6),
        "https://github.com/alya-lang/json.git"
    );

    // Third-party repository
    let (name_ext, auto_url_ext) = resolve_package_spec("octocat/hello-world");
    assert_eq!(name_ext, "hello-world");
    assert_eq!(
        auto_url_ext,
        Some("https://github.com/octocat/hello-world.git".to_string())
    );
}

#[test]
fn test_pure_rust_sha256_rfc_compliance() {
    // RFC 6234 standard test vectors
    // 1. ""
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // 2. "abc"
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // 3. "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn test_lockfile_serialization_with_checksum() {
    let pkg = LockedPackage {
        name: "http".to_string(),
        version: "1.2.0".to_string(),
        source: format_git_source(
            "https://github.com/alya-lang/http.git",
            None,
            Some("v1.2.0"),
            None,
            Some("a1b2c3d4e5f60718293041526374859607182930"),
        ),
        entry: "src/lib.alya".to_string(),
        checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            .to_string(),
        dependencies: vec!["std/net".to_string()],
    };

    let lock = PackageLock {
        version: 1,
        packages: vec![pkg.clone()],
    };

    let serialized = serialize_lockfile(&lock);
    assert!(serialized.contains("name = \"http\""));
    assert!(serialized.contains("version = \"1.2.0\""));
    assert!(serialized.contains(
        "checksum = \"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\""
    ));

    let parsed = parse_lockfile(&serialized).expect("Failed to parse lockfile");
    assert_eq!(parsed.version, 1);
    assert_eq!(parsed.packages.len(), 1);
    assert_eq!(parsed.packages[0].name, "http");
    assert_eq!(parsed.packages[0].checksum, pkg.checksum);
    assert_eq!(parsed.packages[0].dependencies, vec!["std/net"]);
}

#[test]
fn test_lsp_type_checker_diagnostics() {
    let mut server = ServerState::new();
    let uri = "file:///type_test.alya";

    // Open document with type mismatch
    let type_error_source = "let count: int = \"not an int\"\ncount = 42\n";
    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert(
        "text".to_string(),
        JsonValue::String(type_error_source.to_string()),
    );
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));

    let notif = server
        .handle_message(&JsonValue::Object(did_open))
        .expect("Expected notification");
    let diags = notif
        .get("params")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_array())
        .expect("Expected diagnostics array");

    assert!(!diags.is_empty(), "Should report type error diagnostic");
    let type_diag = diags
        .iter()
        .find(|d| d.get("source").and_then(|s| s.as_str()) == Some("alya-typecheck"));
    assert!(
        type_diag.is_some(),
        "Should find diagnostic from alya-typecheck"
    );
    let msg = type_diag
        .unwrap()
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap();
    assert!(msg.contains("Type mismatch in 'let count'"));
}

#[test]
fn test_lsp_milestone3_features() {
    let mut server = ServerState::new();

    // 1. Test initialize capabilities for Milestone 3
    let mut init_req = BTreeMap::new();
    init_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    init_req.insert("id".to_string(), JsonValue::Number(1.0));
    init_req.insert(
        "method".to_string(),
        JsonValue::String("initialize".to_string()),
    );
    init_req.insert("params".to_string(), JsonValue::Object(BTreeMap::new()));

    let init_resp = server
        .handle_message(&JsonValue::Object(init_req))
        .expect("Expected initialize response");
    let caps = init_resp
        .get("result")
        .and_then(|r| r.get("capabilities"))
        .expect("Expected capabilities");

    assert!(
        caps.get("signatureHelpProvider").is_some(),
        "signatureHelpProvider must be registered"
    );
    assert!(
        caps.get("renameProvider").is_some(),
        "renameProvider must be registered"
    );
    assert_eq!(
        caps.get("inlayHintProvider"),
        Some(&JsonValue::Bool(true)),
        "inlayHintProvider must be registered"
    );
    assert!(
        caps.get("semanticTokensProvider").is_some(),
        "semanticTokensProvider must be registered"
    );

    // 2. Open document with functions, calls, and variables
    let uri = "file:///workspace/milestone3.alya";
    let source = r#"# Scales vector components by factor
pub function scale(factor: float, delta: int = 1) -> float
    return factor * delta
end

function main()
    let count = 42
    let s = "hello"
    let res = scale(2.5, 10)
    say res
end
"#;

    let mut did_open_params = BTreeMap::new();
    let mut doc_info = BTreeMap::new();
    doc_info.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    doc_info.insert("text".to_string(), JsonValue::String(source.to_string()));
    did_open_params.insert("textDocument".to_string(), JsonValue::Object(doc_info));

    let mut did_open = BTreeMap::new();
    did_open.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    did_open.insert(
        "method".to_string(),
        JsonValue::String("textDocument/didOpen".to_string()),
    );
    did_open.insert("params".to_string(), JsonValue::Object(did_open_params));
    server.handle_message(&JsonValue::Object(did_open));

    // 3. Test textDocument/signatureHelp
    // Line 8: "    let res = scale(2.5, 10)" -> position inside second argument (character 26)
    let mut sig_params = BTreeMap::new();
    let mut sig_doc = BTreeMap::new();
    sig_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    sig_params.insert("textDocument".to_string(), JsonValue::Object(sig_doc));
    sig_params.insert("position".to_string(), Position::new(8, 26).to_json());

    let mut sig_req = BTreeMap::new();
    sig_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    sig_req.insert("id".to_string(), JsonValue::Number(2.0));
    sig_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/signatureHelp".to_string()),
    );
    sig_req.insert("params".to_string(), JsonValue::Object(sig_params));

    let sig_resp = server
        .handle_message(&JsonValue::Object(sig_req))
        .expect("Expected signatureHelp response");
    let sig_res = sig_resp
        .get("result")
        .expect("Expected signatureHelp result");
    let sigs = sig_res
        .get("signatures")
        .and_then(|s| s.as_array())
        .expect("Expected signatures array");
    assert_eq!(sigs.len(), 1);
    let sig_label = sigs[0].get("label").and_then(|l| l.as_str()).unwrap();
    assert!(sig_label.contains("scale(factor: float, delta: int = ...) -> float"));
    assert_eq!(
        sig_res.get("activeParameter"),
        Some(&JsonValue::Number(1.0)),
        "Active parameter should be 1 (delta)"
    );

    // 4. Test textDocument/prepareRename & textDocument/rename
    let mut prep_params = BTreeMap::new();
    let mut prep_doc = BTreeMap::new();
    prep_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    prep_params.insert("textDocument".to_string(), JsonValue::Object(prep_doc));
    // Line 1: "pub function scale(factor: float..." -> on 'scale' at column 14
    prep_params.insert("position".to_string(), Position::new(1, 14).to_json());

    let mut prep_req = BTreeMap::new();
    prep_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    prep_req.insert("id".to_string(), JsonValue::Number(3.0));
    prep_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/prepareRename".to_string()),
    );
    prep_req.insert("params".to_string(), JsonValue::Object(prep_params));

    let prep_resp = server
        .handle_message(&JsonValue::Object(prep_req))
        .expect("Expected prepareRename response");
    let prep_res = prep_resp
        .get("result")
        .expect("Expected prepareRename result");
    assert!(
        prep_res.get("start").is_some(),
        "prepareRename must return symbol range"
    );

    // Now execute textDocument/rename
    let mut ren_params = BTreeMap::new();
    let mut ren_doc = BTreeMap::new();
    ren_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    ren_params.insert("textDocument".to_string(), JsonValue::Object(ren_doc));
    ren_params.insert("position".to_string(), Position::new(1, 14).to_json());
    ren_params.insert(
        "newName".to_string(),
        JsonValue::String("scale_factor".to_string()),
    );

    let mut ren_req = BTreeMap::new();
    ren_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    ren_req.insert("id".to_string(), JsonValue::Number(4.0));
    ren_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/rename".to_string()),
    );
    ren_req.insert("params".to_string(), JsonValue::Object(ren_params));

    let ren_resp = server
        .handle_message(&JsonValue::Object(ren_req))
        .expect("Expected rename response");
    let ren_res = ren_resp.get("result").expect("Expected rename result");
    let changes = ren_res
        .get("changes")
        .and_then(|c| c.as_object())
        .expect("Expected changes map");
    let edits = changes
        .get(uri)
        .and_then(|e| e.as_array())
        .expect("Expected edits for uri");
    assert_eq!(
        edits.len(),
        2,
        "Both function declaration and call site should be renamed"
    );

    // 5. Test textDocument/inlayHint
    let mut hint_params = BTreeMap::new();
    let mut hint_doc = BTreeMap::new();
    hint_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    hint_params.insert("textDocument".to_string(), JsonValue::Object(hint_doc));

    let mut hint_req = BTreeMap::new();
    hint_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    hint_req.insert("id".to_string(), JsonValue::Number(5.0));
    hint_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/inlayHint".to_string()),
    );
    hint_req.insert("params".to_string(), JsonValue::Object(hint_params));

    let hint_resp = server
        .handle_message(&JsonValue::Object(hint_req))
        .expect("Expected inlayHint response");
    let hints = hint_resp
        .get("result")
        .and_then(|r| r.as_array())
        .expect("Expected hints array");
    assert!(!hints.is_empty(), "Inlay hints should not be empty");

    let has_int_hint = hints.iter().any(|h| {
        h.get("label").and_then(|l| l.as_str()) == Some(": int")
            && h.get("kind").and_then(|k| k.as_f64()) == Some(1.0)
    });
    assert!(
        has_int_hint,
        "Should generate ': int' type hint for 'let count = 42'"
    );

    let has_str_hint = hints.iter().any(|h| {
        h.get("label").and_then(|l| l.as_str()) == Some(": string")
            && h.get("kind").and_then(|k| k.as_f64()) == Some(1.0)
    });
    assert!(
        has_str_hint,
        "Should generate ': string' type hint for 'let s = \"hello\"'"
    );

    // 6. Test textDocument/semanticTokens/full
    let mut sem_params = BTreeMap::new();
    let mut sem_doc = BTreeMap::new();
    sem_doc.insert("uri".to_string(), JsonValue::String(uri.to_string()));
    sem_params.insert("textDocument".to_string(), JsonValue::Object(sem_doc));

    let mut sem_req = BTreeMap::new();
    sem_req.insert("jsonrpc".to_string(), JsonValue::String("2.0".to_string()));
    sem_req.insert("id".to_string(), JsonValue::Number(6.0));
    sem_req.insert(
        "method".to_string(),
        JsonValue::String("textDocument/semanticTokens/full".to_string()),
    );
    sem_req.insert("params".to_string(), JsonValue::Object(sem_params));

    let sem_resp = server
        .handle_message(&JsonValue::Object(sem_req))
        .expect("Expected semanticTokens response");
    let sem_res = sem_resp
        .get("result")
        .expect("Expected semanticTokens result");
    let sem_data = sem_res
        .get("data")
        .and_then(|d| d.as_array())
        .expect("Expected semantic data array");
    assert!(
        !sem_data.is_empty(),
        "Semantic tokens data must not be empty"
    );
    assert_eq!(
        sem_data.len() % 5,
        0,
        "Semantic token integer stream must be multiples of 5"
    );
}

#[test]
fn test_dap_server_lifecycle() {
    use alya::tools::dap::DapServerState;

    let mut dap = DapServerState::new();

    // 1. Initialize
    let mut init_req = BTreeMap::new();
    init_req.insert("seq".to_string(), JsonValue::Number(1.0));
    init_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    init_req.insert(
        "command".to_string(),
        JsonValue::String("initialize".to_string()),
    );

    let init_resps = dap.handle_message(&JsonValue::Object(init_req));
    assert_eq!(
        init_resps.len(),
        2,
        "Initialize should return response and initialized event"
    );
    assert_eq!(
        init_resps[0].get("command").and_then(|c| c.as_str()),
        Some("initialize")
    );
    assert_eq!(
        init_resps[0].get("success").and_then(|s| s.as_bool()),
        Some(true)
    );
    assert_eq!(
        init_resps[1].get("event").and_then(|e| e.as_str()),
        Some("initialized")
    );

    // 2. Set Breakpoints
    let sample_source = r#"const PI = 3.14159

function compute_total(price: float, tax: float) -> float
    let factor = 1.2
    let subtotal = price * factor
    return subtotal + tax
end

function main()
    let initial_price = 100.0
    let tax_rate = 18.0
    let total = compute_total(initial_price, tax_rate)
    say total
end
"#;

    let test_file = "test_app.alya";
    dap.load_source(sample_source);
    dap.program_path = test_file.to_string();

    let mut bp_args = BTreeMap::new();
    let mut src_map = BTreeMap::new();
    src_map.insert("path".to_string(), JsonValue::String(test_file.to_string()));
    bp_args.insert("source".to_string(), JsonValue::Object(src_map));
    bp_args.insert(
        "lines".to_string(),
        JsonValue::Array(vec![JsonValue::Number(10.0), JsonValue::Number(12.0)]),
    );

    let mut bp_req = BTreeMap::new();
    bp_req.insert("seq".to_string(), JsonValue::Number(2.0));
    bp_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    bp_req.insert(
        "command".to_string(),
        JsonValue::String("setBreakPoints".to_string()),
    );
    bp_req.insert("arguments".to_string(), JsonValue::Object(bp_args));

    let bp_resps = dap.handle_message(&JsonValue::Object(bp_req));
    assert_eq!(bp_resps.len(), 1);
    let bps = bp_resps[0]
        .get("body")
        .and_then(|b| b.get("breakpoints"))
        .and_then(|arr| arr.as_array())
        .expect("breakpoints");
    assert_eq!(bps.len(), 2);
    assert_eq!(bps[0].get("verified").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(bps[0].get("line").and_then(|l| l.as_i64()), Some(10));

    // 3. Launch & configurationDone
    let mut launch_args = BTreeMap::new();
    launch_args.insert(
        "program".to_string(),
        JsonValue::String(test_file.to_string()),
    );
    launch_args.insert("stopOnEntry".to_string(), JsonValue::Bool(false));
    launch_args.insert("memTrace".to_string(), JsonValue::Bool(true));

    let mut launch_req = BTreeMap::new();
    launch_req.insert("seq".to_string(), JsonValue::Number(3.0));
    launch_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    launch_req.insert(
        "command".to_string(),
        JsonValue::String("launch".to_string()),
    );
    launch_req.insert("arguments".to_string(), JsonValue::Object(launch_args));

    let launch_resps = dap.handle_message(&JsonValue::Object(launch_req));
    assert_eq!(launch_resps.len(), 1);
    assert_eq!(
        launch_resps[0].get("success").and_then(|s| s.as_bool()),
        Some(true)
    );

    let mut conf_req = BTreeMap::new();
    conf_req.insert("seq".to_string(), JsonValue::Number(4.0));
    conf_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    conf_req.insert(
        "command".to_string(),
        JsonValue::String("configurationDone".to_string()),
    );

    let conf_resps = dap.handle_message(&JsonValue::Object(conf_req));
    assert_eq!(
        conf_resps.len(),
        2,
        "configurationDone should return response and stopped event"
    );
    assert_eq!(
        conf_resps[1].get("event").and_then(|e| e.as_str()),
        Some("stopped")
    );
    assert_eq!(
        conf_resps[1]
            .get("body")
            .and_then(|b| b.get("reason"))
            .and_then(|r| r.as_str()),
        Some("breakpoint")
    );
    assert_eq!(
        dap.current_line, 10,
        "Should be paused at first breakpoint line 10"
    );

    // 4. Threads & StackTrace
    let mut threads_req = BTreeMap::new();
    threads_req.insert("seq".to_string(), JsonValue::Number(5.0));
    threads_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    threads_req.insert(
        "command".to_string(),
        JsonValue::String("threads".to_string()),
    );

    let thread_resps = dap.handle_message(&JsonValue::Object(threads_req));
    let threads = thread_resps[0]
        .get("body")
        .and_then(|b| b.get("threads"))
        .and_then(|t| t.as_array())
        .unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(
        threads[0].get("name").and_then(|n| n.as_str()),
        Some("Main Fiber (Thread 1)")
    );

    let mut stack_req = BTreeMap::new();
    stack_req.insert("seq".to_string(), JsonValue::Number(6.0));
    stack_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    stack_req.insert(
        "command".to_string(),
        JsonValue::String("stackTrace".to_string()),
    );

    let stack_resps = dap.handle_message(&JsonValue::Object(stack_req));
    let frames = stack_resps[0]
        .get("body")
        .and_then(|b| b.get("stackFrames"))
        .and_then(|f| f.as_array())
        .unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].get("line").and_then(|l| l.as_i64()), Some(10));
    assert_eq!(
        frames[0].get("name").and_then(|n| n.as_str()),
        Some("main()")
    );

    // 5. Scopes & Variables
    let mut scopes_req = BTreeMap::new();
    scopes_req.insert("seq".to_string(), JsonValue::Number(7.0));
    scopes_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    scopes_req.insert(
        "command".to_string(),
        JsonValue::String("scopes".to_string()),
    );

    let scopes_resps = dap.handle_message(&JsonValue::Object(scopes_req));
    let scopes = scopes_resps[0]
        .get("body")
        .and_then(|b| b.get("scopes"))
        .and_then(|s| s.as_array())
        .unwrap();
    assert_eq!(scopes.len(), 3);

    // Read Locals (variablesReference: 1001)
    let mut vars_args = BTreeMap::new();
    vars_args.insert("variablesReference".to_string(), JsonValue::Number(1001.0));
    let mut vars_req = BTreeMap::new();
    vars_req.insert("seq".to_string(), JsonValue::Number(8.0));
    vars_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    vars_req.insert(
        "command".to_string(),
        JsonValue::String("variables".to_string()),
    );
    vars_req.insert("arguments".to_string(), JsonValue::Object(vars_args));

    let vars_resps = dap.handle_message(&JsonValue::Object(vars_req));
    let vars = vars_resps[0]
        .get("body")
        .and_then(|b| b.get("variables"))
        .and_then(|v| v.as_array())
        .unwrap();
    let var_names: Vec<&str> = vars
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        var_names.contains(&"initial_price"),
        "Locals must contain initial_price"
    );

    // 6. Variable Evaluation
    let mut eval_args = BTreeMap::new();
    eval_args.insert(
        "expression".to_string(),
        JsonValue::String("initial_price * 2".to_string()),
    );
    let mut eval_req = BTreeMap::new();
    eval_req.insert("seq".to_string(), JsonValue::Number(9.0));
    eval_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    eval_req.insert(
        "command".to_string(),
        JsonValue::String("evaluate".to_string()),
    );
    eval_req.insert("arguments".to_string(), JsonValue::Object(eval_args));

    let eval_resps = dap.handle_message(&JsonValue::Object(eval_req));
    let eval_body = eval_resps[0].get("body").unwrap();
    assert_eq!(
        eval_body.get("result").and_then(|r| r.as_str()),
        Some("200.00")
    );
    assert_eq!(
        eval_body.get("type").and_then(|t| t.as_str()),
        Some("float")
    );

    // 7. Step Over & Continue
    let mut next_req = BTreeMap::new();
    next_req.insert("seq".to_string(), JsonValue::Number(10.0));
    next_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    next_req.insert("command".to_string(), JsonValue::String("next".to_string()));

    let next_resps = dap.handle_message(&JsonValue::Object(next_req));
    assert_eq!(
        next_resps[1].get("event").and_then(|e| e.as_str()),
        Some("stopped")
    );
    assert_eq!(
        next_resps[1]
            .get("body")
            .and_then(|b| b.get("reason"))
            .and_then(|r| r.as_str()),
        Some("step")
    );
    assert_eq!(dap.current_line, 11);

    // Continue to next breakpoint at line 12
    let mut cont_req = BTreeMap::new();
    cont_req.insert("seq".to_string(), JsonValue::Number(11.0));
    cont_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    cont_req.insert(
        "command".to_string(),
        JsonValue::String("continue".to_string()),
    );

    let cont_resps = dap.handle_message(&JsonValue::Object(cont_req.clone()));
    assert_eq!(
        cont_resps[1].get("event").and_then(|e| e.as_str()),
        Some("stopped")
    );
    assert_eq!(
        dap.current_line, 12,
        "Should hit second breakpoint at line 12"
    );

    // Continue to termination
    let cont_resps2 = dap.handle_message(&JsonValue::Object(cont_req));
    let events: Vec<&str> = cont_resps2
        .iter()
        .filter_map(|m| m.get("event").and_then(|e| e.as_str()))
        .collect();
    assert!(events.contains(&"output"));
    assert!(events.contains(&"terminated"));
    assert!(events.contains(&"exited"));

    // 8. Disconnect
    let mut disc_req = BTreeMap::new();
    disc_req.insert("seq".to_string(), JsonValue::Number(12.0));
    disc_req.insert("type".to_string(), JsonValue::String("request".to_string()));
    disc_req.insert(
        "command".to_string(),
        JsonValue::String("disconnect".to_string()),
    );

    let disc_resps = dap.handle_message(&JsonValue::Object(disc_req));
    assert_eq!(
        disc_resps[0].get("success").and_then(|s| s.as_bool()),
        Some(true)
    );
    assert!(dap.is_shutdown);
}
