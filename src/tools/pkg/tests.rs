use super::*;
use std::fs;

#[test]
fn test_sha256_known_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn test_manifest_parse_and_serialize() {
    let toml = r#"
[package]
name = "demo_pkg"
version = "1.2.3"
entry = "src/main.alya"
authors = ["Alice", "Bob"]
description = "A great package"
license = "MIT"
homepage = "https://github.com/alya-lang/alya"
repository = "https://github.com/alya-lang/demo"
keywords = ["alya", "demo", "fast"]

[dependencies]
local_lib = { path = "../libs/local" }
git_lib = { git = "https://github.com/test/repo", tag = "v1.0" }
simple_ver = "0.5.0"
"#;

    let manifest = parse_manifest(toml).expect("parse manifest failed");
    assert_eq!(manifest.package.name, "demo_pkg");
    assert_eq!(manifest.package.version, "1.2.3");
    assert_eq!(manifest.package.entry, "src/main.alya");
    assert_eq!(manifest.package.authors, vec!["Alice", "Bob"]);
    assert_eq!(
        manifest.package.description,
        Some("A great package".to_string())
    );
    assert_eq!(manifest.package.license, Some("MIT".to_string()));
    assert_eq!(
        manifest.package.homepage,
        Some("https://github.com/alya-lang/alya".to_string())
    );
    assert_eq!(
        manifest.package.repository,
        Some("https://github.com/alya-lang/demo".to_string())
    );
    assert_eq!(manifest.package.keywords, vec!["alya", "demo", "fast"]);
    assert_eq!(manifest.dependencies.len(), 3);

    let serialized = serialize_manifest(&manifest);
    let manifest2 = parse_manifest(&serialized).expect("roundtrip parse failed");
    assert_eq!(manifest, manifest2);
}

#[test]
fn test_manifest_multiline_support() {
    let toml = r#"
[package]
name = "vpn"
version = "0.1.0"
authors = [
    "Alice",
    "Bob"
]

[build]
c-sources = [
    "c/crypto.c",
    "c/proc_resolver.c"
]
c-flags = [
    "-O2"
]
c-include-dirs = [
    "c"
]

[dependencies]
local_lib = {
    path = "../local"
}
"#;

    let manifest = parse_manifest(toml).expect("parse multiline manifest failed");
    assert_eq!(manifest.package.name, "vpn");
    assert_eq!(manifest.package.authors, vec!["Alice", "Bob"]);
    let build = manifest.build.expect("build section expected");
    assert_eq!(build.c_sources, vec!["c/crypto.c", "c/proc_resolver.c"]);
    assert_eq!(build.c_flags, vec!["-O2"]);
    assert_eq!(build.c_include_dirs, vec!["c"]);
    assert_eq!(manifest.dependencies.len(), 1);
}

#[test]
fn test_compiler_compatibility() {
    let toml_ok = r#"
[package]
name = "compatible_pkg"
version = "1.0.0"
alya-version = "0.0.5"
"#;
    let manifest_ok = parse_manifest(toml_ok).unwrap();
    assert_eq!(manifest_ok.package.alya_version, Some("0.0.5".to_string()));
    assert!(check_compiler_compatibility(&manifest_ok).is_ok());

    let toml_incompatible = r#"
[package]
name = "future_pkg"
version = "1.0.0"
alya-version = "99.0.0"
"#;
    let manifest_incompatible = parse_manifest(toml_incompatible).unwrap();
    let err = check_compiler_compatibility(&manifest_incompatible).unwrap_err();
    assert!(err.contains("requires Alya compiler version >="));
}

#[test]
fn test_lockfile_parse_and_serialize() {
    let lock_toml = r#"# Auto-generated
version = 1

[[package]]
name = "raylib"
version = "0.1.0"
source = "path:../libs/raylib"
entry = "../libs/raylib/src/main.alya"
checksum = "sha256:1234567890abcdef"
dependencies = [
 "c_bridge",
]

[[package]]
name = "sqlite"
version = "v1.0.0"
source = "git:https://github.com/alya-lang/sqlite#v1.0.0"
entry = ".alya/packages/sqlite/src/main.alya"
checksum = "sha256:abcdef1234567890"
"#;

    let lock = parse_lockfile(lock_toml).expect("parse lockfile failed");
    assert_eq!(lock.version, 1);
    assert_eq!(lock.packages.len(), 2);
    assert_eq!(lock.packages[0].name, "raylib");
    assert_eq!(lock.packages[0].dependencies, vec!["c_bridge"]);
    assert_eq!(lock.packages[1].name, "sqlite");
    assert!(lock.packages[1].dependencies.is_empty());

    let serialized = serialize_lockfile(&lock);
    let lock2 = parse_lockfile(&serialized).expect("roundtrip parse failed");
    assert_eq!(lock, lock2);
}

#[test]
fn test_git_source_formatting_and_rev_parsing() {
    let url = "https://github.com/alya-lang/rand";
    let sha = "aa1446c94360e0059c0024f3600e553b5df19332";

    // Branch with SHA
    let src_branch = format_git_source(url, Some("main"), None, None, Some(sha));
    assert_eq!(
        src_branch,
        "git:https://github.com/alya-lang/rand?branch=main#aa1446c94360e0059c0024f3600e553b5df19332"
    );
    assert_eq!(parse_git_source_rev(&src_branch), Some(sha.to_string()));

    // Tag with SHA
    let src_tag = format_git_source(url, None, Some("v0.2.0"), None, Some(sha));
    assert_eq!(
        src_tag,
        "git:https://github.com/alya-lang/rand?tag=v0.2.0#aa1446c94360e0059c0024f3600e553b5df19332"
    );
    assert_eq!(parse_git_source_rev(&src_tag), Some(sha.to_string()));

    // Rev directly
    let src_rev = format_git_source(url, None, None, Some(sha), Some(sha));
    assert_eq!(
        src_rev,
        "git:https://github.com/alya-lang/rand#aa1446c94360e0059c0024f3600e553b5df19332"
    );
    assert_eq!(parse_git_source_rev(&src_rev), Some(sha.to_string()));

    // Fallbacks without SHA
    let src_branch_nosha = format_git_source(url, Some("main"), None, None, None);
    assert_eq!(
        src_branch_nosha,
        "git:https://github.com/alya-lang/rand#main"
    );
    assert_eq!(parse_git_source_rev(&src_branch_nosha), None);

    // Non-git source
    assert_eq!(
        parse_git_source_rev("registry+https://github.com/alya-lang/http.git#v0.1.0"),
        None
    );
}

#[test]
fn test_lockfile_dependencies_variations() {
    let lock_toml = r#"version = 1

[[package]]
name = "app"
version = "0.1.0"
source = "path:."
entry = "src/main.alya"
checksum = "sha256:1111"
dependencies = ["dep_inline_1", "dep_inline_2"]

[[package]]
name = "leaf"
version = "0.1.0"
source = "path:../leaf"
entry = "src/lib.alya"
checksum = "sha256:2222"
dependencies = []
"#;

    let lock = parse_lockfile(lock_toml).expect("parse failed");
    assert_eq!(
        lock.packages[0].dependencies,
        vec!["dep_inline_1", "dep_inline_2"]
    );
    assert!(lock.packages[1].dependencies.is_empty());

    let serialized = serialize_lockfile(&lock);
    assert!(serialized.contains("dependencies = [\n \"dep_inline_1\",\n \"dep_inline_2\",\n]"));
    assert!(!serialized.contains("name = \"leaf\"\nversion = \"0.1.0\"\nsource = \"path:../leaf\"\nentry = \"src/lib.alya\"\nchecksum = \"sha256:2222\"\ndependencies = ["));
}

#[test]
fn test_pkg_init_lifecycle() {
    let temp_dir = std::env::temp_dir().join(format!(
        "alya_pkg_test_init_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let temp_str = temp_dir.to_string_lossy().to_string();

    let res = run_init(Some(&temp_str), Some("test_lib"), true);
    assert!(res.is_ok(), "run_init failed: {:?}", res);

    assert!(temp_dir.join("alya.toml").exists());
    assert!(temp_dir.join("src").join("lib.alya").exists());
    assert!(temp_dir.join(".gitignore").exists());

    let content = fs::read_to_string(temp_dir.join("alya.toml")).unwrap();
    let manifest = parse_manifest(&content).unwrap();
    assert_eq!(manifest.package.name, "test_lib");
    assert_eq!(manifest.package.entry, "src/lib.alya");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_pkg_add_and_install_path_dependency() {
    let base_temp = std::env::temp_dir().join(format!(
        "alya_pkg_dep_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let lib_dir = base_temp.join("math_lib");
    let app_dir = base_temp.join("my_app");

    let _ = fs::create_dir_all(&lib_dir);
    let _ = fs::create_dir_all(&app_dir);

    run_init(Some(&lib_dir.to_string_lossy()), Some("math_lib"), true).unwrap();

    run_init(Some(&app_dir.to_string_lossy()), Some("my_app"), false).unwrap();

    // Write custom library function
    fs::write(
        lib_dir.join("src").join("lib.alya"),
        "function add(a, b)\n    return a + b\nend\n",
    )
    .unwrap();

    // Add math_lib as relative dependency in app's alya.toml
    let mut app_manifest =
        parse_manifest(&fs::read_to_string(app_dir.join("alya.toml")).unwrap()).unwrap();
    app_manifest.dependencies.insert(
        "math_lib".to_string(),
        DependencySource::Path {
            path: "../math_lib".to_string(),
        },
    );
    fs::write(app_dir.join("alya.toml"), serialize_manifest(&app_manifest)).unwrap();

    // Run install in app_dir
    run_install_in(&app_dir).unwrap();

    assert!(app_dir.join("alya.lock").exists());
    let lock = parse_lockfile(&fs::read_to_string(app_dir.join("alya.lock")).unwrap()).unwrap();
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].name, "math_lib");
    assert!(lock.packages[0].checksum.starts_with("sha256:"));

    // Test compiler resolution
    let resolved = resolve_package_import("math_lib", &app_dir).unwrap();
    assert!(resolved.is_some());
    let resolved_path = resolved.unwrap();
    assert!(resolved_path.ends_with("lib.alya"));

    let _ = fs::remove_dir_all(&base_temp);
}

#[test]
fn test_resolve_archive_candidates() {
    let gh_branch = resolve_archive_candidates(
        "https://github.com/alya-lang/dotenv",
        None,
        Some("develop"),
        None,
    );
    assert_eq!(gh_branch.len(), 2);
    assert!(gh_branch[0].contains("archive/refs/heads/develop.tar.gz"));
    assert!(gh_branch[1].contains("archive/refs/heads/develop.zip"));

    let gh_tag = resolve_archive_candidates(
        "https://github.com/alya-lang/dotenv.git",
        Some("v0.1.0"),
        None,
        None,
    );
    assert_eq!(gh_tag.len(), 4);
    assert!(gh_tag[0].contains("archive/refs/tags/v0.1.0.tar.gz"));
    assert!(gh_tag[1].contains("archive/refs/tags/v0.1.0.zip"));
    assert!(gh_tag[2].contains("archive/refs/tags/0.1.0.tar.gz"));
    assert!(gh_tag[3].contains("archive/refs/tags/0.1.0.zip"));

    let gh_default =
        resolve_archive_candidates("git@github.com:alya-lang/dotenv.git", None, None, None);
    assert!(gh_default.iter().any(|u| u.contains("heads/main.tar.gz")));

    let gl =
        resolve_archive_candidates("https://gitlab.com/group/repo", Some("v1.0.0"), None, None);
    assert!(gl[0].contains("gitlab.com/group/repo/-/archive/v1.0.0"));
}

#[test]
fn test_download_and_extract_archive_live() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_archive_live_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let candidates = resolve_archive_candidates(
        "https://github.com/alya-lang/dotenv",
        None,
        Some("main"),
        None,
    );
    let res = try_download_and_extract_archive(&candidates, &temp_dir, "dotenv");
    if res.is_ok() {
        assert!(temp_dir.join("alya.toml").exists());
        assert!(temp_dir.join("src").join("lib.alya").exists());
    }
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1536), "1.50 KB");
    assert_eq!(format_bytes(1048576), "1.00 MB");
    assert_eq!(format_bytes(1073741824), "1.00 GB");
}

#[test]
fn test_pkg_cache_inspection_and_clean() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_cache_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    let packages_dir = temp_dir.join(".alya").join("packages");

    let pkg1_dir = packages_dir.join("dummy_pkg");
    let _ = fs::create_dir_all(&pkg1_dir);
    fs::write(
        pkg1_dir.join("alya.toml"),
        "[package]\nname = \"dummy_pkg\"\nversion = \"1.2.3\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(pkg1_dir.join("dummy.txt"), "hello world").unwrap();

    let (size, count) = dir_size_and_count(&packages_dir);
    assert!(size > 0);
    assert_eq!(count, 2);

    let details = inspect_packages_dir(&packages_dir, None);
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].name, "dummy_pkg");
    assert_eq!(details[0].version, "v1.2.3");
    assert!(details[0].size_bytes > 0);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_global_cache_and_copy_dir_all() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_cache_copy_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let cache_key = compute_cache_key("crypto", "v0.1.0", "https://github.com/alya-lang/crypto");
    assert!(cache_key.starts_with("crypto@v0.1.0-"));

    let src_dir = temp_dir.join("cache").join(&cache_key);
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(src_dir.join(".git")).unwrap();
    fs::create_dir_all(src_dir.join("src")).unwrap();

    fs::write(
        src_dir.join("alya.toml"),
        "[package]\nname = \"crypto\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(
        src_dir.join(".alya-source"),
        "git:https://github.com/alya-lang/crypto#v0.1.0",
    )
    .unwrap();
    fs::write(src_dir.join(".git").join("config"), "[core]").unwrap();
    fs::write(src_dir.join("src").join("lib.alya"), "fn hash() {}").unwrap();

    // Test inspect_packages_dir on global cache (lock is None)
    let cache_details = inspect_packages_dir(&temp_dir.join("cache"), None);
    assert_eq!(cache_details.len(), 1);
    assert_eq!(cache_details[0].name, "crypto");
    assert_eq!(cache_details[0].version, "v0.1.0");
    assert_eq!(
        cache_details[0].source,
        "git:https://github.com/alya-lang/crypto#v0.1.0"
    );

    // Test copy_dir_all with skip_git
    let target_dir = temp_dir
        .join("project")
        .join(".alya")
        .join("packages")
        .join("crypto");
    copy_dir_all(&src_dir, &target_dir, true).unwrap();

    assert!(target_dir.join("alya.toml").exists());
    assert!(target_dir.join("src").join("lib.alya").exists());
    assert!(!target_dir.join(".git").exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_transitive_dependency_resolution() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_transitive_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let root_dir = temp_dir.join("app");
    let pkg_a_dir = temp_dir.join("pkg_a");
    let pkg_b_dir = temp_dir.join("pkg_b");

    fs::create_dir_all(&root_dir).unwrap();
    fs::create_dir_all(pkg_a_dir.join("src")).unwrap();
    fs::create_dir_all(pkg_b_dir.join("src")).unwrap();

    // pkg_b: leaf dependency
    fs::write(
        pkg_b_dir.join("alya.toml"),
        "[package]\nname = \"pkg_b\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(pkg_b_dir.join("src").join("lib.alya"), "let b_val = 42\n").unwrap();

    // pkg_a: depends on pkg_b via relative path
    fs::write(
        pkg_a_dir.join("alya.toml"),
        "[package]\nname = \"pkg_a\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\n\n[dependencies]\npkg_b = { path = \"../pkg_b\" }\n",
    )
    .unwrap();
    fs::write(
        pkg_a_dir.join("src").join("lib.alya"),
        "import \"pkg_b\"\nlet a_val = 100\n",
    )
    .unwrap();

    // root app: depends only on pkg_a via relative path
    fs::write(
        root_dir.join("alya.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\npkg_a = { path = \"../pkg_a\" }\n",
    )
    .unwrap();

    // Run install in root
    let res = run_install_in(&root_dir);
    assert!(res.is_ok(), "run_install_in failed: {:?}", res.err());

    // Verify lockfile contains BOTH pkg_a and pkg_b!
    let lock_content = fs::read_to_string(root_dir.join("alya.lock")).unwrap();
    let lock = parse_lockfile(&lock_content).unwrap();
    assert_eq!(lock.packages.len(), 2);
    let pkg_a = lock.packages.iter().find(|p| p.name == "pkg_a").unwrap();
    let pkg_b = lock.packages.iter().find(|p| p.name == "pkg_b").unwrap();
    assert_eq!(pkg_a.dependencies, vec!["pkg_b"]);
    assert!(pkg_b.dependencies.is_empty());
    assert!(lock_content.contains("dependencies = [\n \"pkg_b\",\n]"));

    // Verify module resolution from pkg_a resolving pkg_b
    let resolved_b = resolve_package_import("pkg_b", &pkg_a_dir.join("src")).unwrap();
    assert!(resolved_b.is_some());
    assert_eq!(
        resolved_b.unwrap().canonicalize().unwrap(),
        pkg_b_dir
            .join("src")
            .join("lib.alya")
            .canonicalize()
            .unwrap()
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_transitive_nested_packages_import_resolution() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_trans_nest_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let app_dir = temp_dir.join("my_app");
    let packages_dir = app_dir.join(".alya").join("packages");
    let pkg_crypto = packages_dir.join("crypto");
    let pkg_abc = packages_dir.join("abc");

    fs::create_dir_all(pkg_crypto.join("src")).unwrap();
    fs::create_dir_all(pkg_abc.join("src")).unwrap();

    fs::write(
        app_dir.join("alya.toml"),
        "[package]\nname = \"my_app\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\ncrypto = { git = \"https://github.com/alya-lang/crypto\", tag = \"v0.1.0\" }\n",
    )
    .unwrap();

    fs::write(
        pkg_crypto.join("alya.toml"),
        "[package]\nname = \"crypto\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\n\n[dependencies]\nabc = { git = \"https://github.com/alya-lang/abc\", tag = \"v1.0.0\" }\n",
    )
    .unwrap();
    fs::write(
        pkg_crypto.join("src").join("lib.alya"),
        "import \"abc\"\nlet c = 1\n",
    )
    .unwrap();

    fs::write(
        pkg_abc.join("alya.toml"),
        "[package]\nname = \"abc\"\nversion = \"1.0.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(pkg_abc.join("src").join("lib.alya"), "let abc_num = 999\n").unwrap();

    // When inside crypto/src/lib.alya, resolve_package_import("abc") searches upward and finds app/.alya/packages/abc!
    let resolved = resolve_package_import("abc", &pkg_crypto.join("src")).unwrap();
    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().canonicalize().unwrap(),
        pkg_abc.join("src").join("lib.alya").canonicalize().unwrap()
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_resolve_package_spec() {
    assert_eq!(resolve_package_spec("http"), ("http".to_string(), None));
    assert_eq!(
        resolve_package_spec("alya-lang/http"),
        ("http".to_string(), None)
    );
    assert_eq!(
        resolve_package_spec("myorg/cool-lib"),
        (
            "cool-lib".to_string(),
            Some("https://github.com/myorg/cool-lib.git".to_string())
        )
    );
    assert_eq!(
        resolve_package_spec("https://github.com/foo/bar.git"),
        (
            "bar".to_string(),
            Some("https://github.com/foo/bar.git".to_string())
        )
    );
}

#[test]
fn test_resolve_registry_url() {
    // Without ALYA_REGISTRY set
    let default_url = resolve_registry_url("crypto");
    assert_eq!(default_url, "https://github.com/alya-lang/crypto.git");
}

#[test]
fn test_version_dependency_resolution_and_locking() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_ver_dep_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let app_dir = temp_dir.join("test_app");
    let packages_dir = app_dir.join(".alya").join("packages");
    let pkg_http = packages_dir.join("http");

    fs::create_dir_all(pkg_http.join("src")).unwrap();

    fs::write(
        pkg_http.join("alya.toml"),
        "[package]\nname = \"http\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(
        pkg_http.join("src").join("lib.alya"),
        "function get() { return 200 }\n",
    )
    .unwrap();

    // Project manifest specifies version dependency (SemVer)
    fs::write(
        app_dir.join("alya.toml"),
        "[package]\nname = \"test_app\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nhttp = \"0.1.0\"\n",
    )
    .unwrap();

    let res = run_install_in(&app_dir);
    assert!(res.is_ok(), "run_install_in failed: {:?}", res.err());

    // Verify lockfile
    let lock_content = fs::read_to_string(app_dir.join("alya.lock")).unwrap();
    let lock = parse_lockfile(&lock_content).unwrap();
    let locked_http = lock.packages.iter().find(|p| p.name == "http").unwrap();
    assert_eq!(locked_http.name, "http");
    assert_eq!(locked_http.version, "0.1.0");
    assert!(locked_http
        .source
        .starts_with("registry+https://github.com/alya-lang/http.git#v0.1.0"));
    assert!(locked_http.entry.ends_with("src/lib.alya"));
    assert!(locked_http.checksum.starts_with("sha256:"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_semver_coalescing_and_compatibility() {
    assert_eq!(semver_major("1.2.3"), Some(1));
    assert_eq!(semver_major("^1.4.0"), Some(1));
    assert_eq!(semver_major("~2.5.0"), Some(2));
    assert_eq!(semver_major(">=3.0.0"), Some(3));
    assert_eq!(semver_major("foo"), None);

    assert!(is_semver_compatible("^1.1.0", "^1.4.0"));
    assert!(is_semver_compatible("1.2.0", "1.5.0"));
    assert!(!is_semver_compatible("1.0.0", "2.0.0"));
    assert!(!is_semver_compatible("^1.0.0", "^2.0.0"));

    assert_eq!(
        coalesce_semver_versions("^1.1.0", "^1.4.0").unwrap(),
        "^1.4.0"
    );
    assert_eq!(coalesce_semver_versions("1.5.0", "1.2.0").unwrap(), "1.5.0");
    assert!(coalesce_semver_versions("1.0.0", "2.0.0").is_err());
}

#[test]
fn test_duplicate_native_links_rejection() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_links_dup_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let app_dir = temp_dir.join("app");
    let lib_a = temp_dir.join("lib_a");
    let lib_b = temp_dir.join("lib_b");

    fs::create_dir_all(lib_a.join("src")).unwrap();
    fs::create_dir_all(lib_b.join("src")).unwrap();
    fs::create_dir_all(app_dir.join("src")).unwrap();

    fs::write(
        lib_a.join("alya.toml"),
        "[package]\nname = \"lib_a\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\nlinks = \"sqlite3\"\n",
    )
    .unwrap();
    fs::write(lib_a.join("src").join("lib.alya"), "function a() {}\n").unwrap();

    fs::write(
        lib_b.join("alya.toml"),
        "[package]\nname = \"lib_b\"\nversion = \"0.1.0\"\nentry = \"src/lib.alya\"\nlinks = \"sqlite3\"\n",
    )
    .unwrap();
    fs::write(lib_b.join("src").join("lib.alya"), "function b() {}\n").unwrap();

    fs::write(
        app_dir.join("alya.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nlib_a = {{ path = \"{}\" }}\nlib_b = {{ path = \"{}\" }}\n",
            lib_a.display().to_string().replace('\\', "/"),
            lib_b.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();

    let res = run_install_in(&app_dir);
    assert!(res.is_err());
    let err_msg = res.err().unwrap();
    assert!(
        err_msg.contains("Duplicate native C library link 'sqlite3' required by both"),
        "Unexpected error: {}",
        err_msg
    );
    assert!(
        err_msg.contains("Align dependency versions to resolve"),
        "Unexpected error: {}",
        err_msg
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_strict_direct_dependency_isolation_diagnostic() {
    let temp_dir =
        std::env::temp_dir().join(format!("alya_test_direct_iso_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let app_dir = temp_dir.join("app");
    let pkg_dir = app_dir
        .join(".alya")
        .join("packages")
        .join("transitive_pkg");
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(
        pkg_dir.join("alya.toml"),
        "[package]\nname = \"transitive_pkg\"\nversion = \"1.0.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(
        pkg_dir.join("src").join("lib.alya"),
        "pub function util() {}\n",
    )
    .unwrap();

    // App manifest only depends on "direct_pkg", NOT "transitive_pkg"
    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("alya.toml"),
        "[package]\nname = \"app\"\nversion = \"1.0.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\ndirect_pkg = { path = \"../dummy\" }\n",
    )
    .unwrap();

    let res = resolve_package_import("transitive_pkg", &app_dir.join("src").join("main.alya"));
    assert!(res.is_err());
    let err = res.err().unwrap();
    assert_eq!(
        err,
        "Package 'transitive_pkg' is installed as a transitive dependency, but is not declared in 'alya.toml' of this module. Direct dependency isolation requires explicitly declaring 'transitive_pkg' in 'alya.toml' to import it."
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_major_version_segregation_installation() {
    let temp_dir = std::env::temp_dir().join(format!("alya_test_major_seg_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let app_dir = temp_dir.join("app");
    let dep1_dir = temp_dir.join("dep1");
    let dep2_dir = temp_dir.join("dep2");
    let z1_dir = temp_dir.join("z1");
    let z2_dir = temp_dir.join("z2");

    fs::create_dir_all(z1_dir.join("src")).unwrap();
    fs::write(
        z1_dir.join("alya.toml"),
        "[package]\nname = \"z\"\nversion = \"1.0.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(
        z1_dir.join("src").join("lib.alya"),
        "pub function add(a, b) { return a + b }\n",
    )
    .unwrap();

    fs::create_dir_all(z2_dir.join("src")).unwrap();
    fs::write(
        z2_dir.join("alya.toml"),
        "[package]\nname = \"z\"\nversion = \"2.0.0\"\nentry = \"src/lib.alya\"\n",
    )
    .unwrap();
    fs::write(
        z2_dir.join("src").join("lib.alya"),
        "pub function add(a, b) { return a + b + 10 }\n",
    )
    .unwrap();

    fs::create_dir_all(dep1_dir.join("src")).unwrap();
    fs::write(
        dep1_dir.join("alya.toml"),
        format!(
            "[package]\nname = \"dep1\"\nversion = \"1.0.0\"\nentry = \"src/lib.alya\"\n\n[dependencies]\nz = {{ path = \"{}\" }}\n",
            z1_dir.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        dep1_dir.join("src").join("lib.alya"),
        "import z\npub function run1() { return z::add(1, 2) }\n",
    )
    .unwrap();

    fs::create_dir_all(dep2_dir.join("src")).unwrap();
    fs::write(
        dep2_dir.join("alya.toml"),
        format!(
            "[package]\nname = \"dep2\"\nversion = \"1.0.0\"\nentry = \"src/lib.alya\"\n\n[dependencies]\nz = {{ path = \"{}\" }}\n",
            z2_dir.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    fs::write(
        dep2_dir.join("src").join("lib.alya"),
        "import z\npub function run2() { return z::add(1, 2) }\n",
    )
    .unwrap();

    fs::create_dir_all(app_dir.join("src")).unwrap();
    fs::write(
        app_dir.join("alya.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"1.0.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\ndep1 = {{ path = \"{}\" }}\ndep2 = {{ path = \"{}\" }}\n",
            dep1_dir.display().to_string().replace('\\', "/")
            , dep2_dir.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();

    let res = run_install_in(&app_dir);
    assert!(res.is_ok(), "run_install_in failed: {:?}", res.err());

    let packages_dir = app_dir.join(".alya").join("packages");
    assert!(packages_dir.join("z-v1").exists(), "z-v1 must exist");
    assert!(packages_dir.join("z-v2").exists(), "z-v2 must exist");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_mangled_symbol_name_preservation() {
    use crate::codegen::arch::control::mangle_symbol_name;

    assert_eq!(mangle_symbol_name("_Alya_z_v1::foo"), "_Alya_z_v1_foo");
    assert_eq!(mangle_symbol_name("_Alya_z_v2::calc"), "_Alya_z_v2_calc");
    assert_eq!(mangle_symbol_name("std::io::print"), "std__io__print");
}

#[test]
fn test_manifest_preserves_tool_sections() {
    // Unknown sections ([fmt]/[test]/...) and stray comments must survive
    // a parse -> serialize round-trip (regression: `add` used to delete them).
    let toml = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nrand = { git = \"https://github.com/alya-lang/rand\", tag = \"v0.1.0\" }\n\n# Tool configuration\n[fmt]\nexclude = [\"generated/\"]\n\n[test]\n# slow suites excluded\nexclude = [\"slow/\"]\n";
    let manifest = parse_manifest(toml).expect("parse failed");
    let serialized = serialize_manifest(&manifest);
    assert!(
        serialized.contains("[fmt]"),
        "fmt section lost: {}",
        serialized
    );
    assert!(
        serialized.contains("[test]"),
        "test section lost: {}",
        serialized
    );
    assert!(
        serialized.contains("exclude = [\"generated/\"]"),
        "fmt body lost: {}",
        serialized
    );
    assert!(
        serialized.contains("# slow suites excluded"),
        "inner comment lost: {}",
        serialized
    );
    assert!(
        serialized.contains("# Tool configuration"),
        "section comment lost: {}",
        serialized
    );
    let manifest2 = parse_manifest(&serialized).expect("roundtrip parse failed");
    assert_eq!(manifest, manifest2);
}

#[test]
fn test_manifest_rejects_malformed_dependency() {
    // Inline tables without path/git/version must error instead of
    // silently dropping the dependency.
    let toml = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/main.alya\"\n\n[dependencies]\nbroken = { url = \"https://example.com/x\" }\n";
    let err = parse_manifest(toml).expect_err("malformed dep must error");
    assert!(err.contains("broken"), "error names culprit: {}", err);
}

#[test]
fn test_cache_key_is_rev_scoped() {
    // Same tag, different revs => different keys (moved-tag fix).
    let a = compute_cache_key_rev(
        "rand",
        "v0.1.0",
        "https://github.com/alya-lang/rand",
        Some("bb1b445fb4c7029a0a2d0f68e8735713b93ee3d4"),
    );
    let b = compute_cache_key_rev(
        "rand",
        "v0.1.0",
        "https://github.com/alya-lang/rand",
        Some("323e44585e01584f14c8e7d9487eb8a4a3bb9935"),
    );
    assert_ne!(a, b);
    // Legacy form (no rev) is unchanged for offline fallbacks.
    assert_eq!(
        compute_cache_key("crypto", "v0.1.0", "https://github.com/alya-lang/crypto"),
        compute_cache_key_rev(
            "crypto",
            "v0.1.0",
            "https://github.com/alya-lang/crypto",
            None
        )
    );
}

#[test]
fn test_git_source_rev_accepts_short_sha() {
    assert_eq!(
        parse_git_source_rev("git:https://example.com/r?tag=v0.1.0#bb1b445"),
        Some("bb1b445".to_string())
    );
    assert_eq!(
        parse_git_source_rev(
            "git:https://example.com/r?tag=v0.1.0#bb1b445fb4c7029a0a2d0f68e8735713b93ee3d4"
        ),
        Some("bb1b445fb4c7029a0a2d0f68e8735713b93ee3d4".to_string())
    );
    assert_eq!(
        parse_git_source_rev("git:https://example.com/r?tag=v0.1.0#main"),
        None
    );
}
