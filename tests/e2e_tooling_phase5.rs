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
